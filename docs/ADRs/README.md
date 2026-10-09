# ADRs — Architecture Decision Records

One file per accepted deviation from the C original (`DEVIATION:`).
Format: Status / Context / Decision / Consequences. All in English.

| ID | Deviation | Subject |
|----|-----------|---------|
| 0001 | `DEVIATION-X11-NEVER` | No X11 support, permanently |
| 0002 | `DEVIATION-FUTURE` | v2 scope for features absent from v5.10.2 |
| 0003 | `DEVIATION-WIRE` | Packed little-endian KMS wire protocol |
| 0004 | `DEVIATION-SEC`, `DEVIATION-SEC-PATH` | Helper socket hardening |
| 0005 | — | `pkexec` fallback kept |
| 0006 | `DEVIATION-HELPER-LOGS` | Helper diagnostics stay English |
| 0007 | `DEVIATION-UTC-TIME` | Recording filenames use UTC |
| 0008 | `DEVIATION-HELP-EXIT` | `--help` exits 0 |
| 0009 | `DEVIATION-STRICT-INT` | Strict integer parsing |
| 0010 | `DEVIATION-TYPO` | Fixed C message typo |
| 0011 | `DEVIATION-WINDOW-LEN` | Overlong `-w` rejected, not truncated |
| 0012 | `DEVIATION-ADD` | `--lang` flag |
| 0013 | `DEVIATION-SNAPSHOT` | Owned replay save snapshots |
| 0014 | `DEVIATION-NOW-PARAM` | Injected clock in replay search |
| 0015 | `DEVIATION-NO-KEYFRAME` | Missing keyframe returns `None` |
| 0016 | `DEVIATION-SPAWN` | Detached script spawn |
| 0017 | — | `image` crate for screenshots |
| 0018 | — | Boxed `Config` in the `Action` enum |
