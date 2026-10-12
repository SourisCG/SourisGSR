#ifndef SOURIS_WIN_AUDIO_H
#define SOURIS_WIN_AUDIO_H

/* SourisGSR — audio Windows 11 vía WASAPI.
   - Sistema: loopback (IAudioClient + AUDCLNT_STREAMFLAGS_LOOPBACK).
   - Mic: captura normal (eCapture).
   - Por-app: process-loopback (requiere Win10 build 20348+, ok en Win11).
   El gain 0-100% se aplica igual que en Linux: software post-captura
   vía souris_gain.h. NUNCA IAudioEndpointVolume (no tocar sistema). */

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
