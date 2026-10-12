/* SourisGSR — C ABI implementation driving the real core pipeline.
   sgsr_start() mirrors cli/main.c run()/record(): it parses the -w/-a
   strings with the same parsers, brings up windowing+EGL, creates a
   gsr_recorder and runs it on a worker thread. Control (stop/pause/
   replay/screenshot/volumes) delegates to the thread-safe
   gsr_recorder_* API. Recording-only volumes flow through settings and
   gsr_recorder_set_volumes(); the system mixer is never touched. */
#include "../../include/souris_gsr.h"
#include "../include/recorder/settings.h"
#include "../include/recorder/error.h"
#include "../include/recorder/recorder.h"
#include "../include/recorder/capture_source.h"
#include "../include/recorder/capture_setup.h"
#include "../include/recorder/audio_input.h"
#include "../include/recorder/windowing.h"
#include "../include/recorder/screenshot.h"
#include "../include/recorder/replay_save.h"
#include "../include/sound.h"
#include "../include/souris_gain.h"
#include "../include/utils.h"
#include "../include/log.h"
#include "../include/defs.h"
#ifdef GSR_APP_AUDIO
#include "../include/pipewire_audio.h"
#endif

#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <pthread.h>
#include <stdatomic.h>
#include <limits.h>
#include <libgen.h>
#include <sys/stat.h>
#include <unistd.h>

#define SGSR_STATE_STOPPED 0
#define SGSR_STATE_RECORDING 1
#define SGSR_STATE_PAUSED 2

struct sgsr_recorder {
    sgsr_event_cb on_event;
    void *event_userdata;

    /* Owned copies; settings points into these. */
    char *filename;
    char *container;
    char *replay_dir;
    char *capture_str;
    char **audio_strs;
    int num_audio_strs;
    char pending_save_path[PATH_MAX];

    gsr_recorder_settings settings;
    gsr_capture_sources capture_sources;
    gsr_audio_input_tracks audio_tracks;
    gsr_app_audio_names app_audio_names;
    gsr_windowing windowing;
    gsr_capture_deps capture_deps;
#ifdef GSR_APP_AUDIO
    gsr_pipewire_audio pipewire_audio;
    bool pipewire_init;
#endif
    bool windowing_init;
    bool deps_init;
    bool sources_init;
    bool tracks_init;
    bool app_names_init;

    gsr_recorder *rec;
    pthread_t thread;
    bool thread_started;
    atomic_int state;
    pthread_mutex_t screenshot_mutex;

    int mic_volume_percent;
    int system_volume_percent;
    int app_volume_percent;
};

const char* sgsr_version(void) { return "0.2.0-souris"; }
int sgsr_api_version(void) { return SGSR_API_VERSION; }

sgsr_error sgsr_init(void) { return SGSR_OK; }
void sgsr_shutdown(void) {}

static int clamp_vol(int v) {
    if(v < 0) return 0;
    if(v > 100) return 100;
    return v;
}

static sgsr_error map_gsr_error(int e) {
    switch(e) {
        case GSR_ERROR_OK: return SGSR_OK;
        case GSR_ERROR_UNSUPPORTED: return SGSR_ERR_UNSUPPORTED;
        case GSR_ERROR_AUDIO_DEVICE_NOT_FOUND: return SGSR_ERR_NO_DEVICE;
        default: return SGSR_ERR_GENERIC;
    }
}

static void emit_event(sgsr_recorder *r, sgsr_event_kind kind, int code, double data0, const char *message) {
    if(!r || !r->on_event)
        return;
    sgsr_event ev;
    memset(&ev, 0, sizeof(ev));
    ev.kind = kind;
    ev.code = code;
    ev.data0 = data0;
    if(message)
        snprintf(ev.message, sizeof(ev.message), "%s", message);
    r->on_event(&ev, r->event_userdata);
}

static void on_recording_started(const char *filepath, void *userdata) {
    sgsr_recorder *r = userdata;
    emit_event(r, SGSR_EVENT_STARTED, 0, 0.0, filepath);
}

