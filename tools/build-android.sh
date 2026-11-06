#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
if [[ ${TENCHU_ANDROID_SHELL:-} != 1 ]]; then
    flavour=android
    if [[ ${1:-} == --emulator ]]; then flavour=android-emulator; shift; fi
    exec nix --extra-experimental-features 'nix-command flakes' develop ".#$flavour" --command bash tools/build-android.sh "$@"
fi
case "$CARGO_BUILD_TARGET" in
    aarch64-linux-android) abi=arm64-v8a ;;
    x86_64-linux-android) abi=x86_64 ;;
    *) printf 'Unsupported Android target: %s\n' "$CARGO_BUILD_TARGET" >&2; exit 1 ;;
esac
sdk_tools="$ANDROID_HOME/build-tools/35.0.0"
android_jar="$ANDROID_HOME/platforms/android-35/android.jar"
version=$(python3 -c 'import tomllib; print(tomllib.load(open("Cargo.toml", "rb"))["workspace"]["package"]["version"])')
work="target/android/$abi"
# These are this script's staging directories, never user assets.
if [[ -d "$work/apk" ]]; then chmod -R u+w "$work/apk"; fi
rm -rf -- "$work/classes" "$work/dex" "$work/apk"
mkdir -p "$work/classes" "$work/dex" "$work/apk/lib/$abi" "$work/apk/assets/licenses"
cargo rustc --locked --release -p tenchu-pc --lib --crate-type cdylib
cp "target/$CARGO_BUILD_TARGET/release/libtenchu_pc.so" "$work/apk/lib/$abi/"
javac -Xlint:-options -source 8 -target 8 -classpath "$android_jar" -d "$work/classes" android/java/org/tenchu/port/*.java
mapfile -t classes < <(rg --files "$work/classes" -g '*.class')
"$sdk_tools/d8" --min-api 28 --lib "$android_jar" --output "$work/dex" "${classes[@]}"
cp "$work/dex/classes.dex" "$work/apk/"
notices=$(nix build .#dependency-notices --no-link --print-out-paths)
cp "$notices/rust.html" LICENSE COPYING.md "$work/apk/assets/licenses/"
python3 - "$notices" "$work/apk/assets/licenses/rust-toolchain.txt" <<'PYTHON'
import pathlib, sys
root = pathlib.Path(sys.argv[1])
notices = {}
for path in sorted(root.glob("native/rustc-*/*")):
    if path.is_file():
        notices.setdefault(path.read_bytes(), []).append(path.name)
with open(sys.argv[2], "wb") as output:
    for text, names in notices.items():
        output.write(("\nFiles sharing this notice:\n" + "\n".join(names) + "\n\n").encode())
        output.write(text + b"\n")
PYTHON
for directory in crates/tenchu-assets/data/{japanese-font,mdec} crates/tenchu-pc/data/{hd-font,image-compat}; do
    name=${directory##*/}
    mkdir -p "$work/apk/assets/licenses/$name"
    for notice in "$directory"/LICENSE* "$directory"/OFL* "$directory"/README.md; do
        if [[ -f "$notice" ]]; then cp "$notice" "$work/apk/assets/licenses/$name/"; fi
    done
done
python3 - "$work" "$abi" <<'PYTHON'
import json, os, pathlib, subprocess, sys, tomllib, xml.etree.ElementTree as xml
work = pathlib.Path(sys.argv[1])
version = tomllib.loads(pathlib.Path("Cargo.toml").read_text())["workspace"]["package"]["version"]
major, minor, patch = map(int, version.split("-")[0].split("."))
code = major * 1_000_000 + minor * 1_000 + patch + 1
if minor >= 1000 or patch >= 1000 or not 1 <= code <= 2_100_000_000:
    raise SystemExit("Version is outside the Android versionCode range")
revision = os.environ.get("TENCHU_BUILD_REVISION")
if not revision:
    revision = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
    if subprocess.check_output(["git", "status", "--porcelain", "--untracked-files=no"], text=True).strip():
        revision += "-dirty"
name = version if os.environ.get("GITHUB_REF") == f"refs/tags/v{version}" else f"{version}-dev+{revision[:8]}"
locked = json.loads(pathlib.Path("flake.lock").read_text())
(work / "apk/assets/build-info.json").write_text(json.dumps({
    "version": version, "revision": revision,
    "nixpkgsRevision": locked["nodes"]["nixpkgs"]["locked"]["rev"],
    "target": "tenchu-android-" + {"arm64-v8a": "arm64", "x86_64": "x86_64"}[sys.argv[2]],
    "versionCode": code, "versionName": name,
}) + "\n")
xml.register_namespace("android", "http://schemas.android.com/apk/res/android")
manifest = xml.parse("android/AndroidManifest.xml")
if not os.environ.get("TENCHU_ANDROID_KEYSTORE"):
    manifest.getroot().find("application").set("{http://schemas.android.com/apk/res/android}debuggable", "true")
manifest.getroot().set("{http://schemas.android.com/apk/res/android}versionCode", str(code))
manifest.getroot().set("{http://schemas.android.com/apk/res/android}versionName", name)
manifest.write(work / "AndroidManifest.xml", encoding="utf-8", xml_declaration=True)
PYTHON
"$sdk_tools/aapt2" link -I "$android_jar" --manifest "$work/AndroidManifest.xml" \
    -o "$work/unsigned.apk"
(cd "$work/apk" && zip -q -r ../unsigned.apk lib classes.dex assets)
"$sdk_tools/zipalign" -f -P 16 4 "$work/unsigned.apk" "$work/aligned.apk"
keystore=${TENCHU_ANDROID_KEYSTORE:-private/android/debug.keystore}
if [[ ! -f "$keystore" ]]; then
    if [[ -n ${TENCHU_ANDROID_KEYSTORE:-} ]]; then printf 'Signing keystore does not exist: %s\n' "$keystore" >&2; exit 1; fi
    mkdir -p "$(dirname -- "$keystore")"
    (umask 077; keytool -genkeypair -keystore "$keystore" -alias androiddebugkey -storepass android -keypass android \
        -keyalg RSA -keysize 2048 -validity 10000 -dname 'CN=Tenchu local development')
fi
apk="$work/tenchu-$version-$abi.apk"
if [[ -n ${TENCHU_ANDROID_KEYSTORE:-} ]]; then
    passwords=()
    if [[ -n ${TENCHU_ANDROID_KEYSTORE_PASSWORD:-} ]]; then
        passwords=(--ks-pass env:TENCHU_ANDROID_KEYSTORE_PASSWORD --key-pass env:TENCHU_ANDROID_KEYSTORE_PASSWORD)
    fi
    "$sdk_tools/apksigner" sign --ks "$keystore" "${passwords[@]}" --out "$apk" "$work/aligned.apk"
else
    "$sdk_tools/apksigner" sign --ks "$keystore" --ks-key-alias androiddebugkey --ks-pass pass:android --key-pass pass:android --out "$apk" "$work/aligned.apk"
fi
"$sdk_tools/apksigner" verify --print-certs "$apk" > "$work/signature.txt"
"$sdk_tools/zipalign" -c -P 16 4 "$apk"
if [[ ${TENCHU_ANDROID_RELEASE_KEY:-} == 1 ]]; then
    expected=$(cat android/signing-certificate.sha256)
    if ! rg -q -F "Signer #1 certificate SHA-256 digest: $expected" "$work/signature.txt"; then
        printf 'APK was not signed by the release key\n' >&2
        exit 1
    fi
fi
printf '\nAPK: %s\nInstall: adb install -r %q\n' "$apk" "$apk"
