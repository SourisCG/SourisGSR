# Spec-06 — I18N (EN/ES) + reserved future surface

Refs: user language contract (ES with user, EN specs/code), `--help` and
error strings in `src/args_parser.c` + `src/main.cpp` (v5.10.2);
`DEVIATION-FUTURE` names come from newer upstream (absent here).

## 1. Rules (MUST)

- Keys live only in `crates/i18n/locales/{en,es}.toml`; sets must be
  identical (enforced by `structure.rs` + `fallback.rs`).
- Runtime selection: `--lang en|es|auto` (default `auto`), else `LANG`.
  Default language is English. `--help`, logs, and every error are 100%
  covered in both locales.
- Zero hardcoded user-facing strings anywhere else: a `no_hardcode` CI
  check fails on user-facing literals outside `crates/i18n`.
- Missing key falls back to English and logs once (tested).

## 2. Reserved v2 namespace (MUST NOT implement in v1)

`-ipc`, V4L2 camera paths, `_vulkan`-suffixed `-k` aliases, `-auth`/WHIP,
`-ffmpeg-opts` family, `-low-power`, `-exclude-metadata`,
`-write-first-frame-ts`, `--list-monitors`/`--list-v4l2-devices`:
parsers reserve these spellings and return localized
`err_unsupported_future`. No silent acceptance, no partial behavior.

## 3. Tests (MUST)

- `fallback.rs`: every EN key resolves in ES.
- `structure.rs::locales_have_identical_keys`: key-set equality.
- `--help` EN/ES snapshot tests (T10): same options, same order, same
  defaults, translated prose.
