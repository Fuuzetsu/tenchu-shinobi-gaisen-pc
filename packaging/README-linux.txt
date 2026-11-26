Tenchu Native — Linux

Extract the whole archive and run ./tenchu-native to choose your game CUE, or:
  ./tenchu-native play '/path/to/game.cue'

The former tenchu-pc launcher is included for existing shortcuts and scripts.

The last verified disc is remembered. Run without a path to reuse it next time;
if it is missing or invalid, the picker opens again. An explicit path takes priority.
F1 > General > Controls remaps keyboard/controller inputs and saves the bindings.
F1 > Vibration adjusts global and per-action controller strength (0-200%).

This archive includes the program's shared libraries and dynamic loader. Nix
is not required. Keep lib/, libexec/ and share/ beside the tenchu-native launcher;
do not launch libexec/tenchu-native directly. Use the archive matching your CPU:
x86_64 for ordinary Intel/AMD PCs, aarch64 for 64-bit ARM Linux.

An installed Vulkan or OpenGL GPU driver, a graphical desktop and working
desktop audio are required for gameplay. GPU drivers are supplied by your OS.
Opening/ending movies use the built-in decoder when FFmpeg is absent.
For video export or browser preparation, install FFmpeg through your distribution
or set TENCHU_FFMPEG to its executable. FFmpeg is not bundled. An invalid explicit
path reports an error; remove it or set TENCHU_RUST_MOVIES=1 for built-in playback.

Supply your own Shinobi Gaisen disc. Keep the CUE and referenced BIN files
together. Startup verifies MAIN.EXE, MENU.EXE, ENDING.EXE and DATA.VOL against
the original SLPS-019.01 SHA-256 hashes. Add --quick to open the development launcher. F1 opens PC settings;
F6 opens the development launcher from the original menu.

Download tenchu-hd.tar.zst from the artwork-v3 release and place it beside
the executable/launcher. It is found and cached automatically. F1 > Visual assets
controls it. TENCHU_ASSET_PACK also accepts an archive or unpacked directory;
an empty value disables pack loading.
Mixed textures are reconstructed from your disc in Rust; Python is not needed.
Saves/settings use the normal XDG directories, or TENCHU_SAVE/TENCHU_SETTINGS.
New installs use tenchu-native under those roots. Existing
tenchu-shinobi-gaisen-pc directories are used if the new directories are absent;
their saves, campaign folders, settings and artwork stay together.
No disc image or extracted game assets are included.

F1 > General > Benchmarks runs fixed performance scenes and can select an
Auto-optimised preset for your FPS target. Presets / save as offers Low, Medium,
High, Ultra and persistent named user presets. Fresh preferences use Modern Low.

The file picker uses your desktop portal, with Zenity as a fallback. If neither
is installed, use the explicit play command above.

build-info.json and --version identify this download. Keep licenses/ with the
program. The matching dependency-source archive is provided on the release page
for rebuilding/redistribution; it is not needed to play.

Controller troubleshooting: F1 > General > Controls > Controller connection and
input test switches between gilrs (default) and SDL without restarting. Windows
also offers XInput-only / WGI-only modes. Use a Steam Gamepad layout with extra
buttons mapped to keyboard keys; keep the standard gamepad outputs enabled.
Select the intended device, test/release each button, add connection/Steam notes,
then Save diagnostic report after comparing modes. See docs/controllers.md in
the source repository for the complete comparison checklist.
