# ADR-0011 — Overlong `-w` rejected, not truncated (`DEVIATION-WINDOW-LEN`)

Status: accepted.

## Context

C copies `-w` with `snprintf` into `char window[64]`, silently truncating
monitor names longer than 63 bytes.

## Decision

Values over 63 bytes fail with a localized error. Silent truncation of
an identifier the backends must match is a bug farm; failing loud is
safer and tested.

## Consequences

- One more reserved error key (`err_window_too_long`); behavior on all
  valid inputs is identical.
