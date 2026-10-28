Tenchu: Shinobi Gaisen PC — Windows x64

Extract the archive and double-click tenchu-pc.exe to choose your game CUE,
or run from PowerShell:
  .\tenchu-pc.exe play 'C:\Games\Tenchu\game.cue'

Install Windows FFmpeg and add its bin directory to PATH for opening/ending
movies and video export, or set:
  $env:TENCHU_FFMPEG = 'C:\Tools\ffmpeg\bin\ffmpeg.exe'
Windows builds are linked from https://ffmpeg.org/download.html.

Supply your own Shinobi Gaisen disc. Keep the CUE and referenced BIN files
together. Startup verifies MAIN.EXE, MENU.EXE, ENDING.EXE and DATA.VOL against
the original SLPS-019.01 SHA-256 hashes. Add --quick to open the development launcher. F1 opens PC settings;
F6 opens the development launcher from the original menu.

Optional replacement artwork: set TENCHU_ASSET_PACK to your artwork directory.
Saves/settings use %APPDATA%\tenchu-shinobi-gaisen-pc, unless TENCHU_SAVE or
TENCHU_SETTINGS overrides the paths. No game assets or FFmpeg are bundled.

F1 > General > Benchmarks runs fixed performance scenes and can select an
Auto-optimised preset for your FPS target. Presets / save as offers Low, Medium,
High, Ultra and persistent named user presets. Fresh preferences use Modern Low.

build-info.json and --version identify this download. Keep licenses/ with the
program. The matching dependency-source archive is provided on the release page
for rebuilding/redistribution; it is not needed to play. This executable is not
code-signed; obtain it from the project's release page and check SHA256SUMS.
