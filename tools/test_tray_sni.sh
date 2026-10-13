#!/usr/bin/env bash
# SourisGSR tray test: proves sgsr-ui registers a StatusNotifierItem with a
# working GDBus menu (Show/Record/Replay/Save replay/Quit), using dbus-send.
# Usage: ./tools/test_tray_sni.sh [--with-quit]
# Prints SKIP (exit 0) without session bus / Watcher — never false-green.
set -u
WITH_QUIT=0
[ "${1:-}" = "--with-quit" ] && WITH_QUIT=1

need() { command -v "$1" >/dev/null 2>&1 || { echo "SKIP: missing $1"; exit 0; }; }
need dbus-send
[ -n "${DBUS_SESSION_BUS_ADDRESS:-}" ] || { echo "SKIP: no session bus"; exit 0; }

WATCHER="org.kde.StatusNotifierWatcher"
has_owner() {
  dbus-send --session --print-reply --dest=org.freedesktop.DBus \
    /org/freedesktop/DBus org.freedesktop.DBus.NameHasOwner "string:$WATCHER" 2>/dev/null \
    | grep -q "boolean true"
}
has_owner || { echo "SKIP: no StatusNotifierWatcher (no tray host)"; exit 0; }

ITEM="$(dbus-send --session --print-reply --dest=org.freedesktop.DBus \
  /org/freedesktop/DBus org.freedesktop.DBus.ListNames 2>/dev/null \
  | grep -o "org.kde.StatusNotifierItem-[0-9]*" | head -1)"
[ -n "$ITEM" ] || { echo "FAIL: no sgsr tray item on the bus (is sgsr-ui running with tray enabled?)"; exit 1; }
echo "item: $ITEM"

# Watcher lists items by unique connection name (":N.M/StatusNotifierItem"),
# not by well-known name. Find ours by asking each for its SNI IconName.
registered() {
  local items
  items="$(dbus-send --session --print-reply --dest="$WATCHER" \
    /StatusNotifierWatcher org.freedesktop.DBus.Properties.Get \
    string:org.kde.StatusNotifierWatcher string:RegisteredStatusNotifierItems 2>/dev/null \
    | grep -o '":[^"]*/StatusNotifierItem"' | tr -d '"')"
  for it in $items; do
    # Watcher lists ":N.M/StatusNotifierItem"; the dbus destination is the unique
    # name part only (":N.M"), the object path stays "/StatusNotifierItem".
    local unique="${it%%/*}"
    local p
    p="$(dbus-send --session --print-reply --dest="$unique" \
      /StatusNotifierItem org.freedesktop.DBus.Properties.Get \
      string:org.kde.StatusNotifierItem string:IconName 2>/dev/null)"
    echo "$p" | grep -q 'string "sourisgsr"' && return 0
  done
  return 1
}
registered || { echo "FAIL: no sgsr item with IconName=sourisgsr in RegisteredStatusNotifierItems"; exit 1; }
echo "ok: registered with watcher (IconName match)"

props() {
  dbus-send --session --print-reply --dest="$ITEM" \
    /StatusNotifierItem org.freedesktop.DBus.Properties.GetAll string:org.kde.StatusNotifierItem 2>/dev/null
}
P="$(props)"
echo "$P" | grep -q '"IconName"' && echo "$P" | grep -q 'string "sourisgsr"' \
  || { echo "FAIL: IconName prop"; exit 1; }
echo "$P" | grep -q '"Status"' && echo "$P" | grep -q 'string "Active"' \
  || { echo "FAIL: Status prop"; exit 1; }
echo "ok: props IconName/Status"

# GDBus menu: GetLayout returns (revision:u, layout:(i, props, children)).
# Labels follow the user's locale; check the Spanish set by default and fall
# back to English if the UI language differs.
L="$(dbus-send --session --print-reply --dest="$ITEM" \
  /Menu com.canonical.dbusmenu.GetLayout int32:0 uint32:-1 array:string: 2>/dev/null)"
menu_has() { echo "$L" | grep -q "string \"$1\""; }
if menu_has "Mostrar"; then
  for label in "Mostrar" "Grabar" "Replay" "Guardar replay" "Salir"; do
    menu_has "$label" || { echo "FAIL: menu missing '$label'"; exit 1; }
  done
else
  for label in "Show UI" "Record" "Replay" "Save replay" "Quit"; do
    menu_has "$label" || { echo "FAIL: menu missing '$label'"; exit 1; }
  done
fi
echo "ok: menu has 5 items"

dbus-send --session --dest="$ITEM" \
  /StatusNotifierItem org.kde.StatusNotifierItem.Activate int32:0 int32:0 >/dev/null 2>&1
sleep 1
if pgrep -f "sgsr-ui$" >/dev/null; then echo "ok: Activate sent, sgsr-ui alive"; else echo "FAIL: sgsr-ui died after Activate"; exit 1; fi

if [ "$WITH_QUIT" = 1 ]; then
  PID="$(pgrep -f "sgsr-ui$" | head -1)"
  # Menu item id 5 = "Quit" (com.canonical.dbusmenu.Event with "clicked").
  dbus-send --session --dest="$ITEM" \
    /Menu com.canonical.dbusmenu.Event int32:5 string:"clicked" variant:int32:0 uint32:0 >/dev/null 2>&1
  sleep 3
  if kill -0 "$PID" 2>/dev/null; then echo "FAIL: menu Quit did not stop sgsr-ui"; exit 1; fi
  echo "ok: menu Quit stopped sgsr-ui"
fi
echo "TRAY SNI PASSED"