static void on_recording_stopped(const char *filepath, void *userdata) {
    sgsr_recorder *r = userdata;
    atomic_store(&r->state, SGSR_STATE_STOPPED);
    emit_event(r, SGSR_EVENT_STOPPED, 0, 0.0, filepath);
}

static void on_replay_saved(const char *filepath, void *userdata) {
    sgsr_recorder *r = userdata;
    char final_path[PATH_MAX];
    final_path[0] = '\0';
    if(filepath)
        snprintf(final_path, sizeof(final_path), "%s", filepath);
    /* Honor the desired path when given: the core saves timestamped files
       into the replay directory, so rename into place (same filesystem).
       On failure (e.g. EXDEV) keep the original and report that. */
    if(r->pending_save_path[0] && final_path[0] && strcmp(r->pending_save_path, final_path) != 0) {
        if(rename(final_path, r->pending_save_path) == 0)
            snprintf(final_path, sizeof(final_path), "%s", r->pending_save_path);
    }
    r->pending_save_path[0] = '\0';
    if(final_path[0])
        emit_event(r, SGSR_EVENT_REPLAY_SAVED, 0, 0.0, final_path);
    else
        emit_event(r, SGSR_EVENT_ERROR, (int)SGSR_ERR_GENERIC, 0.0, "replay save failed");
}

static void* recorder_thread(void *userdata) {
    sgsr_recorder *r = userdata;
    gsr_recorder_run(r->rec);
    return NULL;
}

static bool sgsr_is_livestream_path(const char *str) {
    if(!str)
        return false;
    const size_t len = strlen(str);
    if((len >= 7 && memcmp(str, "http://", 7) == 0) || (len >= 8 && memcmp(str, "https://", 8) == 0))
        return true;
    if((len >= 7 && memcmp(str, "rtmp://", 7) == 0) || (len >= 8 && memcmp(str, "rtmps://", 8) == 0))
        return true;
    if(len >= 7 && memcmp(str, "rtsp://", 7) == 0)
        return true;
    if(len >= 6 && memcmp(str, "srt://", 6) == 0)
        return true;
    if(len >= 6 && memcmp(str, "tcp://", 6) == 0)
        return true;
    if(len >= 6 && memcmp(str, "udp://", 6) == 0)
        return true;
    return false;
}

static int sgsr_map_video_codec(const char *name) {
    if(!name || strcmp(name, "auto") == 0)
        return GSR_VIDEO_CODEC_AUTO;
    if(strcmp(name, "h264") == 0)
        return GSR_VIDEO_CODEC_H264;
    if(strcmp(name, "hevc") == 0 || strcmp(name, "h265") == 0)
        return GSR_VIDEO_CODEC_HEVC;
    if(strcmp(name, "av1") == 0)
        return GSR_VIDEO_CODEC_AV1;
    return GSR_VIDEO_CODEC_AUTO;
}

static void sgsr_teardown(sgsr_recorder *r) {
    if(!r)
        return;
    if(r->thread_started) {
        pthread_join(r->thread, NULL);
        r->thread_started = false;
    }
    if(r->rec) {
        gsr_recorder_destroy(r->rec, true);
        r->rec = NULL;
    }
    if(r->deps_init) {
        gsr_capture_deps_deinit(&r->capture_deps);
        r->deps_init = false;
    }
    if(r->windowing_init) {
        gsr_windowing_deinit(&r->windowing);
        r->windowing_init = false;
    }
#ifdef GSR_APP_AUDIO
    if(r->pipewire_init) {
        gsr_pipewire_audio_deinit(&r->pipewire_audio);
        r->pipewire_init = false;
    }
#endif
    if(r->app_names_init) {
        gsr_app_audio_names_deinit(&r->app_audio_names);
        r->app_names_init = false;
    }
    if(r->tracks_init) {
        gsr_audio_input_tracks_deinit(&r->audio_tracks);
        r->tracks_init = false;
    }
    if(r->sources_init) {
        gsr_capture_sources_deinit(&r->capture_sources);
        r->sources_init = false;
    }
}

