# Install, update and enable HD artwork

## First run

1. Open the [latest versioned release](https://github.com/Fuuzetsu/tenchu-shinobi-gaisen-pc/releases/latest)
   and expand **Assets**. Download **one** game archive:

   | Your computer | Download |
   | --- | --- |
   | Windows, Intel/AMD 64-bit | `tenchu-windows-x86_64.zip` |
   | Linux, Intel/AMD 64-bit | `tenchu-linux-x86_64.tar.gz` |
   | Linux, ARM64 | `tenchu-linux-aarch64.tar.gz` |

   GitHub's **Source code** downloads and `tenchu-dependency-sources.tar.gz`
   are not playable builds. Browser setup is [separate](guide.md#browser-build).

2. Extract the **whole** game archive into a folder you can write to.
   On Windows, run `tenchu-pc.exe` from that folder, not from inside the ZIP.
   On Linux, run `./tenchu-pc`; keep its accompanying files and directories.

3. Choose the `.cue` from your own **Tenchu: Shinobi Gaisen (SLPS-019.01)** disc
   dump. Keep every `.bin` it references at the path named inside the CUE.
   Other editions of Tenchu are not supported. The game verifies the required
   disc data and remembers this path for next time. No PlayStation BIOS is needed.

4. For opening/ending movies, install **FFmpeg** or put `ffmpeg.exe` beside the
   Windows executable. See [platform details](guide.md#build-and-inspect-a-disc).

## Add HD artwork

The game download and HD pack are separate. A graphics preset cannot supply a
missing pack.

1. Download **`tenchu-hd.tar.zst`** from
   [artwork-v4](https://github.com/Fuuzetsu/tenchu-shinobi-gaisen-pc/releases/tag/artwork-v4).
   Keep this filename. **Do not extract it** for a normal installation.
2. Put it next to the executable or Linux launcher, then start the game:

   ```text
   Tenchu/
     tenchu-pc.exe       (Windows) or tenchu-pc (Linux)
     tenchu-hd.tar.zst
     ...other files from the game archive...
   ```

3. During a mission, open **F1 → Visual assets**. Enable **Replacement artwork**.
   The status should say **Loaded for this mission: … textures, … models**
   and show the pack path. The count varies by mission; it is not the total
   number of files in the pack.
4. Toggle **Replacement artwork** to compare the same view. **F8** temporarily
   restores original graphics, including original textures; turn Retro comparison
   off when checking the pack. Lighting and model refinement have separate settings.

The first launch unpacks the archive into your user cache. Later launches reuse
it. Some artwork is reconstructed from your disc while loading a menu or mission;
no Python, external unpacker or image service is needed. Movie footage is unchanged.

If the status says **No replacement textures or models loaded**, check the
filename and placement, then restart. If you use `TENCHU_ASSET_PACK`, that path
takes priority; an empty value disables pack loading. An old `assets/hd/` folder
beside the game also takes priority over the archive. Keep custom artwork in its
own folder and select it explicitly. See [editing packs](../artwork/README.md).
If loading fails, include the status message in a diagnostic report from
**F1 → General → Save diagnostic report…**.

## Update

For the optional Tenchu 2 sandbox, use its separate
[disc and artwork setup](tenchu2-sandbox.md). Its `tenchu2-hd.tar.zst` can be
installed alongside the Gaisen pack and reused across game updates too.

1. Close the game and extract the new platform archive into a **new folder**.
   Use all its files, including Linux's shared libraries.
2. Copy your `tenchu-hd.tar.zst` beside the new executable/launcher. It does not
   need downloading again unless the release notes announce a new artwork pack.
   When updating the pack, replace the archive; its changed contents get a new
   cache automatically. Do not edit the disposable cache.
3. Start the new executable. Saves, settings, controls and the remembered disc
   path live in your user profile, so moving the installation does not reset them.
   Explicit `TENCHU_*` path overrides still apply.
4. Check **F1 → Visual assets** again. Keep the old installation until the new one
   works; you can then delete the old game folder without deleting your profile.

Use [HEAD](https://github.com/Fuuzetsu/tenchu-shinobi-gaisen-pc/releases/tag/head)
only if you want the latest successful development build. Versioned releases are
the normal download. [Changes between versions](../CHANGELOG.md).
