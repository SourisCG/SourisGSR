#ifndef SOURIS_WIN_AUDIO_H
#define SOURIS_WIN_AUDIO_H

/* SourisGSR — Windows 11 audio via WASAPI.
   - System: loopback (IAudioClient + AUDCLNT_STREAMFLAGS_LOOPBACK).
   - Mic: plain capture (eCapture).
   - Per-app: process-loopback (requires Win10 build 20348+, fine on Win11).
   The 0-100% gain is applied exactly like on Linux: post-capture software
   via souris_gain.h. NEVER IAudioEndpointVolume (don't touch the system). */

#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
    SOURIS_WIN_AUDIO_SYSTEM_LOOPBACK = 0,
    SOURIS_WIN_AUDIO_MIC = 1,
    SOURIS_WIN_AUDIO_APP_LOOPBACK = 2,
} souris_win_audio_kind;

int souris_win_audio_available(souris_win_audio_kind kind);
const char* souris_win_audio_kind_name(souris_win_audio_kind kind);

#ifdef __cplusplus
}
#endif

#endif /* SOURIS_WIN_AUDIO_H */
