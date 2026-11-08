use super::*;

#[test]
#[ignore = "requires TENCHU2_DISC, TENCHU_MODEL_REVIEW and a GPU; streamed cutscene meshes and facial updates match a fresh frame"]
fn animated_story_draw_matches_fresh_meshes() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let graphics = Graphics {
        asset_pack: false,
        ..Default::default()
    };
    for (sequence, event) in [(0, 201), (10, 4)] {
        let streamed = output.join(format!("story-{event}-streamed.png"));
        let fresh = output.join(format!("story-{event}-fresh.png"));
        let mut story = Story::load(&disc, sequence, event, 17).unwrap();
        story.step(true).unwrap();
        let vram = story.level.vram.clone();
        capture_frame(&vram, &graphics, &streamed, |renderer| {
            let mut drawing = Drawing::new(&story, &graphics, renderer)?;
            for _ in 1..=600 {
                story.step(true)?;
                drawing.update(&story, renderer)?;
            }
            Ok((drawing.scene, story.state.view, drawing.texts))
        })
        .unwrap();
        capture_frame(&vram, &graphics, &fresh, |renderer| {
            let drawing = Drawing::new(&story, &graphics, renderer)?;
            Ok((drawing.scene, story.state.view, drawing.texts))
        })
        .unwrap();
        assert_eq!(
            std::fs::read(&streamed).unwrap(),
            std::fs::read(&fresh).unwrap(),
            "event {event}: animated face/material updates differ from fresh scene"
        );
    }
}
