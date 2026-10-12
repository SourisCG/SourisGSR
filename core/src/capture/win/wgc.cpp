// SourisGSR — Windows Graphics Capture backend (default on Windows 11).
//
// Captures individual windows (HWND) and monitors through
// Windows.Graphics.Capture (WinRT). Frames arrive as ID3D11Texture2D, so
// they can feed FFmpeg d3d11va/NVENC/AMF/QSV with zero CPU copies,
// mirroring the Linux zero-copy design.
//
// Integration point: wrap this session in a gsr_capture (see
// include/capture/capture.h, like portal.c does for PipeWire).
//
// NOTE: compiles on Windows only (MSVC + C++/WinRT). Not built on Linux.

#ifdef _WIN32

#include <windows.h>
#include <new>
#include <d3d11.h>
#include <dxgi1_6.h>

// C++/WinRT (vcpkg: cppwinrt)
#include <winrt/Windows.Foundation.h>
#include <winrt/Windows.Graphics.Capture.h>
#include <winrt/Windows.Graphics.DirectX.Direct3d11.h>
#include <windows.graphics.directx.direct3d11.interop.h>
#include <windows.graphics.capture.interop.h>

extern "C" {
#include "capture/win_capture.h"
}

using namespace winrt::Windows::Graphics::Capture;
using namespace winrt::Windows::Graphics::DirectX::Direct3d11;

// Opaque session, owned by the gsr_capture wrapper.
struct souris_wgc_session {
    winrt::com_ptr<ID3D11Device> d3d_device;
    winrt::com_ptr<ID3D11DeviceContext> d3d_context;
    IDirect3DDevice winrt_device{nullptr};
    GraphicsCaptureItem item{nullptr};
    Direct3D11CaptureFramePool pool{nullptr};
    GraphicsCaptureSession session{nullptr};
    bool started = false;
};

static IDirect3DDevice dxgi_to_winrt_device(ID3D11Device *d3d) {
    auto dxgi = d3d->as<IDXGIDevice>();
    winrt::com_ptr<IInspectable> inspectable;
    winrt::check_hresult(CreateDirect3D11DeviceFromDXGIDevice(dxgi.get(), inspectable.put()));
    return inspectable.as<IDirect3DDevice>();
}

static GraphicsCaptureItem item_from_hwnd(HWND hwnd) {
    auto factory = winrt::get_activation_factory<GraphicsCaptureItem, IGraphicsCaptureItemInterop>();
    GraphicsCaptureItem item{nullptr};
    winrt::check_hresult(factory->CreateForWindow(
        hwnd, winrt::guid_of<ABI::Windows::Graphics::Capture::IGraphicsCaptureItem>(),
        reinterpret_cast<void **>(winrt::put_abi(item))));
    return item;
}

static GraphicsCaptureItem item_from_monitor(HMONITOR monitor) {
    auto factory = winrt::get_activation_factory<GraphicsCaptureItem, IGraphicsCaptureItemInterop>();
    GraphicsCaptureItem item{nullptr};
    winrt::check_hresult(factory->CreateForMonitor(
        monitor, winrt::guid_of<ABI::Windows::Graphics::Capture::IGraphicsCaptureItem>(),
        reinterpret_cast<void **>(winrt::put_abi(item))));
    return item;
}

extern "C" {

// 0 when WGC capture sessions are supported on this system.
int souris_wgc_is_supported(void) {
    // WinRT must be initialized on the calling thread (winrt::init_apartment).
    return GraphicsCaptureSession::IsSupported() ? 0 : -1;
}

// Create a session. hwnd != 0 captures that window, else monitor_index
// selects the display (0 = primary). Returns NULL on failure.
souris_wgc_session* souris_wgc_session_create(unsigned long long hwnd, int monitor_index, int *out_width, int *out_height) {
    auto *s = new (std::nothrow) souris_wgc_session();
    if(!s)
        return nullptr;

    UINT flags = D3D11_CREATE_DEVICE_BGRA_SUPPORT;
#ifdef _DEBUG
    flags |= D3D11_CREATE_DEVICE_DEBUG;
#endif
    D3D_FEATURE_LEVEL levels[] = {D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0};
    winrt::com_ptr<ID3D11Device> device;
    winrt::com_ptr<ID3D11DeviceContext> context;
    D3D_FEATURE_LEVEL chosen = D3D_FEATURE_LEVEL_11_0;
    if(FAILED(D3D11CreateDevice(nullptr, D3D_DRIVER_TYPE_HARDWARE, nullptr, flags,
                                levels, ARRAYSIZE(levels), D3D11_SDK_VERSION,
                                device.put(), &chosen, context.put()))) {
        delete s;
        return nullptr;
    }
    s->d3d_device = device;
    s->d3d_context = context;
    s->winrt_device = dxgi_to_winrt_device(device.get());

    try {
        if(hwnd != 0) {
            s->item = item_from_hwnd(reinterpret_cast<HWND>(hwnd));
        } else {
            // Nth monitor via EnumDisplayMonitors.
            struct Ctx { int want; int seen; HMONITOR found; };
            Ctx ctx{monitor_index, 0, nullptr};
            EnumDisplayMonitors(nullptr, nullptr,
                [](HMONITOR h, HDC, LPRECT, LPARAM p) -> BOOL {
                    auto *c = reinterpret_cast<Ctx *>(p);
                    if(c->seen == c->want) {
                        c->found = h;
                        return FALSE;
                    }
                    ++c->seen;
                    return TRUE;
                },
                reinterpret_cast<LPARAM>(&ctx));
            if(!ctx.found) {
                delete s;
                return nullptr;
            }
            s->item = item_from_monitor(ctx.found);
        }
        const auto size = s->item.Size();
        if(out_width)
            *out_width = size.Width;
        if(out_height)
            *out_height = size.Height;
        s->pool = Direct3D11CaptureFramePool::Create(
            s->winrt_device, winrt::Windows::Graphics::DirectX::DXGI_FORMAT_B8G8R8A8_UNORM,
            2, size);
        s->session = s->pool.CreateCaptureSession(s->item);
        // NOTE: IsCursorCaptureEnabled(false) keeps the HW cursor plane,
        // matching the DXGI path. Default (true) uses a software cursor.
    } catch(...) {
        delete s;
        return nullptr;
    }
    return s;
}

void souris_wgc_session_start(souris_wgc_session *s) {
    if(s && !s->started) {
        s->session.StartCapture();
        s->started = true;
    }
}

// Newest frame as ID3D11Texture2D* (owned: release with
// souris_wgc_texture_release). Returns NULL when no new frame is ready.
// The texture holds its own reference, so outliving the pool frame is safe.
void* souris_wgc_session_acquire(souris_wgc_session *s) {
    if(!s || !s->started)
        return nullptr;
    try {
        auto frame = s->pool.TryGetNextFrame();
        if(!frame)
            return nullptr;
        auto access = frame.Surface().as<IDirect3DDxgiInterfaceAccess>();
        winrt::com_ptr<ID3D11Texture2D> texture;
        winrt::check_hresult(access->GetInterface(IID_PPV_ARGS(texture.put())));
        return texture.detach(); // ownership to caller
    } catch(...) {
        return nullptr;
    }
}

void souris_wgc_texture_release(void *texture) {
    if(texture)
        static_cast<IUnknown *>(texture)->Release();
}

void souris_wgc_session_destroy(souris_wgc_session *s) {
    delete s;
}

} // extern "C"

#endif // _WIN32
