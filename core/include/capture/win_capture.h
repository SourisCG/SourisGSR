#ifndef SOURIS_WIN_CAPTURE_H
#define SOURIS_WIN_CAPTURE_H

/* SourisGSR — captura Windows 11 (prioritaria) vía Windows Graphics Capture.
   DXGI Desktop Duplication solo como fallback para monitores.
   En Linux este header existe pero las funciones devuelven GSR_ERROR_UNSUPPORTED.
   Referencias:
   - WGC: Windows.Graphics.Capture (WinRT, captura ventana HWND + monitor,
     funciona con DirectFlip, requiere Win10 1803+, ideal Win11).
   - DXGI: IDXGIOutputDuplication::AcquireNextFrame (solo monitores,
     necesita correr en la misma GPU que el display).
   - Encode: texturas D3D11 -> FFmpeg d3d11va -> h264_nvenc/hevc_nvenc/
     av1_nvenc, h264_amf/hevc_amf, h264_qsv/hevc_qsv.
   - Sample NVIDIA: nvEncDXGIOutputDuplicationSample (DDA + NVENC). */

#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
    SOURIS_WIN_CAPTURE_WGC = 0,   /* defecto: ventana individual + monitor */
    SOURIS_WIN_CAPTURE_DXGI = 1,  /* fallback: solo monitor */
} souris_win_capture_backend;

typedef struct {
    souris_win_capture_backend backend;
    unsigned long long hwnd; /* HWND como entero; 0 = monitor */
    int monitor_index;
    int fps;
    bool record_cursor;
} souris_win_capture_params;

/* Devuelve 0 si el backend está disponible en este OS, negativo si no.
   En Linux siempre negativo (stub). En Windows chequea WinRT WGC / D3D11. */
int souris_win_capture_available(souris_win_capture_backend backend);

/* Describe la fuente para logs/UI. */
const char* souris_win_capture_backend_name(souris_win_capture_backend backend);

#ifdef __cplusplus
}
#endif

#endif /* SOURIS_WIN_CAPTURE_H */
