# Spec-01 — Core CLI / Config / Replay / Muxer

Refs (verified against frozen reference v5.10.2):
`src/args_parser.c`, `include/args_parser.h`, `src/defs.c`, `include/defs.h`,
`src/main.cpp`, `src/replay_buffer/replay_buffer.c`,
`src/replay_buffer/replay_buffer_ram.c`,
`src/replay_buffer/replay_buffer_disk.c`, `src/encoder/encoder.c`,
`include/encoder/encoder.h`, `src/image_writer.c`, `include/image_writer.h`,
`src/plugins.c`, `include/plugins.h`, `plugin/plugin.h`,
`src/utils.c`, `src/library_loader.c`, `src/cuda.c`,
`gpu-screen-recorder.1`, `meson.build`.

X11-only sources (`src/capture/xcomposite.c`, `src/capture/ximage.c`,
`src/capture/nvfbc.c`, `src/window/x11.c`, `src/overclock.c`, `src/xnvctrl.c`)
are IGNORED permanently (`DEVIATION-X11-NEVER`).

## 1. CLI parity (MUST)

Exact v5.10.2 flag set (from `args_parser.c` lines 737-768):

`-w` (required, string) `-c` `-f` (1-1000) `-s` `-region` `-a` (list)
`-q` `-o` `-ro` `-r` (2-86400) `-restart-replay-on-save` (yes/no)
`-k` (enum) `-ac` (enum) `-ab` (0-50000) `-oc` (yes/no, see DEVIATION)
`-fm` (enum) `-bm` (enum) `-pixfmt` (enum) `-v` `-gl-debug` `-df`
`-sc` `-cr` `-tune` `-cursor` `-keyint` (0-500) `-restore-portal-session`
`-portal-session-token-filepath` `-encoder` `-fallback-cpu-encoding`
`-replay-storage` `-p` (list),
plus `--help`/`-h`, `--version`, `--info`,
`--list-capture-options [card_path]`, `--list-audio-devices`,
`--list-application-audio`.

Enums (exact names from C):

- `-k`: `auto h264 h265 hevc hevc_hdr hevc_10bit av1 av1_hdr av1_10bit vp8 vp9`.
  `h265` is an alias of `hevc`. Default `auto` -> `h264`.
- `-ac`: `opus aac flac`. Default `opus`. `flac` prints
  `warn_flac_disabled_fallback_opus` and uses Opus (as in C).
- `-encoder`: `gpu cpu`. Default `gpu`. CPU only for H264.
- `-pixfmt`: `yuv420 yuv444`. Default `yuv420`.
- `-fm`: `vfr cfr content`. Default `vfr`. `content` only on KMS/portal.
- `-bm`: `auto qp cbr vbr`. Default `auto` -> `qp`.
- `-cr`: `limited full`. Default `limited`.
- `-tune`: `performance quality`. Default `performance` (NVIDIA only).
- `-replay-storage`: `ram disk`. Default `ram`.
- Booleans are strictly `yes|no` (anything else is an error, as in C).
- `-a` and `-p` are repeatable (list). Any other flag repeated is an error.
- Unknown flag, missing value, or out-of-range integer/double is a localized
  error on stderr + usage on stdout + exit code 2 (mirrors C `usage()`).

`-w` values (from `main.cpp` + `args_parser.c` usage text):

- `screen`: first monitor found (KMS).
- `screen-direct` / `screen-direct-force`: NVIDIA X11 only ->
  localized `err_unsupported_x11` (`DEVIATION-X11-NEVER`).
- `focused`: X11 focused window, requires `-s` -> `err_unsupported_x11`.
- `portal`: xdg-desktop-portal ScreenCast + PipeWire (Wayland only).
- `region`: requires `-region WxH+X+Y` (width/height may be 0 = whole monitor
  containing the position). `-region` without `-w region` is an error.
- Monitor name (e.g. `DP-1`) or connector: KMS capture.
- Raw window id: X11 only -> `err_unsupported_x11`.
- `DEVIATION-FUTURE`: V4L2 camera paths (`/dev/videoN`) do NOT exist in
  v5.10.2 and are rejected with `err_unsupported_source_future`; camera
  support is v2 after upstream re-sync.

