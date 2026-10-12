// SourisGSR — DXGI Desktop Duplication fallback (monitors only).
//
// Used when WGC is unavailable or for HW-cursor diagnostics. Requires
// running on the same GPU as the display (unlike WGC, which is cross-GPU).
// See the NVIDIA bulletin recommending DDA over NvFBC on Windows 10+ and
// the nvEncDXGIOutputDuplicationSample for the encode pairing.
//
// NOTE: compiles on Windows only. Not built on Linux.

#ifdef _WIN32

#include <windows.h>
#include <new>
#include <d3d11.h>
#include <dxgi1_2.h>

extern "C" {
#include "capture/win_capture.h"
}

struct souris_dxgi_session {
    ID3D11Device *device = nullptr;
    ID3D11DeviceContext *context = nullptr;
    IDXGIOutputDuplication *duplication = nullptr;
    ID3D11Texture2D *last_frame = nullptr; // owned
    int width = 0;
    int height = 0;
};

extern "C" {

// Create a duplication session for monitor_index (0 = primary).
// Returns NULL on failure (e.g. wrong GPU, protected content).
souris_dxgi_session* souris_dxgi_session_create(int monitor_index, int *out_width, int *out_height) {
    auto *s = new (std::nothrow) souris_dxgi_session();
    if(!s)
        return nullptr;

    D3D_FEATURE_LEVEL levels[] = {D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0};
    D3D_FEATURE_LEVEL chosen = D3D_FEATURE_LEVEL_11_0;
    if(FAILED(D3D11CreateDevice(nullptr, D3D_DRIVER_TYPE_HARDWARE, nullptr, 0,
                                levels, ARRAYSIZE(levels), D3D11_SDK_VERSION,
                                &s->device, &chosen, &s->context))) {
        delete s;
        return nullptr;
    }

    IDXGIDevice *dxgi_device = nullptr;
    IDXGIAdapter *adapter = nullptr;
    IDXGIOutput *output = nullptr;
    IDXGIOutput1 *output1 = nullptr;
    bool ok = false;
    if(SUCCEEDED(s->device->QueryInterface(__uuidof(IDXGIDevice), (void **)&dxgi_device)) &&
       SUCCEEDED(dxgi_device->GetAdapter(&adapter)) &&
       SUCCEEDED(adapter->EnumOutputs(monitor_index, &output)) &&
       SUCCEEDED(output->QueryInterface(__uuidof(IDXGIOutput1), (void **)&output1))) {
        DXGI_OUTDUPL_DESC desc;
        output1->GetDesc(&desc);
        // NOTE: rotation is reported un-rotated by AcquireNextFrame; the
        // gsr_capture wrapper must rotate like the Linux KMS path does not
        // need to (see desktop-duplication docs on rotated modes).
        s->width = desc.ModeDesc.Width;
        s->height = desc.ModeDesc.Height;
        if(SUCCEEDED(output1->DuplicateOutput(s->device, &s->duplication)))
            ok = true;
    }
    if(dxgi_device)
        dxgi_device->Release();
    if(adapter)
        adapter->Release();
    if(output)
        output->Release();
    if(output1)
        output1->Release();
    if(!ok) {
        if(s->context)
            s->context->Release();
        if(s->device)
            s->device->Release();
        delete s;
        return nullptr;
    }
    if(out_width)
        *out_width = s->width;
    if(out_height)
        *out_height = s->height;
    return s;
}

// Acquire the newest desktop image (ID3D11Texture2D*, owned: release with
// souris_dxgi_frame_release). Waits up to timeout_ms. Returns NULL on
// timeout; DXGI_ERROR_ACCESS_LOST means re-create the session.
void* souris_dxgi_session_acquire(souris_dxgi_session *s, int timeout_ms) {
    if(!s)
        return nullptr;
    if(s->last_frame) {
        s->last_frame->Release();
        s->last_frame = nullptr;
    }
    IDXGIResource *resource = nullptr;
    DXGI_OUTDUPL_FRAME_INFO info;
    HRESULT hr = s->duplication->AcquireNextFrame(timeout_ms, &info, &resource);
    if(hr == DXGI_ERROR_WAIT_TIMEOUT)
        return nullptr;
    if(FAILED(hr))
        return nullptr; // incl. DXGI_ERROR_ACCESS_LOST -> recreate session
    ID3D11Texture2D *texture = nullptr;
    if(FAILED(resource->QueryInterface(__uuidof(ID3D11Texture2D), (void **)&texture))) {
        resource->Release();
        s->duplication->ReleaseFrame();
        return nullptr;
    }
    resource->Release();
    s->last_frame = texture;
    s->duplication->ReleaseFrame();
    return texture; // owned by session until next acquire/destroy
}

void souris_dxgi_frame_release(void *texture) {
    // Frames are owned by the session; explicit release is a no-op helper
    // for API symmetry with the WGC path.
    (void)texture;
}

void souris_dxgi_session_destroy(souris_dxgi_session *s) {
    if(!s)
        return;
    if(s->last_frame)
        s->last_frame->Release();
    if(s->duplication)
        s->duplication->Release();
    if(s->context)
        s->context->Release();
    if(s->device)
        s->device->Release();
    delete s;
}

} // extern "C"

#endif // _WIN32
