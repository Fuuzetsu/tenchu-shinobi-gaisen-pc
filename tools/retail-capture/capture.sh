#!/usr/bin/env bash
set -euo pipefail
(( $# == 4 || $# == 5 )) || { echo 'Usage: capture.sh ARCHIVE STATE|--boot SEGMENTS NEW_OUTPUT_DIRECTORY [ITEM_SETUP.json]' >&2; exit 2; }
script=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo=$(cd -- "$script/../.." && pwd)
archive=$(realpath -- "$1")
state=$2
if [[ $state != --boot ]]; then state=$(realpath -- "$state"); fi
plan=$(realpath -- "$3")
out=$(realpath -m -- "$4")
[[ ! -e $out && ! -e $out.runtime && -s $plan ]] || { echo 'Missing input or output already exists' >&2; exit 2; }
if [[ ${TENCHU_RETAIL_CAPTURE_TOOLS:-0} != 1 ]]; then
    pkgs=$(jq -r .nixpkgs "$archive/dependencies.json")
    exec nix --extra-experimental-features 'nix-command flakes' shell \
        "$pkgs#mono" "$pkgs#jq" "$pkgs#ripgrep" "$pkgs#xvfb-run" "$pkgs#python3" \
        -c env TENCHU_RETAIL_CAPTURE_TOOLS=1 bash "$0" "$@"
fi
mkdir -p "$out.runtime"
cp "$script/probe.rs" "$script/PhaseProbe.cs" "$script/capture.sh" "$out.runtime/"
"$repo/dev.sh" rustc --edition=2024 --crate-type cdylib -O "$out.runtime/probe.rs" -o "$out.runtime/libtenchu_retail_probe.so"
runtime=$archive/runtime
asms=${ONIKAGE_BIZHAWK_ASMS:-/nix/store/l4jcq3far00z6b90v16x43zib8r39v9z-bizhawk-asms-and-wbox}
refs=(-r:System.Web.Extensions -r:System.Windows.Forms -r:System.Drawing -r:"$asms/EmuHawk.exe")
for dll in "$asms"/dll/BizHawk*.dll; do refs+=(-r:"$dll"); done
python3 - "$runtime/tools/tenchu-worker/SimCadenceProbe.cs" "$out.runtime/CadenceProbe.cs" <<'PY'
from pathlib import Path
import sys
source = Path(sys.argv[1]).read_text()
old = 'if(!string.IsNullOrEmpty(Environment.GetEnvironmentVariable("ONIKAGE_SIM_PHASE_FIELDS")))'
assert source.count(old) == 1, 'Unsupported archived cadence observer'
# Register the new cross-executable observer independently of the original
# short MAIN-only phase probe. Keep its core hash and memory-layout checks.
Path(sys.argv[2]).write_text(source.replace(old, 'if(Environment.GetEnvironmentVariable("TENCHU_RETAIL_OBSERVER") == "1")'))
PY
mcs -sdk:4.8 -langversion:7.2 -target:library -optimize+ -out:"$out.runtime/TenchuScenarioSetup.dll" "${refs[@]}" \
    "$runtime/tools/tenchu-worker/ScenarioSetup.cs" "$runtime/tools/tenchu-worker/ItemSetup.cs" \
    "$runtime/tools/tenchu-worker/CampaignAudit.cs" "$out.runtime/CadenceProbe.cs" "$out.runtime/PhaseProbe.cs"
sha256sum "$out.runtime/"*.rs "$out.runtime/"*.cs "$out.runtime/"*.sh "$out.runtime/"*.so "$out.runtime/"*.dll > "$out.runtime/SHA256SUMS"
unset TENCHU_SETUP_LAYOUT TENCHU_SETUP_PREFIX TENCHU_SETUP_ITEMS TENCHU_SETUP_MISSION_STAGE
unset TENCHU_SETUP_CAPTURE_CLOCK_RESTORE TENCHU_SETUP_STOP_OBSERVATION TENCHU_SETUP_ENTRY_CVA
if (( $# == 5 )); then
    export TENCHU_SETUP_ITEMS
    TENCHU_SETUP_ITEMS=$(realpath -- "$5")
fi
unset ONIKAGE_SIM_PHASE_FIELDS ONIKAGE_SIM_PHASE_ACTORS ONIKAGE_SIM_BOUNDARY_SNAPSHOTS ONIKAGE_VIEWER_WINDOW ONIKAGE_VIEWER_OBSERVER ONIKAGE_WORKER_IDENTITY_PATH
export TENCHU_SETUP_STATE=$state TENCHU_SETUP_PLAN=$plan TENCHU_SETUP_OUTPUT=$out TENCHU_SETUP_COLD_BOOT=0
if [[ $state == --boot ]]; then TENCHU_SETUP_STATE=; export TENCHU_SETUP_COLD_BOOT=1; fi
export TENCHU_SETUP_MENU_ONLY=1 TENCHU_SETUP_CAMPAIGN_AUDIT=1 TENCHU_SETUP_SCREENSHOT_EVERY=1000000
export ONIKAGE_EXTERNAL_TOOL_LIBRARY=$runtime/tools/tenchu-worker/target/release/libtenchu_worker.so
export ONIKAGE_SEARCH_BIN=$runtime/bin/tenchu-search
if [[ ! -x $ONIKAGE_SEARCH_BIN && -x $archive/bin/tenchu-search ]]; then
    ONIKAGE_SEARCH_BIN=$archive/bin/tenchu-search
fi
export BIZHAWK_RUN_DIR=$out.runtime BIZHAWK_KEEP_RUN=1 BIZHAWK_TIMEOUT=${BIZHAWK_TIMEOUT:-3600}
export BIZHAWK_EXPECT_MARKER='[scenario-setup] PASS'
export BIZHAWK
BIZHAWK=$(jq -r .local_dependencies.bizhawk_launcher.path "$archive/dependencies.json")
export GAISEN_CUE
GAISEN_CUE=$(jq -r .local_dependencies.cue.path "$archive/dependencies.json")
export ONIKAGE_SIM_CPU_PROBE_WBX=$asms/dll/shock.wbx.zst TENCHU_RETAIL_OBSERVER=1
export LD_LIBRARY_PATH="$out.runtime${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
args=("$out.runtime/TenchuScenarioSetup.dll")
if [[ $state != --boot ]]; then args+=("$state"); fi
bash "$runtime/tools/bizhawk/run-onikage.sh" "${args[@]}"