sgsr_error sgsr_enumerate_video_sources(sgsr_source **out, size_t *out_n) {
    if(!out || !out_n) return SGSR_ERR_INVALID_ARG;
    sgsr_source *list = calloc(2, sizeof(sgsr_source));
    if(!list) return SGSR_ERR_GENERIC;
    list[0].kind = SGSR_SOURCE_MONITOR;
    snprintf(list[0].id, sizeof(list[0].id), "screen");
    snprintf(list[0].title, sizeof(list[0].title), "Primary monitor");
    list[1].kind = SGSR_SOURCE_PORTAL;
    snprintf(list[1].id, sizeof(list[1].id), "portal");
    snprintf(list[1].title, sizeof(list[1].title), "Picker (portal/WGC)");
    *out = list;
    *out_n = 2;
    return SGSR_OK;
}

sgsr_error sgsr_enumerate_audio_devices(sgsr_audio_device **out, size_t *out_n) {
    if(!out || !out_n) return SGSR_ERR_INVALID_ARG;
    gsr_audio_devices devices;
    memset(&devices, 0, sizeof(devices));
    get_pulseaudio_inputs(&devices);
    const size_t n = 2 + devices.num_items;
    sgsr_audio_device *list = calloc(n ? n : 1, sizeof(sgsr_audio_device));
    if(!list) {
        gsr_audio_devices_deinit(&devices);
        return SGSR_ERR_GENERIC;
    }
    snprintf(list[0].name, sizeof(list[0].name), "default_output");
    snprintf(list[0].description, sizeof(list[0].description), "Default output (system)");
    list[0].is_input = 0;
    snprintf(list[1].name, sizeof(list[1].name), "default_input");
    snprintf(list[1].description, sizeof(list[1].description), "Default input (microphone)");
    list[1].is_input = 1;
    for(size_t i = 0; i < devices.num_items; ++i) {
        snprintf(list[2 + i].name, sizeof(list[2 + i].name), "%s", devices.items[i].name);
        snprintf(list[2 + i].description, sizeof(list[2 + i].description), "%s", devices.items[i].description);
        list[2 + i].is_input = souris_device_name_is_mic(devices.items[i].name) ? 1 : 0;
    }
    gsr_audio_devices_deinit(&devices);
    *out = list;
    *out_n = n;
    return SGSR_OK;
}

void sgsr_free_video_sources(sgsr_source *p) { free(p); }
void sgsr_free_audio_devices(sgsr_audio_device *p) { free(p); }

#ifdef GSR_APP_AUDIO
static bool sgsr_app_audio_name_callback(const char *app_name, void *userdata) {
    gsr_app_audio_names *names = userdata;
    return gsr_app_audio_names_add(names, app_name);
}
#endif

