/* SourisGSR — cross-platform stub for Windows audio (WASAPI).
   On Linux it returns unsupported. See core/docs/WINDOWS_PORT.md for the
   real implementation plan. */

#include "../include/souris_win_audio.h"

int souris_win_audio_available(souris_win_audio_kind kind) {
#ifdef _WIN32
    (void)kind;
    return 0;
#else
    (void)kind;
    return -1;
#endif
}

const char* souris_win_audio_kind_name(souris_win_audio_kind kind) {
    switch(kind) {
        case SOURIS_WIN_AUDIO_SYSTEM_LOOPBACK: return "system-loopback";
        case SOURIS_WIN_AUDIO_MIC: return "mic";
        case SOURIS_WIN_AUDIO_APP_LOOPBACK: return "app-loopback";
        default: return "unknown";
    }
}
