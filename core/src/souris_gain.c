#include "../include/souris_gain.h"
#include <string.h>
#include <strings.h>

float souris_volume_percent_to_gain(int percent) {
    if(percent < SOURIS_VOLUME_MIN_PERCENT)
        percent = SOURIS_VOLUME_MIN_PERCENT;
    if(percent > SOURIS_VOLUME_MAX_PERCENT)
        percent = SOURIS_VOLUME_MAX_PERCENT;
    return (float)percent / 100.0f;
}

void souris_gain_apply_s16(int16_t *samples, size_t nb_samples_total, float gain) {
    if(!samples || gain == 1.0f)
        return;
    if(gain == 0.0f) {
        for(size_t i = 0; i < nb_samples_total; ++i)
            samples[i] = 0;
        return;
    }
    for(size_t i = 0; i < nb_samples_total; ++i)
        samples[i] = (int16_t)((float)samples[i] * gain);
}

void souris_gain_apply_s32(int32_t *samples, size_t nb_samples_total, float gain) {
    if(!samples || gain == 1.0f)
        return;
    if(gain == 0.0f) {
        for(size_t i = 0; i < nb_samples_total; ++i)
            samples[i] = 0;
        return;
    }
    for(size_t i = 0; i < nb_samples_total; ++i)
        samples[i] = (int32_t)((double)samples[i] * (double)gain);
}

void souris_gain_apply_flt(float *samples, size_t nb_samples_total, float gain) {
    if(!samples || gain == 1.0f)
        return;
    if(gain == 0.0f) {
        for(size_t i = 0; i < nb_samples_total; ++i)
            samples[i] = 0.0f;
        return;
    }
    for(size_t i = 0; i < nb_samples_total; ++i)
        samples[i] *= gain;
}

void souris_gain_apply_fltp(float **data, int nb_channels, int nb_samples, float gain) {
    if(!data || gain == 1.0f)
        return;
    for(int ch = 0; ch < nb_channels; ++ch) {
        if(!data[ch])
            continue;
        souris_gain_apply_flt(data[ch], (size_t)nb_samples, gain);
    }
}

/* Simple heuristic: default_input / *input* / *mic* (case-insensitive)
   count as microphones; every other device is output/system. */
static bool souris_name_is_mic(const char *name) {
    if(!name || !name[0])
        return false;
    if(strcasecmp(name, "default_input") == 0)
        return true;
    if(strcasecmp(name, "default_output") == 0)
        return false;
    /* strcasestr is GNU; use a small manual case-insensitive search instead */
    char lower[256];
    size_t i = 0;
    for(; name[i] && i < sizeof(lower) - 1; ++i) {
        char c = name[i];
        lower[i] = (c >= 'A' && c <= 'Z') ? (char)(c + 32) : c;
    }
    lower[i] = '\0';
    if(strstr(lower, "input") || strstr(lower, "mic"))
        return true;
    return false;
}

bool souris_device_name_is_mic(const char *name) {
    return souris_name_is_mic(name);
}

float souris_resolve_gain(bool has_custom_volume, float custom_volume,
                          bool is_app, bool is_mic_device,
                          int mic_percent, int system_percent, int app_percent) {
    if(has_custom_volume) {
        if(custom_volume < 0.0f)
            custom_volume = 0.0f;
        if(custom_volume > 1.0f)
            custom_volume = 1.0f;
        return custom_volume;
    }
    if(is_app)
        return souris_volume_percent_to_gain(app_percent);
    if(is_mic_device)
        return souris_volume_percent_to_gain(mic_percent);
    return souris_volume_percent_to_gain(system_percent);
}
