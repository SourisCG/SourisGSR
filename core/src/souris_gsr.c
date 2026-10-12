/* SourisGSR — implementación inicial de la C ABI sobre el core.
   v1: enumerate + lifecycle con validación; el binding real al recorder
   (gsr_recorder_start/stop) se conecta en la fase de integración Windows.
   Solo gain/volúmenes ya están conectados en el pipeline de audio. */
#include "../../include/souris_gsr.h"
#include "../include/recorder/settings.h"
#include "../include/souris_gain.h"
#include <stdlib.h>
#include <string.h>

struct sgsr_recorder {
    sgsr_config config;
    int running; /* 1 grabando, 2 pausado */
};

const char* sgsr_version(void) { return "0.1.0-souris"; }
int sgsr_api_version(void) { return SGSR_API_VERSION; }

sgsr_error sgsr_init(void) { return SGSR_OK; }
void sgsr_shutdown(void) {}

static int clamp_vol(int v) {
    if(v < 0) return 0;
    if(v > 100) return 100;
    return v;
}

sgsr_error sgsr_enumerate_video_sources(sgsr_source **out, size_t *out_n) {
    if(!out || !out_n) return SGSR_ERR_INVALID_ARG;
    /* v1: fuentes genéricas; el backend (portal/WGC/KMS) las resuelve al start. */
    sgsr_source *list = calloc(2, sizeof(sgsr_source));
    if(!list) return SGSR_ERR_GENERIC;
    list[0].kind = SGSR_SOURCE_MONITOR;
    snprintf(list[0].id, sizeof(list[0].id), "screen");
    snprintf(list[0].title, sizeof(list[0].title), "Monitor principal");
    list[1].kind = SGSR_SOURCE_PORTAL;
    snprintf(list[1].id, sizeof(list[1].id), "portal");
    snprintf(list[1].title, sizeof(list[1].title), "Picker (portal/WGC)");
    *out = list;
    *out_n = 2;
    return SGSR_OK;
}

sgsr_error sgsr_enumerate_audio_devices(sgsr_audio_device **out, size_t *out_n) {
    if(!out || !out_n) return SGSR_ERR_INVALID_ARG;
    sgsr_audio_device *list = calloc(2, sizeof(sgsr_audio_device));
    if(!list) return SGSR_ERR_GENERIC;
    snprintf(list[0].name, sizeof(list[0].name), "default_output");
    snprintf(list[0].description, sizeof(list[0].description), "Default output (sistema)");
    list[0].is_input = 0;
    snprintf(list[1].name, sizeof(list[1].name), "default_input");
    snprintf(list[1].description, sizeof(list[1].description), "Default input (micrófono)");
    list[1].is_input = 1;
    *out = list;
    *out_n = 2;
    return SGSR_OK;
}

void sgsr_free_video_sources(sgsr_source *p) { free(p); }
void sgsr_free_audio_devices(sgsr_audio_device *p) { free(p); }

sgsr_error sgsr_start(const sgsr_config *config, sgsr_recorder **out) {
    if(!config || !out) return SGSR_ERR_INVALID_ARG;
    if(config->fps < 1 || config->fps > 1000) return SGSR_ERR_INVALID_ARG;
    if(config->num_audio_inputs < 0 || config->num_audio_inputs > 8) return SGSR_ERR_INVALID_ARG;
    if(config->mic_volume_percent < 0 || config->mic_volume_percent > 100) return SGSR_ERR_INVALID_ARG;
    if(config->system_volume_percent < 0 || config->system_volume_percent > 100) return SGSR_ERR_INVALID_ARG;
    if(config->app_volume_percent < 0 || config->app_volume_percent > 100) return SGSR_ERR_INVALID_ARG;
    sgsr_recorder *r = calloc(1, sizeof(*r));
    if(!r) return SGSR_ERR_GENERIC;
    r->config = *config;
    r->config.mic_volume_percent = clamp_vol(config->mic_volume_percent);
    r->config.system_volume_percent = clamp_vol(config->system_volume_percent);
    r->config.app_volume_percent = clamp_vol(config->app_volume_percent);
    r->running = 1;
    /* TODO(integración): mapear sgsr_config -> gsr_recorder_settings
       (incl. gsr_recorder_settings_init_souris_volumes + -a + vol=) y
       arrancar el pipeline real. v1 valida y guarda estado. */
    if(r->config.on_event) {
        sgsr_event ev;
        memset(&ev, 0, sizeof(ev));
        ev.kind = SGSR_EVENT_STARTED;
        r->config.on_event(&ev, r->config.event_userdata);
    }
    *out = r;
    return SGSR_OK;
}

sgsr_error sgsr_stop(sgsr_recorder *r) {
    if(!r) return SGSR_ERR_INVALID_ARG;
    if(r->running == 0) return SGSR_ERR_NOT_RUNNING;
    r->running = 0;
    if(r->config.on_event) {
        sgsr_event ev;
        memset(&ev, 0, sizeof(ev));
        ev.kind = SGSR_EVENT_STOPPED;
        r->config.on_event(&ev, r->config.event_userdata);
    }
    free(r);
    return SGSR_OK;
}

sgsr_error sgsr_pause(sgsr_recorder *r) {
    if(!r) return SGSR_ERR_INVALID_ARG;
    if(r->running != 1) return SGSR_ERR_NOT_RUNNING;
    r->running = 2;
    return SGSR_OK;
}

sgsr_error sgsr_resume(sgsr_recorder *r) {
    if(!r) return SGSR_ERR_INVALID_ARG;
    if(r->running != 2) return SGSR_ERR_NOT_RUNNING;
    r->running = 1;
    return SGSR_OK;
}

sgsr_error sgsr_save_replay(sgsr_recorder *r, const char *path) {
    if(!r || !path || !path[0]) return SGSR_ERR_INVALID_ARG;
    if(r->config.replay_buffer_secs <= 0) return SGSR_ERR_UNSUPPORTED;
    return SGSR_OK;
}

sgsr_error sgsr_screenshot(sgsr_recorder *r, const char *path) {
    if(!r || !path || !path[0]) return SGSR_ERR_INVALID_ARG;
    return SGSR_OK;
}

sgsr_error sgsr_set_volumes(sgsr_recorder *r, int mic, int system, int app) {
    if(!r) return SGSR_ERR_INVALID_ARG;
    if(mic < 0 || mic > 100 || system < 0 || system > 100 || app < 0 || app > 100)
        return SGSR_ERR_INVALID_ARG;
    /* Solo grabación: actualiza el gain efectivo; nunca el mixer del SO. */
    r->config.mic_volume_percent = mic;
    r->config.system_volume_percent = system;
    r->config.app_volume_percent = app;
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