sgsr_error sgsr_start(const sgsr_config *config, sgsr_recorder **out) {
    if(!config || !out) return SGSR_ERR_INVALID_ARG;
    if(config->fps < 1 || config->fps > 1000) return SGSR_ERR_INVALID_ARG;
    if(config->num_audio_inputs < 0 || config->num_audio_inputs > 8) return SGSR_ERR_INVALID_ARG;
    if(config->mic_volume_percent < 0 || config->mic_volume_percent > 100) return SGSR_ERR_INVALID_ARG;
    if(config->system_volume_percent < 0 || config->system_volume_percent > 100) return SGSR_ERR_INVALID_ARG;
    if(config->app_volume_percent < 0 || config->app_volume_percent > 100) return SGSR_ERR_INVALID_ARG;
    if(config->video_source.kind == SGSR_SOURCE_REGION) return SGSR_ERR_UNSUPPORTED;
    if(!config->output_path || !config->output_path[0]) return SGSR_ERR_INVALID_ARG;

    sgsr_recorder *r = calloc(1, sizeof(*r));
    if(!r) return SGSR_ERR_GENERIC;
    atomic_init(&r->state, SGSR_STATE_STOPPED);
    if(pthread_mutex_init(&r->screenshot_mutex, NULL) != 0) {
        free(r);
        return SGSR_ERR_GENERIC;
    }
    r->on_event = config->on_event;
    r->event_userdata = config->event_userdata;
    r->mic_volume_percent = clamp_vol(config->mic_volume_percent);
    r->system_volume_percent = clamp_vol(config->system_volume_percent);
    r->app_volume_percent = clamp_vol(config->app_volume_percent);
    gsr_capture_deps_init(&r->capture_deps);
    r->deps_init = true;

    sgsr_error result = SGSR_ERR_GENERIC;

    /* Owned string copies (settings borrows these). */
    r->container = strdup(config->container && config->container[0] ? config->container : "mp4");
    r->filename = strdup(config->output_path);
    if(!r->container || !r->filename) goto fail;
    if(strcmp(r->container, "mkv") == 0) {
        free(r->container);
        r->container = strdup("matroska");
        if(!r->container) goto fail;
    }
    switch(config->video_source.kind) {
        case SGSR_SOURCE_MONITOR:
            r->capture_str = strdup(config->video_source.id[0] ? config->video_source.id : "screen");
            break;
        case SGSR_SOURCE_WINDOW:
            if(!config->video_source.id[0]) { result = SGSR_ERR_INVALID_ARG; goto fail; }
            r->capture_str = strdup(config->video_source.id);
            break;
        case SGSR_SOURCE_PORTAL:
            r->capture_str = strdup("portal");
            break;
        default:
            result = SGSR_ERR_UNSUPPORTED;
            goto fail;
    }
    if(!r->capture_str) goto fail;

    if(config->num_audio_inputs > 0) {
        r->audio_strs = calloc((size_t)config->num_audio_inputs, sizeof(char*));
        if(!r->audio_strs) goto fail;
        for(int i = 0; i < config->num_audio_inputs; ++i) {
            if(!config->audio_inputs[i]) { result = SGSR_ERR_INVALID_ARG; goto fail; }
            r->audio_strs[i] = strdup(config->audio_inputs[i]);
            if(!r->audio_strs[i]) goto fail;
        }
        r->num_audio_strs = config->num_audio_inputs;
    }

    /* Settings (mirrors the CLI defaults in args_parser). */
    gsr_recorder_settings *s = &r->settings;
    memset(s, 0, sizeof(*s));
    gsr_recorder_settings_init_souris_volumes(s);
    s->video_encoder = GSR_VIDEO_ENCODER_HW_GPU;
    s->pixel_format = GSR_PIXEL_FORMAT_YUV420;
    s->framerate_mode = GSR_FRAMERATE_MODE_VARIABLE;
    s->color_range = GSR_COLOR_RANGE_LIMITED;
    s->tune = GSR_TUNE_PERFORMANCE;
    s->video_codec = sgsr_map_video_codec(config->video_codec);
    s->audio_codec = GSR_AUDIO_CODEC_OPUS;
    s->bitrate_mode = GSR_BITRATE_MODE_AUTO;
    s->video_quality = GSR_VIDEO_QUALITY_VERY_HIGH;
    s->replay_storage = GSR_REPLAY_STORAGE_RAM;
    s->capture_source = r->capture_str;
    s->container_format = r->container;
    s->filename = r->filename;
    s->record_cursor = true;
    s->keyint = 2.0;
    s->fps = config->fps;
    s->audio_bitrate = 0;
    s->mic_volume_percent = r->mic_volume_percent;
    s->system_volume_percent = r->system_volume_percent;
    s->app_volume_percent = r->app_volume_percent;
    if(config->video_source.width > 0 && config->video_source.height > 0)
        s->output_resolution = (vec2i){config->video_source.width, config->video_source.height};

    const bool replay = config->replay_buffer_secs > 0;
    if(replay) {
        struct stat st;
        if(stat(r->filename, &st) == 0 && !S_ISDIR(st.st_mode)) {
            result = SGSR_ERR_INVALID_ARG;
            goto fail;
        }
        /* -o is a directory in replay mode; make sure it exists. */
        {
            char dirbuf[PATH_MAX];
            snprintf(dirbuf, sizeof(dirbuf), "%s", r->filename);
            if(create_directory_recursive(dirbuf) != 0) {
                result = SGSR_ERR_GENERIC;
                goto fail;
            }
        }
        r->replay_dir = strdup(r->filename);
        if(!r->replay_dir) goto fail;
        s->is_replaying = true;
        s->replay_buffer_size_secs = (int64_t)config->replay_buffer_secs + (int64_t)(s->keyint + 0.5);
        s->replay_recording_directory = r->replay_dir;
    } else {
        s->is_livestream = sgsr_is_livestream_path(r->filename);
        if(!s->is_livestream) {
            char directory_buf[PATH_MAX];
            snprintf(directory_buf, sizeof(directory_buf), "%s", r->filename);
            char *directory = dirname(directory_buf);
            if(strcmp(directory, ".") != 0 && strcmp(directory, "/") != 0) {
                if(create_directory_recursive(directory) != 0) {
                    result = SGSR_ERR_GENERIC;
                    goto fail;
                }
            }
        }
        s->low_latency_recording = s->is_livestream;
    }

    /* Capture sources (-w with the same parser as the CLI). */
    {
        const int pr = gsr_capture_sources_parse(&r->capture_sources, r->capture_str, (vec2i){0, 0}, (vec2i){0, 0});
        if(pr != GSR_ERROR_OK) {
            result = map_gsr_error(pr);
            goto fail;
        }
        r->sources_init = true;
    }
    if(r->capture_sources.num_items == 0) {
        result = SGSR_ERR_INVALID_ARG;
        goto fail;
    }

    /* Audio tracks (-a with the same parser, vol= included). */
    if(r->num_audio_strs > 0) {
        gsr_audio_devices devices;
        memset(&devices, 0, sizeof(devices));
        get_pulseaudio_inputs(&devices);
        const int pr = gsr_audio_input_tracks_parse(&r->audio_tracks, (const char**)r->audio_strs, r->num_audio_strs, &devices);
        gsr_audio_devices_deinit(&devices);
        if(pr != GSR_ERROR_OK) {
            result = map_gsr_error(pr);
            goto fail;
        }
        r->tracks_init = true;

        const bool uses_app_audio = gsr_audio_input_tracks_has_app_audio(&r->audio_tracks);
#ifdef GSR_APP_AUDIO
        if(uses_app_audio) {
            if(!pulseaudio_server_is_pipewire()) {
                result = SGSR_ERR_UNSUPPORTED;
                goto fail;
            }
            if(!gsr_pipewire_audio_init(&r->pipewire_audio)) {
                result = SGSR_ERR_GENERIC;
                goto fail;
            }
            r->pipewire_init = true;
            gsr_pipewire_audio_for_each_app(&r->pipewire_audio, sgsr_app_audio_name_callback, &r->app_audio_names);
            r->app_names_init = true;
        }
#else
        if(uses_app_audio) {
            result = SGSR_ERR_UNSUPPORTED;
            goto fail;
        }
#endif
        const int vr = gsr_audio_input_tracks_validate_app_audio(&r->audio_tracks, &r->app_audio_names);
        if(vr != GSR_ERROR_OK) {
            result = map_gsr_error(vr);
            goto fail;
        }
    }

    /* Windowing + EGL (mirrors run()). */
    {
        char monitor_to_capture[GSR_CAPTURE_SOURCE_NAME_MAX_SIZE];
        monitor_to_capture[0] = '\0';
        gsr_windowing_params windowing_params;
        memset(&windowing_params, 0, sizeof(windowing_params));
        windowing_params.monitor_capture = gsr_capture_sources_has_monitor_or_region(&r->capture_sources);
        windowing_params.listen_to_x11_events = true;
        windowing_params.monitor_to_capture = monitor_to_capture;
        if(gsr_windowing_init(&r->windowing, &windowing_params) != GSR_ERROR_OK) {
            result = SGSR_ERR_GENERIC;
            goto fail;
        }
        r->windowing_init = true;

        if(gsr_windowing_is_wayland(&r->windowing))
            gsr_capture_sources_get_monitor_to_capture(&r->capture_sources, r->windowing.window, monitor_to_capture, sizeof(monitor_to_capture));

        if(gsr_windowing_load_egl(&r->windowing, &windowing_params) != GSR_ERROR_OK) {
            result = SGSR_ERR_GENERIC;
            goto fail;
        }
        if(!r->windowing.card_path_found) {
            result = SGSR_ERR_NO_DEVICE;
            goto fail;
        }
    }
    gsr_capture_deps_init_cursor(&r->capture_deps, &r->windowing.egl, true);

    /* Recorder + worker thread. */
    {
        gsr_recorder_params params;
        memset(&params, 0, sizeof(params));
        params.settings = s;
        params.windowing = &r->windowing;
        params.capture_deps = &r->capture_deps;
        params.capture_sources = &r->capture_sources;
        params.audio_input_tracks = &r->audio_tracks;
#ifdef GSR_APP_AUDIO
        params.pipewire_audio = r->pipewire_init ? &r->pipewire_audio : NULL;
#endif
        gsr_recorder_callbacks callbacks;
        memset(&callbacks, 0, sizeof(callbacks));
        callbacks.recording_started = on_recording_started;
        callbacks.recording_stopped = on_recording_stopped;
        callbacks.replay_saved = on_replay_saved;
        callbacks.userdata = r;

        int error = GSR_ERROR_OK;
        r->rec = gsr_recorder_create(&params, &callbacks, &error);
        if(!r->rec) {
            result = map_gsr_error(error);
            goto fail;
        }
        if(pthread_create(&r->thread, NULL, recorder_thread, r) != 0) {
            gsr_recorder_destroy(r->rec, true);
            r->rec = NULL;
            result = SGSR_ERR_GENERIC;
            goto fail;
        }
        r->thread_started = true;
    }

    atomic_store(&r->state, SGSR_STATE_RECORDING);
    *out = r;
    return SGSR_OK;

fail:
    sgsr_teardown(r);
    pthread_mutex_destroy(&r->screenshot_mutex);
    free(r->container);
    free(r->filename);
    free(r->replay_dir);
    free(r->capture_str);
    if(r->audio_strs) {
        for(int i = 0; i < r->num_audio_strs; ++i)
            free(r->audio_strs[i]);
        free(r->audio_strs);
    }
    free(r);
    return result;
}

