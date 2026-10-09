# Changelog — SourisGSR

Format: one entry per commit, newest last per release section (Keep-a-Changelog
spirit, without version numbers until the first release).

## Unreleased

### Specs and skeleton

- Reference mirror cloned (Gnu1, v5.10.2) and verified; `reference/MAPA.md`.
- Constitution, specs 01–06, plan and tasks materialized.
- Workspace skeleton: 7 domain crates + structure-mapping test + EN/ES
  locales + systemd/udev extras + CI skeleton.

### Core (all gates green, 95+ tests)

- i18n catalog API (`Lang`, `resolve_lang`, `Catalog`, `fill`) (T13a).
- CLI parser + validated config with C parity, `--help` EN/ES snapshots,
  exit 1 on parse errors (T10).
- Replay ring buffer RAM/disk with C parity (T11).
- Muxer timing engine + screenshots via `image`, `ffprobe` harness proven
  (ignored until T40) (T12).
- All user strings localized, `no_hardcode` lint + allowlist, coverage
  90.49% overall / 86.8% core-src / 89% i18n-src (T13b).
- Plugin C ABI with real `.so` load/draw/unload tests (T14).

### Linux capture

- KMS helper protocol (packed-LE v5), `SCM_RIGHTS` transport, DRM grab via
  `drm` 0.15, client with spawn modes incl. `pkexec`, fake-helper
  full-dance test (T20). HW serve path is manual-QA only.

### Docs (this phase, one commit per document)

- `AGENTS.md` working agreement; `README.md` rewrite with credits + GSR link.
- `CONTRIBUTING.md` with SDD workflow and gates.
- 18 ADRs, one per deviation (`docs/ADRs/`).
- Guides: `build.md`, `testing.md`, `security.md`, `i18n.md`,
  `qa-manual.md`; expanded `architecture.md`, fixed `parity.md`.