`-o` is a file, except replay mode (`-r` set) where it is a directory.
`-c` defaults from `-o` extension (`mkv` -> `matroska` internally).
`-f` default 60. `-q` default `very_high`; with `-bm cbr` it must be an
integer bitrate in kbps (else localized error).
`-ab` default 0 = automatic (x1000 internally). `-keyint` default 2.0.
`-s` format `WxH`, `0x0` = original; required and exact when `-w focused`
(X11 path, therefore unreachable in v1 but still validated).
`-oc` parses (compatibility) but on Wayland/Windows prints
`warn_oc_requires_x11` and continues without overclocking
(`DEVIATION-X11-NEVER`).

Crates: `crates/core/src/cli.rs` (parser), `config.rs` (validated `Config`),
`main.rs` (signal loop). No hardcoded strings; all output via `crates/i18n`.

## 2. Signals (MUST, v5.10.2 behavior)

From `main.cpp` / service (`KillSignal=SIGINT`):

- `SIGINT` (Ctrl+C): stop and save recording; in replay mode stop without save.
- `SIGUSR1`: save replay (replay mode only).
- `SIGUSR2`: pause/unpause (not for streaming/replay).
- `SIGRTMIN`: start/stop regular `-ro` recording during replay/streaming.
- `SIGRTMIN+1/+2/+3/+4/+5/+6`: save last 10s/30s/60s/5min/10min/30min.
- Saved replay path goes to stdout; everything else to stderr (scriptable).
- Windows: map to Ctrl-C + named-pipe commands (`DEVIATION-WIN-NEW`).
- `DEVIATION-FUTURE`: UNIX-socket JSON `-ipc` does NOT exist in v5.10.2;
  tracked for v2 (spec-06 notes the reserved key namespace).

## 3. Replay ring (MUST)

`src/replay_buffer/*`:

- `-r 2..86400`, `-replay-storage ram|disk`, `-restart-replay-on-save`,
  `-df yes|no` date folders, `-sc` script `(path, regular|replay|screenshot)`.
- RAM default (encoded packets only). CBR recommended for bounded RAM/disk.
- Modules: `replay.rs` (facade) + `replay_ram.rs` + `replay_disk.rs`,
  mirroring the three C files.
- Unit tests: ring eviction order, save-last-N, restart flag clears buffer
  only when the whole buffer is saved, date-folder path format.

## 4. Muxer (MUST)

`src/encoder/encoder.c` via FFmpeg (`libavcodec/format/util`,
`libswresample`, `libavfilter`):

- MP4/MKV (+ container from `-c`), H264/H265/AV1/VP8/VP9 + AAC/Opus,
  monotonic timestamps, VFR default, CFR/Content modes, `-keyint` seconds,
  `-pixfmt`, `-cr`, `-tune`, `-encoder gpu|cpu` + `-fallback-cpu-encoding`.
- Module: `muxer.rs`. Integration: 10s record with mocks + `ffprobe` checks
  (codec, duration, no timestamp regression).

## 5. Screenshot (MUST)

`src/image_writer.c`: `-o *.jpg/*.png` writes a single frame; `-sc` second
arg is `screenshot`. Module: `screenshot.rs`.

## 6. Plugins (MUST)

`src/plugins.c` + `plugin/plugin.h`: `-p` repeatable `.so` overlay.
Rust: `crates/plugin` trait + `examples/triangle`. Failure to load is a
localized error, never a silent skip.

## 7. Info commands (MUST)

`--info`, `--list-capture-options [card]`, `--list-audio-devices`,
`--list-application-audio` are machine-parsable (same item-per-line style
as C: `puts("focused")`, `puts("region")`, `puts("portal")`, monitor lines).
`--help` in EN/ES must be semantically identical (parity test diffs keys).

## 8. Acceptance

- `cargo test -p gsr-core` green with mocks (no GPU).
- `--help` EN/ES snapshot tests.
- Invalid-args tests exit 2 with localized message.
- Parity vs C: normalized `diff <(C --help) <(rs --help)` + `ffprobe` compare.
