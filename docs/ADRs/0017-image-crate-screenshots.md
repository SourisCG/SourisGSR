# ADR-0017 — `image` crate for screenshots

Status: accepted (user-approved dependency).

## Context

C writes JPEG/PNG via bundled `stb_image_write.h` from RGBA8 pixels
(JPEG quality clamped 1–100, PNG ignoring quality, no HDR/10-bit).

## Decision

Pure-Rust `image` crate (`png` + `jpeg` features only). RGBA→RGB drop for
JPEG mirrors what stb does internally. Same formats, same quality clamp,
same TODO on HDR.

## Consequences

- PNG roundtrip is byte-exact (tested); JPEG keeps dimensions (tested).
- Callers display localized `err_screenshot_write`; raw `io::Error`
  details stay diagnostic (allowlisted with reason).
