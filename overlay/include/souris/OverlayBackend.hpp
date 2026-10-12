#pragma once

/* SourisGSR — abstracción del backend del overlay estilo ShadowPlay.
   Reuso máximo: la lógica UX (hotkeys, replay, selección, config.json,
   invocación al CLI) se queda intacta. Solo se abstrae el dibujo/input.

   - Linux (existente, congelar): Overlay+EGL/X11/Wayland/Pango,
     GlobalHotkeys evdev/X11.
   - Windows (nuevo): OverlayWin con Win32 layered + D3D11 + Direct2D/
     DirectWrite, GlobalHotkeysWin con RegisterHotKey.

   La UI nunca captura video directamente; siempre maneja el CLI/core
   (igual que hoy) o el SDK Rust embebido. */

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
