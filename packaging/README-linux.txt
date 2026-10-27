Tenchu: Shinobi Gaisen PC — Linux

Extract the whole archive and run:
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
together. Add --quick to open the development launcher. F1 opens PC settings;
F6 opens the development launcher from the original menu.

Optional replacement artwork: set TENCHU_ASSET_PACK to your artwork directory.
Saves/settings use the normal XDG directories, or TENCHU_SAVE/TENCHU_SETTINGS.
No disc image or extracted game assets are included.
