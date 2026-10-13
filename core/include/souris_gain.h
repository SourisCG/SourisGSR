#ifndef SOURIS_GAIN_H
#define SOURIS_GAIN_H

/* SourisGSR: recording-only volume modifier (0-100%, OBS style).
   Never touches the system mixer (no pa_context_set_sink_volume
   nor IAudioEndpointVolume). Software post-capture gain applied
   to the AVFrame before encoding/filtering. */

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

#define SOURIS_VOLUME_MIN_PERCENT 0
#define SOURIS_VOLUME_MAX_PERCENT 100
#define SOURIS_VOLUME_DEFAULT_PERCENT 100

/* 0-100 -> 0.0f-1.0f. Clamps out of range. */
float souris_volume_percent_to_gain(int percent);

/* Apply gain to interleaved buffers. nb_samples = per channel. */
void souris_gain_apply_u8(uint8_t *samples, size_t nb_samples_total, float gain);
void souris_gain_apply_s16(int16_t *samples, size_t nb_samples_total, float gain);
void souris_gain_apply_s32(int32_t *samples, size_t nb_samples_total, float gain);
void souris_gain_apply_s64(int64_t *samples, size_t nb_samples_total, float gain);
void souris_gain_apply_flt(float *samples, size_t nb_samples_total, float gain);
void souris_gain_apply_dbl(double *samples, size_t nb_samples_total, float gain);

/* Planar float (AV_SAMPLE_FMT_FLTP): data[ch] with nb_samples each. */
void souris_gain_apply_fltp(float **data, int nb_channels, int nb_samples, float gain);
void souris_gain_apply_dblp(double **data, int nb_channels, int nb_samples, float gain);
void souris_gain_apply_s16p(int16_t **data, int nb_channels, int nb_samples, float gain);
void souris_gain_apply_s32p(int32_t **data, int nb_channels, int nb_samples, float gain);

/* Resolve the effective gain for one input.
   - If has_custom_volume: use custom_volume (0.0-1.0, from vol=NN).
   - If is_app: app_percent.
   - If it is an input (mic) device: mic_percent, else system_percent.
   The caller decides is_mic_device by name. Everything 0-100 -> gain. */
float souris_resolve_gain(bool has_custom_volume, float custom_volume,
                          bool is_app, bool is_mic_device,
                          int mic_percent, int system_percent, int app_percent);

/* true if the device name looks like a mic/input (heuristic). */
bool souris_device_name_is_mic(const char *name);

#ifdef __cplusplus
}
#endif

#endif /* SOURIS_GAIN_H */
