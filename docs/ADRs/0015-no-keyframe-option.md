# ADR-0015 — Missing keyframe returns `None` (`DEVIATION-NO-KEYFRAME`)

Status: accepted.

## Context

C signals "no keyframe" with the `{(size_t)-1, 0}` sentinel (unsigned
wraparound), which callers must remember to check.

## Decision

`find_keyframe` returns `Option<Cursor>`; `plan_save` returns `None` and
the caller reports localized `err_replay_no_keyframe` and aborts the
save, exactly like the C error path. Audio tracks without a keyframe
still fall back to the video start with pts offset 0, mirroring C.

## Consequences

- No sentinel bugs possible; the abort path is unit-tested.
