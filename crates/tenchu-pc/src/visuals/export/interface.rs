use crate::{source::DiscSource, visuals::catalog};
use serde_json::json;
use std::{collections::BTreeMap, fs, io::BufWriter, path::Path};
use tenchu_assets::{Result, font::Font, frontend::Image, game::GameAssets, texture::Vram};

pub fn export(assets: &GameAssets<DiscSource>, directory: &Path) -> Result<()> {
    crate::capture::review::prepare_directory(directory)?;
    fs::create_dir(directory.join("textures")).map_err(|e| e.to_string())?;
    let mut sources = vec![
        "DEMO/OPEN/TITLE.ARC".to_owned(),
        "DEMO/MENU/MENU_J.ARC".to_owned(),
        "DEMO/NUMBER.TIM".to_owned(),
        "IMAGE/FONT.TIM".to_owned(),
        "IMAGE/IMAGES.ARC".to_owned(),
    ];
    for suffix in ['E', 'J', 'F', 'I'] {
        sources.extend([
            format!("DEMO/PLAYER/PLAY_{suffix}.ARC"),
            format!("DEMO/SCORE/SELSTG_{suffix}.ARC"),
            format!("DEMO/RANKS_{suffix}.ARC"),
        ]);
    }
    for suffix in ["ENG", "JAP", "FRE", "ITA"] {
        sources.push(format!("DEMO/RS_E_{suffix}.TIM"));
    }
    let mut entries = BTreeMap::<String, serde_json::Value>::new();
    let mut references = Vec::new();
    for source in sources {
        let bytes = assets.asset(&source)?;
        let images = if source.ends_with(".ARC") {
            Image::archive(&bytes)?
        } else {
            vec![Image::parse(&bytes)?]
        };
        for (index, image) in images.iter().enumerate() {
            let rgba: Vec<_> = image
                .pixels
                .iter()
                .flat_map(|&word| {
                    let alpha = if word == 0 {
                        0
                    } else if word & 0x8000 != 0 {
                        128
                    } else {
                        255
                    };
                    [
                        ((word & 31) * 255 / 31) as u8,
                        (((word >> 5) & 31) * 255 / 31) as u8,
                        (((word >> 10) & 31) * 255 / 31) as u8,
                        alpha,
                    ]
                })
                .collect();
            let id = catalog::image_id(image.size.map(|n| n as u16), &rgba);
            if !entries.contains_key(&id) {
                let path = directory.join("textures").join(format!("{id}.png"));
                let file = fs::File::create(path).map_err(|e| e.to_string())?;
                let mut png = png::Encoder::new(
                    BufWriter::new(file),
                    image.size[0] as u32,
                    image.size[1] as u32,
                );
                png.set_color(png::ColorType::Rgba);
                png.set_depth(png::BitDepth::Eight);
                png.write_header()
                    .map_err(|e| e.to_string())?
                    .write_image_data(&rgba)
                    .map_err(|e| e.to_string())?;
                entries.insert(id.clone(), json!({"id":id,"size":image.size}));
            }
            references.push(json!({"source":source,"index":index,"id":id}));
        }
    }
    let mut vram = Vram::default();
    let font = Font::load(assets, &mut vram)?;
    let glyphs: Vec<_> = (32..=255u8)
        .map(|code| {
            let (glyph, nudge) = font.glyph(code);
            json!({"code":code,"advance":font.width(code),"nudge":nudge,
            "uv":glyph.uv,"page":glyph.page,"size":glyph.size})
        })
        .collect();
    let manifest = json!({"version":1,"textures":entries.keys().collect::<Vec<_>>(),
        "entries":entries.values().collect::<Vec<_>>(),"references":references,"glyphs":glyphs});
    fs::write(
        directory.join("catalog.json"),
        serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    println!(
        "Saved {} interface identities to {}",
        entries.len(),
        directory.display()
    );
    Ok(())
}
