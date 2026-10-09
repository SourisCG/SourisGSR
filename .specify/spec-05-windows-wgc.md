# Spec-05 — Windows WGC window capture + WASAPI audio

Refs: no C equivalent exists in `reference/gpu-screen-recorder`
(`DEVIATION-WIN-NEW`). Covers the `-w <window>` story on Windows, where
the C `-w focused`/window-id path is X11-only and rejected.

## 1. Capture (MUST)

- `WgcCapturer` (`crates/capture/src/wgc.rs`) via
  `Windows.Graphics.Capture` on an explicit HWND (`-w <window_id>`,
  numeric). Per-source `;k=v` (`x/y/width/height/halign/valign/hflip/vflip`)
  and `|` composition behave exactly like spec-01.
- `-s WxH` caps output resolution (aspect-preserving fit); `-cursor`,
  `-fm`, `-v` semantics match spec-01.
- Anticheat rule from spec-04 applies unchanged.

## 2. Audio (MUST)

- `wasapi.rs` loopback for `default_output` + microphone `default_input`,
  repeatable `-a` with `|` multitrack, same `-ac/-ab` semantics as spec-01/03.

## 3. Tests (MUST)

- WGC mock: fake HWND frame source, resize/occlusion handling, `;k=v`
  layout math incl. flips and alignment.
- WASAPI mock: loopback + mic multitrack graph builds without hardware.
- `--list-capture-options` includes window/monitor entries in C line format.