sgsr_error sgsr_stop(sgsr_recorder *r) {
    if(!r) return SGSR_ERR_INVALID_ARG;
    if(atomic_load(&r->state) == SGSR_STATE_STOPPED && !r->thread_started)
        return SGSR_ERR_NOT_RUNNING;
    if(r->rec)
        gsr_recorder_stop(r->rec);
    atomic_store(&r->state, SGSR_STATE_STOPPED);
    sgsr_teardown(r);
    pthread_mutex_destroy(&r->screenshot_mutex);
    free(r->container);
    free(r->filename);
    free(r->replay_dir);
    free(r->capture_str);
    if(r->audio_strs) {
        for(int i = 0; i < r->num_audio_strs; ++i)
            free(r->audio_strs[i]);
        free(r->audio_strs);
    }
    free(r);
    return SGSR_OK;
}

sgsr_error sgsr_pause(sgsr_recorder *r) {
    if(!r) return SGSR_ERR_INVALID_ARG;
    if(atomic_load(&r->state) != SGSR_STATE_RECORDING) return SGSR_ERR_NOT_RUNNING;
    if(!r->rec) return SGSR_ERR_NOT_RUNNING;
    gsr_recorder_set_paused(r->rec, true);
    atomic_store(&r->state, SGSR_STATE_PAUSED);
    emit_event(r, SGSR_EVENT_PAUSED, 0, 0.0, NULL);
    return SGSR_OK;
}

