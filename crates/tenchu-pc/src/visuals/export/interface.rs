use crate::{source::DiscSource, visuals::catalog};
use serde_json::json;
use std::{collections::BTreeMap, fs, io::BufWriter, path::Path};
use tenchu_assets::{
    Result,
    font::Font,
    frontend::{
        Image,
        mission::{Briefing, Screens},
    },
    game::GameAssets,
    script::Language,
    texture::Vram,
};

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
    for language in [
        Language::English,
        Language::Japanese,
        Language::French,
        Language::Italian,
    ] {
        let screens = Screens::load(&assets.executable, language)?;
        sources.extend([
            screens.shop,
            screens.help,
            screens.results,
            screens.ranks,
            screens.training,
            screens.training_ranks,
        ]);
        for stage in assets.stages()?.into_iter().filter(|stage| stage.order > 0) {
            let briefing = Briefing::load(&assets.executable, language, stage.id, 0)?;
            sources.extend([briefing.background, briefing.text]);
        }
    }
    for source in &mut sources {
        *source = source.replace('\\', "/").to_ascii_uppercase();
    }
    sources.sort_unstable();
    sources.dedup();
    let mut entries = BTreeMap::<String, serde_json::Value>::new();
    let mut references = Vec::new();
    for source in sources {
        let bytes = assets.asset(&source)?;
        let images = if source.ends_with(".TIM") {
            Image::parse(&bytes).map(|image| vec![image])
        } else {
            Image::archive(&bytes)
        }
        .map_err(|error| format!("{source}: {error}"))?;
        for (index, image) in images.iter().enumerate() {
            let rgba: Vec<_> = image
                .pixels
                .iter()
                .copied()
                .flat_map(tenchu_assets::texture::rgba8)
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
