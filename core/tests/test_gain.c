/* SourisGSR gain unit tests (offline, no GPU). Run via `meson test`. */
#include "../include/souris_gain.h"
#include <assert.h>
#include <math.h>
#include <stdio.h>
#include <string.h>

static int failures = 0;
#define CHECK(cond, msg) do { \
    if(!(cond)) { printf("FAIL: %s (line %d)\n", msg, __LINE__); failures++; } \
    else { printf("ok: %s\n", msg); } \
} while(0)

static int feq(float a, float b) { return fabsf(a - b) < 1e-6f; }

int main(void) {
    /* percent -> gain + clamp */
    CHECK(feq(souris_volume_percent_to_gain(100), 1.0f), "100 -> 1.0");
    CHECK(feq(souris_volume_percent_to_gain(0), 0.0f), "0 -> 0.0");
    CHECK(feq(souris_volume_percent_to_gain(80), 0.8f), "80 -> 0.8");
    CHECK(feq(souris_volume_percent_to_gain(200), 1.0f), "200 clamps to 1.0");
    CHECK(feq(souris_volume_percent_to_gain(-5), 0.0f), "-5 clamps to 0.0");

    /* s16 */
    {
        int16_t s[] = {1000, -2000, 3000};
        souris_gain_apply_s16(s, 3, 0.5f);
        CHECK(s[0] == 500 && s[1] == -1000 && s[2] == 1500, "s16 0.5");
    }
    {
        int16_t s[] = {1000, -2000};
        souris_gain_apply_s16(s, 2, 1.0f);
        CHECK(s[0] == 1000 && s[1] == -2000, "s16 1.0 no-op");
    }
    {
        int16_t s[] = {1000, -2000};
        souris_gain_apply_s16(s, 2, 0.0f);
        CHECK(s[0] == 0 && s[1] == 0, "s16 0.0 mutes");
    }
    /* s32 */
    {
        int32_t s[] = {100000, -200000};
        souris_gain_apply_s32(s, 2, 0.5f);
        CHECK(s[0] == 50000 && s[1] == -100000, "s32 0.5");
    }
    /* flt */
    {
        float s[] = {1.0f, -2.0f};
        souris_gain_apply_flt(s, 2, 0.5f);
        CHECK(feq(s[0], 0.5f) && feq(s[1], -1.0f), "flt 0.5");
    }
    /* fltp */
    {
        float ch0[] = {1.0f, 2.0f};
        float ch1[] = {3.0f, 4.0f};
        float *data[] = {ch0, ch1};
        souris_gain_apply_fltp(data, 2, 2, 0.5f);
        CHECK(feq(ch0[0], 0.5f) && feq(ch1[1], 2.0f), "fltp 0.5");
    }
    /* null safety */
    souris_gain_apply_s16(NULL, 10, 0.5f);
    souris_gain_apply_flt(NULL, 10, 0.5f);
    souris_gain_apply_fltp(NULL, 2, 2, 0.5f);
    CHECK(1, "null-safe");

    /* mic heuristic: exact defaults first, then substring */
    CHECK(souris_device_name_is_mic("default_input") == true, "default_input is mic");
    CHECK(souris_device_name_is_mic("default_output") == false, "default_output is not mic");
    CHECK(souris_device_name_is_mic("alsa_input.usb-mic") == true, "alsa_input is mic");
    CHECK(souris_device_name_is_mic("Microphone USB") == true, "Microphone is mic");
    CHECK(souris_device_name_is_mic("alsa_output.pci") == false, "alsa_output is not mic");
    CHECK(souris_device_name_is_mic("comic sans") == false, "comic is not mic (word boundary)");
    CHECK(souris_device_name_is_mic("my capture device") == true, "capture is mic");
    CHECK(souris_device_name_is_mic("") == false, "empty is not mic");
    CHECK(souris_device_name_is_mic(NULL) == false, "NULL is not mic");

    /* u8/s64/dbl */
    {
        uint8_t s[] = {128, 138};
        souris_gain_apply_u8(s, 2, 0.5f);
        CHECK(s[0] == 128 && s[1] == 133, "u8 0.5 centered");
    }
    {
        int64_t s[] = {1000000, -2000000};
        souris_gain_apply_s64(s, 2, 0.5f);
        CHECK(s[0] == 500000 && s[1] == -1000000, "s64 0.5");
    }
    {
        double s[] = {2.0, -4.0};
        souris_gain_apply_dbl(s, 2, 0.5f);
        CHECK(fabs(s[0] - 1.0) < 1e-9 && fabs(s[1] + 2.0) < 1e-9, "dbl 0.5");
    }
    {
        double ch0[] = {2.0, 4.0};
        double *d[] = {ch0};
        souris_gain_apply_dblp(d, 1, 2, 0.5f);
        CHECK(fabs(ch0[0] - 1.0) < 1e-9, "dblp 0.5");
    }

    /* resolve precedence: custom > app > mic > system */
    CHECK(feq(souris_resolve_gain(true, 0.8f, false, false, 100, 100, 100), 0.8f), "custom wins");
    CHECK(feq(souris_resolve_gain(true, 2.0f, false, false, 100, 100, 100), 1.0f), "custom clamps hi");
    CHECK(feq(souris_resolve_gain(true, -1.0f, false, false, 100, 100, 100), 0.0f), "custom clamps lo");
    CHECK(feq(souris_resolve_gain(false, 0, true, false, 100, 100, 50), 0.5f), "app uses app_percent");
    CHECK(feq(souris_resolve_gain(false, 0, false, true, 30, 100, 100), 0.3f), "mic uses mic_percent");
    CHECK(feq(souris_resolve_gain(false, 0, false, false, 100, 70, 100), 0.7f), "system uses system_percent");

    if(failures == 0) printf("ALL GAIN TESTS PASSED\n");
    return failures == 0 ? 0 : 1;
}