sgsr_error sgsr_resume(sgsr_recorder *r) {
    if(!r) return SGSR_ERR_INVALID_ARG;
    if(atomic_load(&r->state) != SGSR_STATE_PAUSED) return SGSR_ERR_NOT_RUNNING;
    if(!r->rec) return SGSR_ERR_NOT_RUNNING;
    gsr_recorder_set_paused(r->rec, false);
    atomic_store(&r->state, SGSR_STATE_RECORDING);
    emit_event(r, SGSR_EVENT_RESUMED, 0, 0.0, NULL);
    return SGSR_OK;
}

sgsr_error sgsr_save_replay(sgsr_recorder *r, const char *path) {
    if(!r || !path || !path[0]) return SGSR_ERR_INVALID_ARG;
    if(atomic_load(&r->state) == SGSR_STATE_STOPPED || !r->rec) return SGSR_ERR_NOT_RUNNING;
    if(!r->settings.is_replaying) return SGSR_ERR_UNSUPPORTED;
    if(strlen(path) >= sizeof(r->pending_save_path)) return SGSR_ERR_INVALID_ARG;
    snprintf(r->pending_save_path, sizeof(r->pending_save_path), "%s", path);
    gsr_recorder_save_replay(r->rec, GSR_SAVE_REPLAY_SECONDS_FULL, GSR_RESTART_REPLAY_USE_OPTION);
    return SGSR_OK;
}

