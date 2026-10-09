# Manual QA checklist (hardware only)

CI proves protocol, transport, parsing and mocks. These checklists prove
reality. Record date, machine, GPU and driver with each run.

## KMS helper dance (T20)

- [ ] On a card you can open with caps (`sudo setcap cap_sys_admin+ep`
      on the just-built `gsr-kms-server`, or root shell): run the client
      dance manually — expect `KmsResult::Ok`, `items >= 1`, and
      `fds.len() == sum(num_dma_bufs)` on `GET_KMS`.
- [ ] Without caps: expect the localized permission hint, exit path clean
      (no orphaned `gsr-kms-server`, no socket litter in `$XDG_RUNTIME_DIR`).
- [ ] `pkexec` path with an auth agent: password prompt appears once,
      dance completes, helper reaped on client exit (`ps` shows nothing).
- [ ] Monitor hotplug during capture (T21+): 2 s refresh picks it up.

## Portal (T22, when implemented)

- [ ] GNOME + KDE/Wayland: session popup, `portal` appears in
      `--list-capture-options`, restore-token file created and reused on
      second run (`-restore-portal-session yes` skips the popup).
- [ ] Cursor embedded vs hidden honors `-cursor`; crop/rotation sane on a
      rotated monitor.

## Performance (T50)

- [ ] 1080p144 criterion bench on target hardware, no frame-time
      regressions vs the C build at the same settings.
- [ ] 30 min soak: RSS flat, no fd growth (`/proc/<pid>/fd` count),
      replay save at the end opens and plays.
- [ ] `ffprobe` parity on a 10 s capture vs the C binary (codec,
      duration, timestamps).

## Release (T51)

- [ ] `extra/systemd` unit starts replay at login; env file honored.
- [ ] `extra/udev` setcap + modprobe conf survive reboot; suspend/resume
      keeps CUDA alive (NVIDIA).
- [ ] Flatpak: host proxy path works, portal token persists.
