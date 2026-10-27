Tenchu: Shinobi Gaisen PC — Linux

Extract the whole archive and run ./tenchu-pc to choose your game CUE, or:
  ./tenchu-pc play '/path/to/game.cue'

This archive includes the program's shared libraries and dynamic loader. Nix
is not required. Keep lib/, libexec/ and share/ beside the tenchu-pc launcher;
do not launch libexec/tenchu-pc directly. Use the archive matching your CPU:
x86_64 for ordinary Intel/AMD PCs, aarch64 for 64-bit ARM Linux.

An installed Vulkan or OpenGL GPU driver, a graphical desktop and working
desktop audio are required for gameplay. GPU drivers are supplied by your OS.
Install FFmpeg through your distribution for opening/ending movies and video
export, or set TENCHU_FFMPEG to its executable. FFmpeg is not bundled.

Supply your own Shinobi Gaisen disc. Keep the CUE and referenced BIN files
together. Startup verifies MAIN.EXE, MENU.EXE, ENDING.EXE and DATA.VOL against
the original SLPS-019.01 SHA-256 hashes. Add --quick to open the development launcher. F1 opens PC settings;
F6 opens the development launcher from the original menu.

Optional replacement artwork: set TENCHU_ASSET_PACK to your artwork directory.
Saves/settings use the normal XDG directories, or TENCHU_SAVE/TENCHU_SETTINGS.
No disc image or extracted game assets are included.

F1 > General > Benchmarks runs fixed performance scenes and can select an
Auto-optimised preset for your FPS target. Presets / save as offers Low, Medium,
High, Ultra and persistent named user presets. Fresh preferences use Modern Low.

The file picker uses your desktop portal, with Zenity as a fallback. If neither
is installed, use the explicit play command above.
