# Grant the KMS helper its capability at install time.
# Mirrors reference/gpu-screen-recorder/extra/meson_post_install.sh (v5.10.2).
# Removes the password prompt when recording a monitor without the portal
# option on AMD/Intel or NVIDIA Wayland.
#
# Run as root after installing the binaries:
set -u
PREFIX="${MESON_INSTALL_DESTDIR_PREFIX:-/usr/local}"
/usr/sbin/setcap cap_sys_admin+ep "${PREFIX}/bin/gsr-kms-server" \
    || echo "Please re-run install as root"
