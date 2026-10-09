# Parity — SourisGSR vs GSR C v5.10.2

## Method

- CLI: normalized `diff <(gsr-c --help) <(gsr-rs --help)>` per locale.
  Normalization ignores binary name and version string only.
- Behavior: same exit codes (0 ok incl. `--help`, 1 usage error),
  stdout = saved path, stderr = logs, identical `--list-*` line format.
  (The spec once said 2; the C `_exit(1)` won — see tasks history.)
- Media: `ffprobe` codec/duration/pix-fmt compare on 10s fixtures
  (harness proven, ignored until T40 encoder output exists).
- Help snapshots freeze both locales; regenerate only with
  `UPDATE_SNAPSHOTS=1` plus a Spanish review.

## Scope notes

- X11-only inputs are parity-by-rejection: both help texts document them,
  the Rust binary returns localized `err_unsupported_x11`.
- `DEVIATION-FUTURE` names (`-ipc`, V4L2, `_vulkan` aliases, `-auth`) are
  reserved and rejected with `err_unsupported_future` until v2.
- Known cosmetic diffs: the `-keyint` typo fix (ADR-0010) and translated
  prose. Normalize those two lines before diffing.

