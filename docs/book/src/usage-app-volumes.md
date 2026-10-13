# Usage: App Capture + Volumes

All volumes are **recording only** (post-capture software gain via `souris_gain.h`).
The system mixer is never touched (no `pa_context_set_sink_volume`, no `IAudioEndpointVolume`).

Volumes are **per audio input**: every input row in the overlay has its own
0-100 slider (drag, arrow keys, mouse wheel, double-click resets to 100),
stored as a `|vol=NN|` suffix. There are no global volume controls in the UI.

## Per-input vol=

`vol=NN` attaches to the previous input in the same `-a` track:

```sh
-a "default_output|vol=80|default_input|vol=90"
-a "app:firefox|vol=70"
```

Rules:

- `vol=` without a prior input errors.
- Out-of-range clamps 0-100.
- An input without `vol=` falls back to the engine globals
  (`-mic-volume` for mic-like names such as `default_input`, `*mic*`, `*input*`,
  `*capture*` word-start; `-app-volume` for `app:`; `-system-volume` otherwise;
  all default 100). The globals remain available as deprecated CLI/SDK
  fallbacks but are no longer shown in the overlay.
- App-audio tracks mix upstream into one combined PipeWire sink:
  a single custom `vol=` on app members is honored for the whole sink;
  mixed customs fall back to `-app-volume` with a warning.

## App-only capture (PipeWire required)

```sh
sgsr --list-application-audio
sgsr -w portal -f 60 -a "app:firefox" -o video.mp4
sgsr -w portal -f 60 -a "app:firefox|app:chrome" -o video.mp4
sgsr -w portal -f 60 -a "app-inverse:firefox" -o video.mp4
```

Pure PulseAudio (no PipeWire) returns `UNSUPPORTED` for `app:` by design.

## Live update

```sh
sgsr -w screen -f 60 -a default_output -o video.mp4 -ipc "$XDG_RUNTIME_DIR/gsr.sock" &
sgsr-cli -ipc "$XDG_RUNTIME_DIR/gsr.sock" set-volumes 80 100 90
```

IPC `set-volumes` takes `{"mic":0-100,"system":0-100,"app":0-100}`.
Overlay Audio section is per-input sliders only (plus a note explaining it).
SDK: `.mic_volume(90).system_volume(80)` + hot `set_volumes()` + `LevelMeter` events.

## System tray (overlay only, never in the SDK)

`sgsr-ui` registers an `org.kde.StatusNotifierItem` tray icon (toggle the
overlay with left/middle click). No right-click menu in v1: libdbus aborts on
dbusmenu layouts whose leaves end in an empty nested array, so `ContextMenu`
answers `/`. Record/Replay/Quit stay on hotkeys, overlay buttons and
`sgsr-ui-cli`. Toggle with the "Show system tray icon" checkbox in global
settings (takes effect on restart). The SDK is headless by construction and
never creates windows, tray icons or notifications.
