Tenchu Native — Windows x64

Extract the archive and double-click tenchu-native.exe to choose your game CUE,
or run from PowerShell:
  .\tenchu-native.exe play 'C:\Games\Tenchu\game.cue'

The former tenchu-pc.exe is included for existing shortcuts and scripts.

The last verified disc is remembered. Double-click to reuse it next time; if it
is missing or invalid, the picker opens again. An explicit path takes priority.
F1 > General > Controls remaps keyboard/controller inputs and saves the bindings.
F1 > Vibration adjusts global and per-action controller strength (0-200%).

Opening/ending movies use the built-in decoder when FFmpeg is absent.
For video export or browser preparation, install Windows FFmpeg and add its
bin directory to PATH, or set:
  $env:TENCHU_FFMPEG = 'C:\Tools\ffmpeg\bin\ffmpeg.exe'
Windows builds are linked from https://ffmpeg.org/download.html.
An invalid explicit path reports an error. Remove the override or set
TENCHU_RUST_MOVIES=1 to use the built-in movie decoder.

Supply your own Shinobi Gaisen disc. Keep the CUE and referenced BIN files
together. Startup verifies MAIN.EXE, MENU.EXE, ENDING.EXE and DATA.VOL against
the original SLPS-019.01 SHA-256 hashes. Add --quick to open the development launcher. F1 opens PC settings;
F6 opens the development launcher from the original menu.

Download tenchu-hd.tar.zst from the artwork-v3 release and place it beside
the executable/launcher. It is found and cached automatically. F1 > Visual assets
controls it. TENCHU_ASSET_PACK also accepts an archive or unpacked directory;
an empty value disables pack loading.
Mixed textures are reconstructed from your disc in Rust; Python is not needed.
Saves/settings use %APPDATA%\tenchu-native, unless TENCHU_SAVE or
TENCHU_SETTINGS overrides the paths. No original disc assets or FFmpeg are bundled.
If that directory is absent, the existing %APPDATA%\tenchu-shinobi-gaisen-pc
directory is used for both reading and writing, including all campaign folders.

F1 > General > Benchmarks runs fixed performance scenes and can select an
Auto-optimised preset for your FPS target. Presets / save as offers Low, Medium,
High, Ultra and persistent named user presets. Fresh preferences use Modern Low.

build-info.json and --version identify this download. Keep licenses/ with the
program. The dependency-source download covers shared libraries in the Linux
bundle; it is not needed for this Windows download. This executable is not
code-signed; obtain it from the project's release page and check SHA256SUMS.

Controller troubleshooting: F1 > General > Controls > Controller connection and
input test switches between gilrs (default) and SDL without restarting. Windows
also offers XInput-only / WGI-only modes. Use a Steam Gamepad layout with extra
buttons mapped to keyboard keys; keep the standard gamepad outputs enabled.
Select the intended device, test/release each button, add connection/Steam notes,
then Save diagnostic report after comparing modes. See docs/controllers.md in
the source repository for the complete comparison checklist.
Keep the bundled SDL3.dll beside tenchu-native.exe. No separate SDL install is needed.
