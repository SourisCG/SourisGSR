#ifndef SOURIS_GSR_H
#define SOURIS_GSR_H

/* SourisGSR — stable C ABI for the Rust SDK and embedders.
   Exposes ALL program features (recording, replay, streaming,
   screenshots, overlays, volumes) without leaking C++ or FFmpeg.
   Versioning: sgsr_version(). Never break compat: only add
   sgsr_*_v2 functions. License: GPL-3.0-only (same as core). */

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define SGSR_API_VERSION 1

typedef struct sgsr_recorder sgsr_recorder;

typedef enum {
    SGSR_OK = 0,
    SGSR_ERR_GENERIC = -1,
    SGSR_ERR_INVALID_ARG = -2,
    SGSR_ERR_UNSUPPORTED = -3,
    SGSR_ERR_NO_DEVICE = -4,
    SGSR_ERR_ALREADY_RUNNING = -5,
    SGSR_ERR_NOT_RUNNING = -6,
} sgsr_error;

typedef enum {
    SGSR_SOURCE_MONITOR = 0,
    SGSR_SOURCE_WINDOW = 1,   /* individual window: portal (Wayland) / HWND (WGC on Win) */
    SGSR_SOURCE_PORTAL = 2,   /* compositor picker (Wayland) */
    SGSR_SOURCE_REGION = 3,
} sgsr_source_kind;

typedef struct {
    sgsr_source_kind kind;
    char id[256];    /* monitor index / window id (X11) / HWND (Win) / portal token */
    char title[256]; /* window/monitor title for pickers */
    int width;
    int height;
} sgsr_source;

typedef struct {
    char name[256];
    char description[256];
    int is_input; /* 1 mic/input, 0 output/system */
} sgsr_audio_device;

typedef enum {
    SGSR_EVENT_STARTED = 0,
    SGSR_EVENT_STOPPED,
    SGSR_EVENT_PAUSED,
    SGSR_EVENT_RESUMED,
    SGSR_EVENT_REPLAY_SAVED,
    SGSR_EVENT_FRAME_DROPPED,
    SGSR_EVENT_ERROR,
    SGSR_EVENT_LEVEL_METER, /* VU for UI: value 0.0-1.0 in data0 */
} sgsr_event_kind;

typedef struct {
    sgsr_event_kind kind;
    int code;
    double data0;
    char message[512];
} sgsr_event;

typedef void (*sgsr_event_cb)(const sgsr_event *ev, void *userdata);
/* NOTE: the event callback fires on the internal worker thread; it must be
   thread-safe. Message carries file paths for replay/screenshot events. */

typedef struct {
    sgsr_source video_source;
    int fps;
    const char *video_codec; /* "auto","h264","hevc","av1" */
    const char *container;   /* "mp4","mkv","mov" */
    const char *output_path; /* or rtmp/srt URL for streaming */
    const char *audio_inputs[8]; /* same ids as CLI -a */
    int num_audio_inputs;
    /* Recording-only volumes 0-100, OBS style. Never touch the system. */
    int mic_volume_percent;
    int system_volume_percent;
    int app_volume_percent;
    int replay_buffer_secs;  /* 0 = normal recording */
    int record_cursor;       /* 1 yes, 0 no */
    sgsr_event_cb on_event;
    void *event_userdata;
} sgsr_config;

const char* sgsr_version(void);
int sgsr_api_version(void);

/* Embedded engine lifecycle (no child processes spawned). */
sgsr_error sgsr_init(void);
void sgsr_shutdown(void);

/* Enumeration (caller frees with sgsr_free_*). */
sgsr_error sgsr_enumerate_video_sources(sgsr_source **out, size_t *out_n);
sgsr_error sgsr_enumerate_audio_devices(sgsr_audio_device **out, size_t *out_n);
void sgsr_free_video_sources(sgsr_source *p);
void sgsr_free_audio_devices(sgsr_audio_device *p);

/* Full control. config must stay alive during start (it is copied).
   start() brings up windowing+EGL and runs the real engine on a worker
   thread; stop() joins it. All control calls are thread-safe. */
sgsr_error sgsr_start(const sgsr_config *config, sgsr_recorder **out);
sgsr_error sgsr_stop(sgsr_recorder *r);
sgsr_error sgsr_pause(sgsr_recorder *r);
sgsr_error sgsr_resume(sgsr_recorder *r);
/* Save the replay buffer. The engine writes a timestamped file into the
   replay directory; when path differs it is renamed into place and the
   final path is reported via SGSR_EVENT_REPLAY_SAVED. Requires replay mode,
   else SGSR_ERR_UNSUPPORTED. */
sgsr_error sgsr_save_replay(sgsr_recorder *r, const char *path);
/* Single screenshot into path (format from extension, JPEG fallback).
   Requires a running recorder, else SGSR_ERR_NOT_RUNNING. */
sgsr_error sgsr_screenshot(sgsr_recorder *r, const char *path);

/* Hot 0-100 volumes (recording only). */
sgsr_error sgsr_set_volumes(sgsr_recorder *r, int mic, int system, int app);

const char* sgsr_strerror(sgsr_error e);

#ifdef __cplusplus
}
#endif

#endif /* SOURIS_GSR_H */
