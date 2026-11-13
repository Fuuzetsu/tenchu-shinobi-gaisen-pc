use super::*;
use crate::level_editor::{Action, Cue, Event, Project};

pub(crate) fn wav() -> Vec<u8> {
    let samples: Vec<i16> = (0..800)
        .map(|i| ((i as f32 * 0.31).sin() * 8000.0) as i16)
        .collect();
    let mut bytes = b"RIFF".to_vec();
    bytes.extend((36 + samples.len() as u32 * 2).to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16u32.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(8000u32.to_le_bytes());
    bytes.extend(16000u32.to_le_bytes());
    bytes.extend(2u16.to_le_bytes());
    bytes.extend(16u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend((samples.len() as u32 * 2).to_le_bytes());
    bytes.extend(samples.iter().flat_map(|s| s.to_le_bytes()));
    bytes
}

fn png() -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 2, 2);
        encoder.set_color(png::ColorType::Rgba);
        let mut writer = encoder.write_header().unwrap();
        writer
            .write_image_data(&[255, 50, 20, 255].repeat(4))
            .unwrap();
    }
    bytes
}

pub(crate) fn sample() -> (Assets, Model, String) {
    let texture = Asset::import("cloth.png".into(), Kind::Texture, png()).unwrap();
    let audio = Asset::import("bell.wav".into(), Kind::Audio, wav()).unwrap();
    let model = Asset::import(
        "banner.obj".into(),
        Kind::Model,
        b"v -1 0 0\nv 1 0 0\nv 1 2 0\nvt 0 0\nvt 1 0\nvt 1 1\nusemtl cloth\nf 1/1 2/2 3/3\n"
            .to_vec(),
    )
    .unwrap();
    let instance = Model {
        asset: model.id().into(),
        scale: 1000,
        y_up: true,
        materials: BTreeMap::from([("cloth".into(), Some(texture.id().into()))]),
    };
    let sound = audio.id().to_owned();
    let assets = [texture, audio, model]
        .into_iter()
        .map(|a| (a.id().into(), a))
        .collect();
    (assets, instance, sound)
}

#[test]
fn exported_map_contains_assets_and_history_shares_decoded_contents() {
    let (assets, model, sound) = sample();
    let mut project = Project::new(
        tenchu_assets::trial::Mission::new(0, tenchu_assets::trial::Objective::ReachLord, 140)
            .unwrap(),
    );
    project.assets = assets;
    project.scenery.push(crate::level_editor::project::Scenery {
        custom: Some(model),
        name: "Banner".into(),
        game: super::super::AssetGame::Tenchu,
        stage: 0,
        part: 0,
        position: [1000, 0, 1000],
        yaw: 0,
    });
    project.events.push(Event {
        cues: vec![Cue {
            delay: 0,
            action: Action::CustomSound {
                asset: sound.clone(),
                volume: 100,
                pan: 64,
            },
        }],
        ..Default::default()
    });
    let exported = project.encode().unwrap();
    let copy = project.clone();
    assert!(Arc::ptr_eq(
        &project.assets[&sound].data.0,
        &copy.assets[&sound].data.0
    ));
    assert!(Arc::ptr_eq(
        &project.assets[&sound].clip().unwrap(),
        &copy.assets[&sound].clip().unwrap()
    ));
    drop(copy);
    drop(project);
    let mut loaded = Project::decode(&exported).unwrap();
    let clip = loaded.assets[&sound].clip().unwrap();
    assert_eq!(clip.rate, 8000);
    assert_eq!(clip.frames.len(), 800);
    assert!(clip.frames.iter().all(|f| f[0] == f[1]));
    assert!(clip.frames.iter().any(|f| f[0].abs() > 7000));
    let model = loaded.scenery[0].custom.as_ref().unwrap();
    let (bounds, pack) = model.build(&loaded.assets, "banner").unwrap();
    assert!(bounds.geometry[0].vertices.contains(&[1000, -2000, 0]));
    let triangle = &pack.models["banner"][0];
    assert_eq!(triangle.corners[1].position, [1000.0, -2000.0, 0.0]);
    assert_eq!(pack.images.values().next().unwrap().size, [2, 2]);
    loaded.assets.remove(&sound);
    assert!(loaded.validate().unwrap_err().contains("Missing Audio"));
}

#[test]
fn rejects_corrupt_assets_and_material_links_without_external_file_access() {
    for kind in [Kind::Texture, Kind::Model, Kind::Audio] {
        assert!(Asset::import("bad".into(), kind, b"garbage".to_vec()).is_err());
    }
    let (mut assets, mut model, _) = sample();
    model
        .materials
        .insert("cloth".into(), Some("../../outside.png".into()));
    assert!(model.build(&assets, "x").is_err());
    let asset = assets.pop_first().unwrap().1;
    assets.insert("forged hash".into(), asset);
    assert!(validate(&assets).is_err());
    let mut many = "v 0 0 0\nv 1 0 0\nv 0 1 0\n".to_owned();
    for i in 0..257 {
        many.push_str(&format!("usemtl material{i}\nf 1 2 3\n"));
    }
    assert!(Asset::import("too many materials".into(), Kind::Model, many.into_bytes()).is_err());
}

#[test]
#[ignore = "requires TENCHU_STUDIO_AUDIO_FIXTURES with independently encoded tone.wav, tone.ogg, tone.mp3 and tone.flac"]
fn decodes_each_supported_custom_audio_format() {
    let root = std::path::PathBuf::from(std::env::var_os("TENCHU_STUDIO_AUDIO_FIXTURES").unwrap());
    for extension in ["wav", "ogg", "mp3", "flac"] {
        let asset = Asset::import(
            format!("tone.{extension}"),
            Kind::Audio,
            std::fs::read(root.join(format!("tone.{extension}"))).unwrap(),
        )
        .unwrap();
        let clip = asset.clip().unwrap();
        assert_eq!(clip.rate, 44100, "{extension}");
        assert!(
            (43_000..=47_000).contains(&clip.frames.len()),
            "{extension}: {}",
            clip.frames.len()
        );
        assert!(clip.frames.iter().any(|f| f[0].abs() > 2000), "{extension}");
    }
}
