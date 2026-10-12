#pragma once

/* SourisGSR — ShadowPlay-style overlay backend abstraction.
   Maximum reuse: the UX logic (hotkeys, replay, selection, config.json,
   CLI invocation) stays untouched. Only drawing/input is abstracted.

   - Linux (existing, frozen): Overlay+EGL/X11/Wayland/Pango,
     GlobalHotkeys evdev/X11.
   - Windows (new): OverlayWin with Win32 layered + D3D11 + Direct2D/
     DirectWrite, GlobalHotkeysWin with RegisterHotKey.

   The UI never captures video directly; it always drives the CLI/core
   (as today) or the embedded Rust SDK. */

namespace souris {

enum class OverlayBackendKind {
    LinuxEgl,
    WindowsD3D11,
};

struct IOverlayBackend {
    virtual ~IOverlayBackend() = default;
    virtual bool init() = 0;
    virtual void draw_rect(float x, float y, float w, float h) = 0;
    virtual void draw_text(float x, float y, const char *text) = 0;
    virtual void present() = 0;
    virtual void shutdown() = 0;
};

struct IGlobalHotkeysBackend {
    virtual ~IGlobalHotkeysBackend() = default;
    virtual bool register_hotkey(int id, unsigned mods, unsigned vk) = 0;
    virtual void unregister_all() = 0;
};

} // namespace souris
