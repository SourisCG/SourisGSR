#ifndef SOURIS_GAIN_H
#define SOURIS_GAIN_H

/* SourisGSR: modificador de volumen SOLO de grabación (0-100% tipo OBS).
   Nunca toca el mixer del sistema (nada de pa_context_set_sink_volume
   ni IAudioEndpointVolume). Es un gain software post-captura aplicado
   al AVFrame antes de encode/filtro. */

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

#define SOURIS_VOLUME_MIN_PERCENT 0
#define SOURIS_VOLUME_MAX_PERCENT 100
#define SOURIS_VOLUME_DEFAULT_PERCENT 100

/* 0-100 -> 0.0f-1.0f. Clampea fuera de rango. */
float souris_volume_percent_to_gain(int percent);

/* Aplica gain a buffers intercalados. nb_samples = por canal. */
void souris_gain_apply_s16(int16_t *samples, size_t nb_samples_total, float gain);
void souris_gain_apply_s32(int32_t *samples, size_t nb_samples_total, float gain);
void souris_gain_apply_flt(float *samples, size_t nb_samples_total, float gain);

/* Planar float (AV_SAMPLE_FMT_FLTP): data[ch] con nb_samples cada uno. */
void souris_gain_apply_fltp(float **data, int nb_channels, int nb_samples, float gain);

/* Resuelve el gain efectivo para un input.
   - Si has_custom_volume: usa custom_volume (0.0-1.0, de vol=NN).
   - Si is_app: app_percent.
   - Si es device de entrada (mic): mic_percent, si no system_percent.
   is_mic_device lo decide el llamador por nombre. Todo 0-100 -> gain. */
float souris_resolve_gain(bool has_custom_volume, float custom_volume,
                          bool is_app, bool is_mic_device,
                          int mic_percent, int system_percent, int app_percent);

/* true si el nombre de device parece micrófono/entrada (heurística). */
bool souris_device_name_is_mic(const char *name);

#ifdef __cplusplus
}
#endif

#endif /* SOURIS_GAIN_H */
