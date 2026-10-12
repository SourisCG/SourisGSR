// SourisGSR — WASAPI audio for Windows 11 (system loopback + mic).
//
// - System: IAudioClient loopback (AUDCLNT_STREAMFLAGS_LOOPBACK) on the
//   default render endpoint.
// - Mic: plain eCapture on the default capture endpoint.
// - Per-app: process-loopback (Windows 10 build 20348+, fine on Win11);
//   same wiring pattern, filtered by process id.
//
// Recording-only gain (0-100%) is applied later with souris_gain.h, exactly
// like Linux. NEVER IAudioEndpointVolume (the system mixer is not touched).
//
// NOTE: compiles on Windows only. Not built on Linux.

#ifdef _WIN32

#include <windows.h>
#include <new>
#include <mmdeviceapi.h>
#include <audioclient.h>
#include <avrt.h>

extern "C" {
#include "souris_win_audio.h"
}

// PCM float frames delivered to the pipeline. The capture thread (added with
// the gsr_audio_device integration) pulls via souris_wasapi_read(); gain is
// applied downstream by souris_gain_apply_flt().
struct souris_wasapi_stream {
    IAudioClient *client = nullptr;
    IAudioCaptureClient *capture = nullptr;
    HANDLE event = nullptr;
    WAVEFORMATEX *format = nullptr; // CoTaskMemFree on destroy
    UINT32 buffer_frames = 0;
    bool loopback = false;
};

extern "C" {

// kind: 0 = system loopback, 1 = mic. Returns NULL on failure.
souris_wasapi_stream* souris_wasapi_open(int kind) {
    auto *s = new (std::nothrow) souris_wasapi_stream();
    if(!s)
        return nullptr;
    s->loopback = (kind == SOURIS_WIN_AUDIO_SYSTEM_LOOPBACK);

    IMMDeviceEnumerator *enumerator = nullptr;
    IMMDevice *device = nullptr;
    bool ok = false;
    if(SUCCEEDED(CoCreateInstance(__uuidof(MMDeviceEnumerator), nullptr, CLSCTX_ALL,
                                  __uuidof(IMMDeviceEnumerator), (void **)&enumerator)) &&
       SUCCEEDED(enumerator->GetDefaultAudioEndpoint(
           s->loopback ? eRender : eCapture, eConsole, &device)) &&
       SUCCEEDED(device->Activate(__uuidof(IAudioClient), CLSCTX_ALL, nullptr, (void **)&s->client))) {
        if(SUCCEEDED(s->client->GetMixFormat(&s->format))) {
            REFERENCE_TIME duration = 0;
            DWORD flags = s->loopback ? AUDCLNT_STREAMFLAGS_LOOPBACK | AUDCLNT_STREAMFLAGS_EVENTCALLBACK
                                      : AUDCLNT_STREAMFLAGS_EVENTCALLBACK;
            s->event = CreateEvent(nullptr, FALSE, FALSE, nullptr);
            if(s->event &&
               SUCCEEDED(s->client->Initialize(AUDCLNT_SHAREMODE_SHARED, flags,
                                               100 * 10000, 0, s->format, nullptr)) &&
               SUCCEEDED(s->client->GetBufferSize(&s->buffer_frames)) &&
               SUCCEEDED(s->client->SetEventHandle(s->event)) &&
               SUCCEEDED(s->client->GetService(__uuidof(IAudioCaptureClient), (void **)&s->capture))) {
                ok = true;
            }
        }
    }
    if(enumerator)
        enumerator->Release();
    if(device)
        device->Release();
    if(!ok) {
        if(s->capture)
            s->capture->Release();
        if(s->client)
            s->client->Release();
        if(s->format)
            CoTaskMemFree(s->format);
        if(s->event)
            CloseHandle(s->event);
        delete s;
        return nullptr;
    }
    return s;
}

void souris_wasapi_start(souris_wasapi_stream *s) {
    if(s && s->client)
        s->client->Start();
}

void souris_wasapi_stop(souris_wasapi_stream *s) {
    if(s && s->client)
        s->client->Stop();
}

// Sample rate / channels negotiated with the endpoint (mix format).
void souris_wasapi_format(const souris_wasapi_stream *s, int *sample_rate, int *channels) {
    if(sample_rate)
        *sample_rate = (s && s->format) ? s->format->nSamplesPerSec : 0;
    if(channels)
        *channels = (s && s->format) ? s->format->nChannels : 0;
}

void souris_wasapi_close(souris_wasapi_stream *s) {
    if(!s)
        return;
    souris_wasapi_stop(s);
    if(s->capture)
        s->capture->Release();
    if(s->client)
        s->client->Release();
    if(s->format)
        CoTaskMemFree(s->format);
    if(s->event)
        CloseHandle(s->event);
    delete s;
}

} // extern "C"

#endif // _WIN32
