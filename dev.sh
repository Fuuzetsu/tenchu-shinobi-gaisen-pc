#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
if [[ ${TENCHU_DEV_SHELL:-} == 1 ]] && command -v cargo >/dev/null; then
    exec "$@"
fi
exec nix --extra-experimental-features 'nix-command flakes' develop --command "$@"
