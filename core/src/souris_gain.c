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

void souris_gain_apply_u8(uint8_t *samples, size_t nb_samples_total, float gain) {
    if(!samples || gain == 1.0f)
        return;
    if(gain == 0.0f) {
        for(size_t i = 0; i < nb_samples_total; ++i)
            samples[i] = 128; /* silence for unsigned 8-bit */
        return;
    }
    for(size_t i = 0; i < nb_samples_total; ++i) {
        float v = ((float)samples[i] - 128.0f) * gain + 128.0f;
        if(v < 0.0f) v = 0.0f;
        if(v > 255.0f) v = 255.0f;
        samples[i] = (uint8_t)(v + 0.5f);
    }
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

void souris_gain_apply_s64(int64_t *samples, size_t nb_samples_total, float gain) {
    if(!samples || gain == 1.0f)
        return;
    if(gain == 0.0f) {
        for(size_t i = 0; i < nb_samples_total; ++i)
            samples[i] = 0;
        return;
    }
    for(size_t i = 0; i < nb_samples_total; ++i)
        samples[i] = (int64_t)((double)samples[i] * (double)gain);
}

void souris_gain_apply_dbl(double *samples, size_t nb_samples_total, float gain) {
    if(!samples || gain == 1.0f)
        return;
    if(gain == 0.0f) {
        for(size_t i = 0; i < nb_samples_total; ++i)
            samples[i] = 0.0;
        return;
    }
    for(size_t i = 0; i < nb_samples_total; ++i)
        samples[i] *= (double)gain;
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

void souris_gain_apply_dblp(double **data, int nb_channels, int nb_samples, float gain) {
    if(!data || gain == 1.0f)
        return;
    for(int ch = 0; ch < nb_channels; ++ch) {
        if(!data[ch])
            continue;
        souris_gain_apply_dbl(data[ch], (size_t)nb_samples, gain);
    }
}

void souris_gain_apply_s16p(int16_t **data, int nb_channels, int nb_samples, float gain) {
    if(!data || gain == 1.0f)
        return;
    for(int ch = 0; ch < nb_channels; ++ch) {
        if(!data[ch])
            continue;
        souris_gain_apply_s16(data[ch], (size_t)nb_samples, gain);
    }
}

void souris_gain_apply_s32p(int32_t **data, int nb_channels, int nb_samples, float gain) {
    if(!data || gain == 1.0f)
        return;
    for(int ch = 0; ch < nb_channels; ++ch) {
        if(!data[ch])
            continue;
        souris_gain_apply_s32(data[ch], (size_t)nb_samples, gain);
    }
}

/* Word-boundary case-insensitive match to avoid "comic" -> mic false positives.
   Exact defaults first, then tokens mic/input/capture. */
static bool token_at(const char *hay, const char *tok) {
    size_t tlen = strlen(tok);
    const char *p = hay;
    while((p = strstr(p, tok)) != NULL) {
        /* Left word boundary only: "microphone" counts as mic, but "comic" does not. */
        bool left_ok = (p == hay) || (!((p[-1] >= 'a' && p[-1] <= 'z') || (p[-1] >= '0' && p[-1] <= '9') || p[-1] == '_'));
        if(left_ok)
            return true;
        p += tlen;
    }
    return false;
}

static bool souris_name_is_mic(const char *name) {
    if(!name || !name[0])
        return false;
    if(strcasecmp(name, "default_input") == 0)
        return true;
    if(strcasecmp(name, "default_output") == 0)
        return false;
    char lower[256];
    size_t i = 0;
    for(; name[i] && i < sizeof(lower) - 1; ++i) {
        char c = name[i];
        lower[i] = (c >= 'A' && c <= 'Z') ? (char)(c + 32) : c;
    }
    lower[i] = '\0';
    if(token_at(lower, "input") || token_at(lower, "mic") || token_at(lower, "capture"))
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
