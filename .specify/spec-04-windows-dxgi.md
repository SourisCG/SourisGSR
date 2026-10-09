# Spec-04 — Windows DXGI capture + D3D11VA encode

Refs: no C equivalent exists in `reference/gpu-screen-recorder`
(`DEVIATION-WIN-NEW`). Design follows the constitution zero-copy rule and
mirrors the Linux `src/capture/*` -> `src/encoder/*` layering.

## 1. Capture (MUST)

- `DxgiCapturer` (`crates/capture/src/dxgi.rs`) via
  `IDXGIOutputDuplication`: monitor enumeration (maps to `-w screen`,
  `<monitor>`, `-w region` + `-region`), cursor composition (`-cursor`),
  damage/duplicate-metadata gating (`-fm content/vfr`), `-v` reporting.
- Zero-copy: acquired `ID3D11Texture2D` stays on the shared device;
  any CPU `Map` in the hot path fails `zero_copy_guard.rs`.
- Anticheat rule: hooking `Present()` or DLL injection is FORBIDDEN.
  Only duplication APIs are allowed (audited by test).

## 2. Encode (MUST)

- `d3d11va.rs` owns the single shared `ID3D11Device`; NVENC (`nvenc.rs`),
  AMF (`amf.rs`), QSV (via `video.rs` dispatch) encode from that device.
- `-k` mapping: `h264`/`hevc` (+`h265` alias, HDR/10-bit where the OS
  stack exposes it); `-tune performance|quality`; `-encoder gpu|cpu`
  + `-fallback-cpu-encoding` (CPU = H264 only, mirrors C semantics).
- `-pixfmt yuv420|yuv444`, `-cr`, `-keyint`, `-bm/-q` semantics identical
  to spec-01.

## 3. Tests (MUST)

- DXGI mock (no GPU in CI): fake duplicated frames with damage rects,
  region math, cursor overlay.
- Shared-device test: encoder backends receive the same device handle.
- No-hooking audit: test fails if `Detours`/`MinHook`-style imports or
  `Present`-hook strings appear in `dxgi.rs`/`wgc.rs`.
