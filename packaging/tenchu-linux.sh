#!/bin/sh
set -eu
bundle=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)

export XKB_CONFIG_ROOT="${XKB_CONFIG_ROOT:-$bundle/share/X11/xkb}"
export ALSA_CONFIG_PATH="${ALSA_CONFIG_PATH:-$bundle/share/alsa/alsa.conf}"
export ALSA_CONFIG_DIR="${ALSA_CONFIG_DIR:-$bundle/share/alsa}"
export XDG_DATA_DIRS="${XDG_DATA_DIRS:-/usr/local/share:/usr/share}:/run/opengl-driver/share"
export __EGL_VENDOR_LIBRARY_DIRS="${__EGL_VENDOR_LIBRARY_DIRS:-/etc/glvnd/egl_vendor.d:/usr/share/glvnd/egl_vendor.d:/run/opengl-driver/share/glvnd/egl_vendor.d}"

# Keep the bundled libc out of the environment inherited by external FFmpeg.
exec "$bundle/lib/@loader@" \
    --library-path "$bundle/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}:@hostLibraries@" \
    "$bundle/libexec/tenchu-pc" "$@"
