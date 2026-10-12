#ifndef SOURIS_GSR_H
#define SOURIS_GSR_H

/* SourisGSR — C ABI estable para el SDK Rust y apps embebidas.
   Expone TODAS las funciones del programa (grabación, replay, streaming,
   screenshots, overlays, volúmenes) sin exponer C++ ni FFmpeg.
   Versionado: sgsr_version(). Nunca romper compat: solo añadir funciones
   sgsr_*_v2. Licencia: GPL-3.0-only (igual que el core). */

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
    SGSR_SOURCE_WINDOW = 1,   /* ventana individual: portal (Wayland) / HWND (WGC Win) */
    SGSR_SOURCE_PORTAL = 2,   /* picker del compositor (Wayland) */
    SGSR_SOURCE_REGION = 3,
} sgsr_source_kind;

typedef struct {
    sgsr_source_kind kind;
    char id[256];    /* monitor index / window id (X11) / HWND (Win) / token portal */
    char title[256]; /* título ventana/monitor para pickers */
    int width;
    int height;
} sgsr_source;

typedef struct {
    char name[256];
    char description[256];
    int is_input; /* 1 mic/entrada, 0 salida/sistema */
} sgsr_audio_device;

typedef enum {
    SGSR_EVENT_STARTED = 0,
    SGSR_EVENT_STOPPED,
    SGSR_EVENT_PAUSED,
    SGSR_EVENT_RESUMED,
    SGSR_EVENT_REPLAY_SAVED,
    SGSR_EVENT_FRAME_DROPPED,
    SGSR_EVENT_ERROR,
    SGSR_EVENT_LEVEL_METER, /* VU para UI: value 0.0-1.0 en data0 */
} sgsr_event_kind;

typedef struct {
    sgsr_event_kind kind;
    int code;
    double data0;
    char message[512];
} sgsr_event;

typedef void (*sgsr_event_cb)(const sgsr_event *ev, void *userdata);

typedef struct {
    sgsr_source video_source;
    int fps;
    const char *video_codec; /* "auto","h264","hevc","av1" */
    const char *container;   /* "mp4","mkv","mov" */
    const char *output_path; /* o URL rtmp/srt para streaming */
    const char *audio_inputs[8]; /* mismos ids que -a del CLI */
    int num_audio_inputs;
    /* Volúmenes SOLO grabación 0-100 tipo OBS. No tocan el sistema. */
    int mic_volume_percent;
    int system_volume_percent;
    int app_volume_percent;
    int replay_buffer_secs;  /* 0 = grabación normal */
    int record_cursor;       /* 1 sí, 0 no */
    sgsr_event_cb on_event;
    void *event_userdata;
} sgsr_config;

const char* sgsr_version(void);
int sgsr_api_version(void);

/* Ciclo de vida del motor embebido (sin spawnear procesos). */
sgsr_error sgsr_init(void);
void sgsr_shutdown(void);

/* Enumeración (el llamador libera con sgsr_free_*). */
sgsr_error sgsr_enumerate_video_sources(sgsr_source **out, size_t *out_n);
sgsr_error sgsr_enumerate_audio_devices(sgsr_audio_device **out, size_t *out_n);
void sgsr_free_video_sources(sgsr_source *p);
void sgsr_free_audio_devices(sgsr_audio_device *p);

/* Control total. config debe seguir viva durante start (se copia). */
sgsr_error sgsr_start(const sgsr_config *config, sgsr_recorder **out);
sgsr_error sgsr_stop(sgsr_recorder *r);
sgsr_error sgsr_pause(sgsr_recorder *r);
sgsr_error sgsr_resume(sgsr_recorder *r);
sgsr_error sgsr_save_replay(sgsr_recorder *r, const char *path);
sgsr_error sgsr_screenshot(sgsr_recorder *r, const char *path);

/* Volúmenes en caliente 0-100 (solo grabación). */
sgsr_error sgsr_set_volumes(sgsr_recorder *r, int mic, int system, int app);

const char* sgsr_strerror(sgsr_error e);

#ifdef __cplusplus
}
#endif

#endif /* SOURIS_GSR_H */
