# Image recipe compatibility

`src/visuals/texture_recipe.rs` and `texture_recipe/resize.rs` adapt Pillow/PIL's
integer alpha compositing and separable resampling conventions to portable Rust.
This preserves existing authored output, including premultiplied RGBA filtering,
Q22 coefficient rounding and blend truncation. There is no Python or Pillow
runtime dependency. The accompanying MIT-CMU license applies to the adapted
algorithms; retain it with redistributed binaries and source.

References: Pillow 12.1.1 `src/libImaging/Resample.c`, `AlphaComposite.c`,
`Convert.c`, `Blend.c`, `Paste.c`, `Geometry.c` and `ImagingUtils.h`:
https://github.com/python-pillow/Pillow/tree/12.1.1/src/libImaging

Synthetic golden results in `resize-golden.json` were produced with Pillow
12.3.0. They contain only hashes of procedurally generated test images, never
game artwork. Each input pixel `i` has RGB `(i*37+11, i*73+29, i*19+203) mod 256`
and alpha `[0,64,128,255][i mod 4]` (or 255 in the opaque case). Hashes use the
pack's image identity: SHA-256 of little-endian u16 dimensions followed by RGBA,
truncated to 32 hexadecimal characters.
