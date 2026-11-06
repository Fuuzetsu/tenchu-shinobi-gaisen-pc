# Android preview

The native port targets **64-bit ARM phones/tablets running Android 9 or newer**.
It uses the same Rust game logic and renderer as desktop, with a touch controller
and Android file pickers. APKs are published alongside the desktop downloads; this is not a Google Play release.

## Download and install

Download **`tenchu-android-arm64.apk`** from the game's
[Releases](https://github.com/Fuuzetsu/tenchu-shinobi-gaisen-pc/releases), open it
on your phone and allow installation from that source when Android asks. Update
by installing the newer APK over the old one; keep the app installed to retain
its imported disc, artwork and saves. The HEAD download follows `master`.
The launcher then imports your disc as described below.

## Build and install from source

On x86-64 Linux with Nix:

```sh
./tools/build-android.sh
adb install -r target/android/arm64-v8a/tenchu-0.15.6-arm64-v8a.apk
```

The script prints the actual versioned filename. You can also copy the APK to the
phone and open it there. The pinned flake supplies Rust, Android SDK 35, NDK r27,
Java and packaging tools; the first build downloads these dependencies. Android
SDK license acceptance is scoped to the Android development shells.

Local builds use a development signing key in `private/android/debug.keystore`.
Keep that key to install updates over your own builds. It is never committed.
`TENCHU_ANDROID_KEYSTORE=/path/to/keystore` selects a separate signing key and
asks for its password; automation can supply `TENCHU_ANDROID_KEYSTORE_PASSWORD`.
Local builds signed with a different key cannot replace the official APK in place. APKs contain the program and license notices, not game
assets. Native libraries are aligned for 16 KiB memory pages.

For an x86-64 Android emulator, use `./tools/build-android.sh --emulator`.
Cross-target linting uses `nix develop .#android --command cargo clippy
--lib -p tenchu-pc -- -D warnings` (or `.#android-emulator`).

## Supply the disc and optional artwork

1. Copy your supported **Shinobi Gaisen (SLPS-019.01)** CUE and its BIN tracks into
   one folder on the phone, for example `Download/Tenchu`.
2. Open Tenchu and choose **Import disc folder…**, select that folder, and allow
   access. The app copies the files into its own storage and verifies the game.
   Firefox's comma-to-underscore filename changes and case differences are
   handled automatically. For other renamed tracks, choose the corresponding
   BIN when prompted. Android restricts selecting the Downloads root itself,
   so use a subfolder. Import needs roughly 750 MB of free space.
3. Optionally choose **Import HD artwork (.tar.zst)…** and select the existing
   [HD pack](https://github.com/Fuuzetsu/tenchu-shinobi-gaisen-pc/releases/tag/artwork-v3).
   It is validated and unpacked during import. Allow about 450 MB more space.
4. Tap **Play**. Subsequent launches reuse the imported disc and artwork.

Original files remain untouched. Preferences, campaign progress, custom presets,
TRIAL missions and imports live in Android's app storage; uninstalling the app
removes them. The usual F1 diagnostic/rewind/TRIAL/seed exports use Android's
Save dialog so you can put those files outside app storage.

## Touch controls

- **Move** is the left stick. Hold it while pressing actions with other fingers.
- **Attack, Jump, Crouch, Item, Look, R1, Item −/+, Map** are the original
  controller inputs. They retain the game's selected original controller layout.
- **Settings** opens/closes F1; **Pause** pauses; **Back** opens the inventory or
  backs out of a menu. Android's Back action does the same.
- In retail menus, **×** confirms and **○** cancels by default; F1's confirmation
  preference can reverse them. **Start** begins a mission from the shop.
  **Quick** opens the fast mission launcher.
- Turn on **Camera-relative controls** under Settings → Movement to add a right
  camera stick. **Center** recenters it. Classic controls remain available.
- Settings → General → **Touch controls…** changes size, opacity, handedness and
  visibility. The Settings/Pause/Back strip remains available when hidden.
- Settings → Player model → **Choose player model…** opens the searchable model
  grid in its own panel on small screens.
- Tap menu controls directly and swipe lists to scroll. Text fields open a native
  Android editor with the software keyboard and normal selection/copy/paste;
  **Apply** returns the edited value to the game. In the free camera, the sticks
  move/look; Rise, Descend and Boost control flight, Center finds the player and
  Back returns to gameplay. Tapping the world supports selection/teleport tools.

Graphics changes that need new GPU shaders show **Preparing graphics**. The game
pauses while those shaders compile; Android input, Home and background/resume
remain responsive. Repeating an unchanged size does not rebuild the effects.

Start with **Modern Low** or Original, then increase graphics as your phone allows.
HD artwork does not require the expensive lighting/weather presets. Desktop,
browser and Android preferences are separate; installing the APK does not alter
settings on the computer.

## Verification and current limits

Tested in an Android 15 x86-64 emulator at phone-sized UI scaling: disc and HD
imports through Android's document picker, opening movie, shop, Mission 1 with
both heroes, independent two-finger movement/jump, settings scrolling, native
text entry and saved presets, model-grid selection, diagnostic export, paused
free-camera movement, and returning
to the same session after Home/Play. The portable movie decoder also passed full
disc checks for all five movie streams and comparisons against desktop decoding.

The ARM64 APK is cross-built, signed and checked for 16 KiB alignment; it still
needs testing on physical phones. Emulator performance does not predict phone
frame rates, battery use or driver compatibility. Android's audio output stream
was exercised, but speaker/headphone output needs a real-device check.

Touch controls are the supported input for this preview. Bluetooth/USB gamepad
input and vibration are not wired to Android yet. Built-in model selection works
through F1, but importing custom model folders through Android's document picker
is not implemented. Campaign saves persist; Android may terminate a background
app, so an unsaved mid-mission session is not guaranteed to survive that.

The touch controller follows the familiar independent-stick/button approach used
by mobile game ports, including [Dragon-Burst](https://github.com/dfdx047/Dragon-Burst),
which was reviewed as a design reference. No code or artwork was copied from it.
