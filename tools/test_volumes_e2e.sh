#!/usr/bin/env bash
# SourisGSR E2E volume test: proves per-track vol= changes recorded levels.
# Sine 1kHz -> null-sink -> record 3 takes (vol 100/50/25) -> compare RMS.
# Expect ~-6dB per halving (tolerance 4-8dB for opus + timing jitter).
# Needs: ffmpeg, pactl, paplay, sgsr, running Pulse/PipeWire.
# Prints SKIP (exit 0) when no audio server is available — never false-green.
set -u
GSR_BIN="${GSR_BIN:-sgsr}"
SINK="sgsr_e2e"
WORK="$(mktemp -d /tmp/sgsr-e2e.XXXXXX)"
trap 'rm -rf "$WORK"; pactl unload-module "$MOD" >/dev/null 2>&1 || true; pkill -f "paplay.*sgsr_sine" 2>/dev/null || true' EXIT

need() { command -v "$1" >/dev/null 2>&1 || { echo "SKIP: missing $1"; exit 0; }; }
need ffmpeg
need pactl
need paplay
need "$GSR_BIN"
pactl info >/dev/null 2>&1 || { echo "SKIP: no audio server"; exit 0; }

ffmpeg -v error -y -f lavfi -i "sine=frequency=1000:duration=8:sample_rate=48000" -ac 2 "$WORK/sgsr_sine.wav" \
  || { echo "FAIL: cannot generate sine"; exit 1; }

MOD="$(pactl load-module module-null-sink sink_name="$SINK" sink_properties=device.description=sgsr-e2e)" \
  || { echo "FAIL: cannot load null sink"; exit 1; }
sleep 2
MON="$SINK.monitor"
for i in $(seq 1 10); do
  pactl list short sources | grep -q "$MON" && break
  sleep 0.5
done
pactl list short sources | grep -q "$MON" || { echo "FAIL: monitor $MON missing"; exit 1; }

rms_of() {
  ffmpeg -v info -i "$1" -af astats=metadata=1:reset=0 -f null - 2>&1 \
    | grep -m1 "RMS level dB" | grep -oE "\-?[0-9]+\.[0-9]+" | head -1
}

take_with_retry() {
  local vol="$1" out="$2" attempt
  for attempt in 1 2 3; do
    paplay --device="$SINK" "$WORK/sgsr_sine.wav" >/dev/null 2>&1 &
    local player_pid=$!
    sleep 1
    timeout 8 "$GSR_BIN" -w portal -f 30 -a "$MON|vol=$vol" -c mp4 -o "$out" >/dev/null 2>&1
    kill "$player_pid" 2>/dev/null || true
    wait "$player_pid" 2>/dev/null || true
    local rms
    rms="$(rms_of "$out")"
    if [ -s "$out" ] && [ -n "$rms" ]; then
      echo "$rms"
      return 0
    fi
    echo "  retry $attempt for vol=$vol (file/out: $([ -s "$out" ] && echo yes || echo no))" >&2
    sleep 1
  done
  return 1
}

declare -A RMS
for VOL in 100 50 25; do
  OUT="$WORK/take$VOL.mp4"
  RMS[$VOL]="$(take_with_retry "$VOL" "$OUT")" \
    || { echo "FAIL: take vol=$VOL produced no usable file after retries"; exit 1; }
  echo "vol=$VOL RMS=${RMS[$VOL]} dB"
done

cmp() { python3 -c "import sys; d=$1-($2); sys.exit(0 if 4.0 <= d <= 8.0 else 1)"; }
cmp "${RMS[100]}" "${RMS[50]}" \
  || { echo "FAIL: 100->50 drop out of 4-8dB (got ${RMS[100]} -> ${RMS[50]})"; exit 1; }
cmp "${RMS[50]}" "${RMS[25]}" \
  || { echo "FAIL: 50->25 drop out of 4-8dB (got ${RMS[50]} -> ${RMS[25]})"; exit 1; }
echo "E2E VOLUMES PASSED: 100=${RMS[100]}dB 50=${RMS[50]}dB 25=${RMS[25]}dB"
