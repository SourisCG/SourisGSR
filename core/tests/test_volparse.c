/* SourisGSR vol= parse tests (offline, no GPU/Pulse needed).
   Run via `meson test`. Links libsouris_gsr objects for the parsers. */
#include "../include/recorder/audio_input.h"
#include "../include/recorder/error.h"
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
    /* 1. single input with vol= */
    {
        gsr_merged_audio_inputs m;
        CHECK(gsr_merged_audio_inputs_parse(&m, "default_output|vol=80") == GSR_ERROR_OK, "parse default_output|vol=80");
        CHECK(m.num_items == 1, "one item");
        CHECK(strcmp(m.items[0].name, "default_output") == 0, "name kept");
        CHECK(m.items[0].has_custom_volume && feq(m.items[0].volume, 0.8f), "vol 0.8 attached");
        gsr_merged_audio_inputs_deinit(&m);
    }
    /* 2. two inputs with own vols */
    {
        gsr_merged_audio_inputs m;
        CHECK(gsr_merged_audio_inputs_parse(&m, "default_output|vol=80|default_input|vol=90") == GSR_ERROR_OK, "parse two vols");
        CHECK(m.num_items == 2, "two items");
        CHECK(feq(m.items[0].volume, 0.8f) && m.items[0].has_custom_volume, "first 0.8");
        CHECK(feq(m.items[1].volume, 0.9f) && m.items[1].has_custom_volume, "second 0.9");
        gsr_merged_audio_inputs_deinit(&m);
    }
    /* 3. vol= without prior input errors */
    {
        gsr_merged_audio_inputs m;
        memset(&m, 0, sizeof(m));
        CHECK(gsr_merged_audio_inputs_parse(&m, "vol=50") != GSR_ERROR_OK, "lone vol= errors");
    }
    /* 4. clamping */
    {
        gsr_merged_audio_inputs m;
        CHECK(gsr_merged_audio_inputs_parse(&m, "default_output|vol=200") == GSR_ERROR_OK, "parse vol=200");
        CHECK(feq(m.items[0].volume, 1.0f), "vol=200 clamps to 1.0");
        gsr_merged_audio_inputs_deinit(&m);
        memset(&m, 0, sizeof(m));
        CHECK(gsr_merged_audio_inputs_parse(&m, "default_output|vol=-5") == GSR_ERROR_OK, "parse vol=-5");
        CHECK(feq(m.items[0].volume, 0.0f), "vol=-5 clamps to 0.0");
        gsr_merged_audio_inputs_deinit(&m);
    }
    /* 5. no vol= keeps global path (has_custom_volume false) */
    {
        gsr_merged_audio_inputs m;
        CHECK(gsr_merged_audio_inputs_parse(&m, "default_output") == GSR_ERROR_OK, "parse bare input");
        CHECK(!m.items[0].has_custom_volume && feq(m.items[0].volume, 1.0f), "bare defaults 1.0 no-custom");
        gsr_merged_audio_inputs_deinit(&m);
    }
    /* 6. app: type preserved with vol= */
    {
        gsr_merged_audio_inputs m;
        CHECK(gsr_merged_audio_inputs_parse(&m, "app:firefox|vol=70") == GSR_ERROR_OK, "parse app vol");
        CHECK(m.items[0].type == GSR_AUDIO_INPUT_TYPE_APPLICATION, "app type");
        CHECK(strcmp(m.items[0].name, "firefox") == 0, "app name stripped");
        CHECK(feq(m.items[0].volume, 0.7f), "app vol 0.7");
        gsr_merged_audio_inputs_deinit(&m);
    }
    /* 7. tracks_parse with fabricated devices (no Pulse needed) */
    {
        gsr_audio_devices devs;
        memset(&devs, 0, sizeof(devs));
        snprintf(devs.default_output, sizeof(devs.default_output), "fake_out");
        snprintf(devs.default_input, sizeof(devs.default_input), "fake_in");
        const char *args[] = {"default_output|vol=80", "app:firefox"};
        gsr_audio_input_tracks tracks;
        CHECK(gsr_audio_input_tracks_parse(&tracks, args, 2, &devs) == GSR_ERROR_OK, "tracks parse ok");
        CHECK(tracks.num_items == 2, "two tracks");
        CHECK(tracks.items[0].items[0].has_custom_volume, "track0 keeps custom");
        gsr_audio_input_tracks_deinit(&tracks);
    }
    /* 8. resolve precedence matrix */
    {
        CHECK(feq(souris_resolve_gain(true, 0.5f, false, false, 100, 100, 100), 0.5f), "custom wins over all");
        CHECK(feq(souris_resolve_gain(false, 0, true, false, 0, 0, 70), 0.7f), "app uses app_percent");
        CHECK(feq(souris_resolve_gain(false, 0, false, true, 30, 90, 70), 0.3f), "mic uses mic_percent");
        CHECK(feq(souris_resolve_gain(false, 0, false, false, 30, 90, 70), 0.9f), "plain uses system_percent");
    }

    if(failures == 0) printf("ALL VOLPARSE TESTS PASSED\n");
    return failures == 0 ? 0 : 1;
}
