/* SourisGSR — stub multiplataforma de captura Windows.
   En Linux compila y devuelve "no soportado" para no romper el build.
   En Windows (_WIN32) este archivo debe reemplazarse por la implementación
   real WGC + DXGI (ver core/docs/WINDOWS_PORT.md). */

#include "../../include/capture/win_capture.h"

int souris_win_capture_available(souris_win_capture_backend backend) {
#ifdef _WIN32
    (void)backend;
    /* TODO(win): implementar chequeo real:
       - WGC: winrt::Windows::Graphics::Capture::GraphicsCaptureSession::IsSupported()
       - DXGI: D3D11CreateDevice + IDXGIOutput1::DuplicateOutput */
    return 0;
#else
    (void)backend;
    return -1;
#endif
}

const char* souris_win_capture_backend_name(souris_win_capture_backend backend) {
    switch(backend) {
        case SOURIS_WIN_CAPTURE_WGC: return "wgc";
        case SOURIS_WIN_CAPTURE_DXGI: return "dxgi";
        default: return "unknown";
    }
}