sgsr_error sgsr_screenshot(sgsr_recorder *r, const char *path) {
    if(!r || !path || !path[0]) return SGSR_ERR_INVALID_ARG;
    if(atomic_load(&r->state) == SGSR_STATE_STOPPED || !r->rec) return SGSR_ERR_NOT_RUNNING;
    pthread_mutex_lock(&r->screenshot_mutex);
    gsr_recorder_settings shot_settings = r->settings;
    shot_settings.filename = path;
    gsr_screenshot_params params;
    memset(&params, 0, sizeof(params));
    params.settings = &shot_settings;
    params.egl = &r->windowing.egl;
    params.window = r->windowing.window;
    params.capture_deps = &r->capture_deps;
    params.capture_sources = &r->capture_sources;
    params.image_format = GSR_IMAGE_FORMAT_JPEG;
    get_image_format_from_filename(path, &params.image_format);
    int ret = gsr_screenshot_take(&params);
    pthread_mutex_unlock(&r->screenshot_mutex);
    return ret == GSR_ERROR_OK ? SGSR_OK : map_gsr_error(ret);
}

sgsr_error sgsr_set_volumes(sgsr_recorder *r, int mic, int system, int app) {
    if(!r) return SGSR_ERR_INVALID_ARG;
    if(mic < 0 || mic > 100 || system < 0 || system > 100 || app < 0 || app > 100)
        return SGSR_ERR_INVALID_ARG;
    /* Recording only: effective gain update; never the OS mixer. */
    r->mic_volume_percent = mic;
    r->system_volume_percent = system;
    r->app_volume_percent = app;
    if(r->rec)
        gsr_recorder_set_volumes(r->rec, mic, system, app);
    return SGSR_OK;
}

const char* sgsr_strerror(sgsr_error e) {
    switch(e) {
        case SGSR_OK: return "ok";
        case SGSR_ERR_GENERIC: return "generic error";
        case SGSR_ERR_INVALID_ARG: return "invalid argument";
        case SGSR_ERR_UNSUPPORTED: return "unsupported";
        case SGSR_ERR_NO_DEVICE: return "no device";
        case SGSR_ERR_ALREADY_RUNNING: return "already running";
        case SGSR_ERR_NOT_RUNNING: return "not running";
        default: return "unknown";
    }
}
