# Parity — gsr-rs vs GSR C v5.10.2

## Method

- CLI: normalized `diff <(gsr-c --help) <(gsr-rs --help)>` per locale.
  Normalization ignores binary name and version string only.
- Behavior: same exit codes (0 ok, 2 usage error), stdout = saved path,
  stderr = logs, identical `--list-*` line format.
- Media: `ffprobe` codec/duration/pix-fmt compare on 10s fixtures.

## Scope notes

- X11-only inputs are parity-by-rejection: both help texts document them,
  the Rust binary returns localized `err_unsupported_x11`.
- `DEVIATION-FUTURE` names (`-ipc`, V4L2, `_vulkan` aliases, `-auth`) are
  reserved and rejected with `err_unsupported_future` until v2.
