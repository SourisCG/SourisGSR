#ifndef SOURIS_WIN_CAPTURE_H
#define SOURIS_WIN_CAPTURE_H

/* SourisGSR — Windows 11 capture (priority) via Windows Graphics Capture.
   DXGI Desktop Duplication only as a monitor fallback.
   On Linux this header exists but the functions return GSR_ERROR_UNSUPPORTED.
   References:
   - WGC: Windows.Graphics.Capture (WinRT, HWND window + monitor capture,
     works with DirectFlip, requires Win10 1803+, best on Win11).
   - DXGI: IDXGIOutputDuplication::AcquireNextFrame (monitors only,
     must run on the same GPU as the display).
   - Encode: D3D11 textures -> FFmpeg d3d11va -> h264_nvenc/hevc_nvenc/
     av1_nvenc, h264_amf/hevc_amf, h264_qsv/hevc_qsv.
   - NVIDIA sample: nvEncDXGIOutputDuplicationSample (DDA + NVENC). */

#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
    SOURIS_WIN_CAPTURE_WGC = 0,   /* default: individual window + monitor */
    SOURIS_WIN_CAPTURE_DXGI = 1,  /* fallback: monitor only */
} souris_win_capture_backend;

typedef struct {
    souris_win_capture_backend backend;
    unsigned long long hwnd; /* HWND as integer; 0 = monitor */
    int monitor_index;
    int fps;
    bool record_cursor;
} souris_win_capture_params;

/* Returns 0 if the backend is available on this OS, negative if not.
   Always negative on Linux (stub). On Windows it checks WinRT WGC / D3D11. */
int souris_win_capture_available(souris_win_capture_backend backend);

/* Describe the source for logs/UI. */
const char* souris_win_capture_backend_name(souris_win_capture_backend backend);

#ifdef __cplusplus
}
#endif

#endif /* SOURIS_WIN_CAPTURE_H */
