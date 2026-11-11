use super::super::transport::{Input, Transport};
use super::*;
use crate::source::DiscSource;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

struct Host {
    playback: Playback<DiscSource>,
    rewind: Rewind,
    transport: Transport,
    drawing: ContentDrawing,
    renderer: Renderer,
    settings: Settings,
    tools: Tools,
    expected: BTreeMap<u64, [u8; 32]>,
}

fn source(playback: &Playback<DiscSource>) -> [u8; 32] {
    Sha256::digest(postcard::to_allocvec(&playback.snapshot().unwrap()).unwrap()).into()
}

impl Host {
    fn new(sequence: usize) -> Self {
        let disc =
            crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
        let mut playback = Playback::load(disc, sequence, 2, 17, Some([0; 21]), None).unwrap();
        let settings = Settings {
            rewind_seconds: 30,
            rewind_speed: 100,
            ..Default::default()
        };
        playback.configure(&settings).unwrap();
        let instance = wgpu::Instance::new(&Default::default());
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let mut renderer = pollster::block_on(Renderer::new(
            &adapter,
            wgpu::TextureFormat::Rgba8Unorm,
            [640, 480],
            playback.vram(),
        ))
        .unwrap();
        let graphics = super::super::preferences::graphics(&settings);
        renderer.configure(&graphics);
        let drawing = ContentDrawing::new(&playback.content, &graphics, &mut renderer).unwrap();
        let mut host = Self {
            playback,
            rewind: Default::default(),
            transport: Default::default(),
            drawing,
            renderer,
            settings,
            tools: Default::default(),
            expected: Default::default(),
        };
        host.ready();
        host
    }

    fn ready(&mut self) {
        self.tools
            .sync(&self.playback.content, self.playback.generation);
        self.drawing
            .update(
                &self.playback.content,
                &mut self.renderer,
                &self.tools.debug,
                &self.settings,
            )
            .unwrap();
        assert!(
            self.rewind
                .record(
                    &self.playback,
                    &self.drawing,
                    &mut self.settings,
                    &self.tools
                )
                .is_none()
        );
        if supported(&self.playback.content) {
            self.expected
                .insert(self.rewind.tick, source(&self.playback));
        }
        self.rewind.sync(&mut self.tools);
    }

    fn step(&mut self) {
        self.rewind.resume().unwrap();
        self.playback.configure(&self.settings).unwrap();
        let graphics = self.settings.rendering().graphics.clone();
        let update = self
            .transport
            .frame_step(&mut self.playback, 0, None, |playback, observed| {
                self.rewind.observed(playback, observed, &self.tools);
                if !observed.replaced {
                    self.drawing.tick(&playback.content, &graphics)?;
                    assert!(
                        self.rewind
                            .record(playback, &self.drawing, &mut self.settings, &self.tools)
                            .is_none()
                    );
                    if supported(&playback.content) {
                        self.expected.insert(self.rewind.tick, source(playback));
                    }
                }
                Ok(())
            })
            .unwrap();
        if update.changed {
            self.drawing
                .replace(
                    &self.playback.content,
                    self.playback.vram(),
                    &super::super::preferences::graphics(&self.settings),
                    &mut self.renderer,
                    false,
                )
                .unwrap();
            self.ready();
        }
    }

    fn state(&mut self) -> (&mut Rewind, State<'_, DiscSource>) {
        (
            &mut self.rewind,
            State {
                playback: &mut self.playback,
                drawing: &mut self.drawing,
                settings: &mut self.settings,
                tools: &mut self.tools,
                renderer: &mut self.renderer,
            },
        )
    }

    fn action(&mut self, action: Action) {
        let (rewind, mut state) = self.state();
        rewind.apply(action, &mut state, None).unwrap();
    }

    fn check(&self, tick: u64) {
        assert_eq!(self.rewind.tick, tick);
        assert_eq!(
            source(&self.playback),
            self.expected[&tick],
            "source at timeline tick {tick}"
        );
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC and GPU; native timeline crosses gameplay/story clocks, seeks, plays forward and branches with exact state"]
fn native_timeline_preserves_transitions_and_branch_continuation() {
    let mut host = Host::new(9);
    assert_eq!(host.rewind.history.entries.len(), 1);
    for _ in 0..60 {
        host.step();
    }
    let count = host.rewind.history.entries.len();
    host.transport
        .advance(
            &mut host.playback,
            Duration::from_secs(1),
            Input {
                buttons: 0,
                cutscene_speed: 1,
                simulation_speed: 100,
                fast_forward: false,
                paused: true,
            },
            None,
            |_, _| panic!("pause recorded an update"),
        )
        .unwrap();
    assert_eq!(host.rewind.history.entries.len(), count);
    if let Content::Gameplay(session) = &mut host.playback.content {
        session.world.mission.phase = tenchu_core::tenchu2::mission::Phase::Finished {
            outcome: tenchu_core::tenchu2::mission::Outcome::Arrived,
            finished_at: session.world.tick,
        };
    }
    host.step();
    assert!(matches!(host.playback.content, Content::Story { .. }));
    assert_eq!(host.rewind.tick, 61);
    assert_eq!(host.playback.content.tick(), 0);
    for _ in 0..3 {
        host.step();
    }
    for tick in [59, 61, 60, 64] {
        host.action(Action::Seek(tick));
        host.check(tick);
    }
    let count = host.rewind.history.entries.len();
    host.action(Action::Seek(59));
    host.action(Action::Forward);
    for _ in 0..5 {
        let (rewind, mut state) = host.state();
        rewind
            .advance(Duration::from_millis(34), &mut state)
            .unwrap();
    }
    host.check(64);
    assert!(!host.rewind.active && host.rewind.preview);
    assert_eq!(host.rewind.history.entries.len(), count);
    host.action(Action::Toggle);
    host.settings.rewind_speed = 200;
    let (rewind, mut state) = host.state();
    rewind
        .advance(Duration::from_millis(100), &mut state)
        .unwrap();
    host.check(58);
    host.action(Action::Resume);
    assert!(!host.rewind.preview);
    assert_eq!(host.rewind.history.entries.back().unwrap().tick, 58);
    let expected = host.expected[&59];
    host.step();
    assert_eq!(source(&host.playback), expected);
    assert_eq!(host.rewind.history.entries.back().unwrap().tick, 59);
    host.settings.rewind_seconds = 0;
    host.ready();
    assert!(host.rewind.history.entries.is_empty());
}

#[test]
#[ignore = "requires TENCHU2_DISC and GPU; native recordings round-trip controller metadata, selected frames, graphics and camera; invalid imports leave the current source and history intact"]
fn native_timeline_recordings_restore_metadata_and_graphics_atomically() {
    let mut host = Host::new(119);
    host.settings.walking.enabled = true;
    host.settings.sheathing.enabled = true;
    host.settings.sheathing.unarmed = true;
    host.settings.sheathing.unarmed_style = tenchu_core::game::sheathing::UnarmedStyle::Tatsumaru;
    host.settings.dismemberment.enabled = true;
    host.settings.dismemberment.player = true;
    host.playback.configure(&host.settings).unwrap();
    let camera = crate::debug::camera::Camera {
        position: glam::Vec3::new(500.0, -2000.0, 1200.0),
        yaw: 0.3,
        pitch: -0.2,
        fov: 70.0,
    };
    host.tools.debug.camera = Some(camera);
    if let Content::Gameplay(session) = &mut host.playback.content {
        session.world.controls.walking.as_mut().unwrap().held = true;
        session.world.actors[session.world.player]
            .actor
            .sheathing
            .as_mut()
            .unwrap()
            .input(true);
        session.navigation = Some(super::super::NavigationInput {
            yaw: 0.3,
            pitch: 200,
            sticks: [glam::Vec2::ZERO; 2],
            deadzone: 18,
            align_aim: true,
        });
    }
    for _ in 0..30 {
        host.step();
    }
    let report = super::super::diagnostics::report(
        &host.playback,
        &host.transport,
        &super::super::preferences::Menu::default(),
        &host.settings,
        &Default::default(),
    )
    .to_string();
    let hash = tenchu_assets::tenchu2::Edition::Japan.game_hash();
    let bytes = host
        .rewind
        .history
        .export_for([10, 30], report, hash)
        .unwrap();
    assert!(History::import(&bytes).is_err());
    let (history, _) = History::import_for(&bytes, hash).unwrap();
    assert_eq!(
        serde_json::to_value(&history.entries[0].input).unwrap(),
        serde_json::to_value(&host.rewind.history.entries[10].input).unwrap()
    );
    assert!(
        history.entries[0]
            .input
            .as_ref()
            .unwrap()
            .native
            .as_ref()
            .unwrap()
            .navigation
            .is_some()
    );
    assert!(
        history.entries[0]
            .input
            .as_ref()
            .unwrap()
            .native
            .as_ref()
            .unwrap()
            .walking
            .is_some_and(|walking| walking.held)
    );
    let graphics = host.settings.graphics.clone();
    assert!(history.entries.iter().any(|entry| {
        entry
            .input
            .as_ref()
            .and_then(|input| input.native.as_ref())
            .and_then(|input| input.sheathing)
            .is_some()
    }));
    host.settings.graphics.fog = !graphics.fog;
    host.tools.debug.camera = None;
    let (rewind, mut state) = host.state();
    rewind.load(&bytes, &mut state).unwrap();
    host.check(10);
    assert_eq!(host.rewind.history.entries.len(), 21);
    assert_eq!(host.settings.graphics, graphics);
    assert_eq!(
        serde_json::to_value(host.tools.debug.camera).unwrap(),
        serde_json::to_value(Some(camera)).unwrap()
    );
    host.action(Action::RecordedVisuals(false));
    host.settings.graphics.fog = !graphics.fog;
    host.tools.debug.camera = None;
    host.action(Action::Seek(25));
    host.check(25);
    assert_ne!(host.settings.graphics.fog, graphics.fog);
    assert!(host.tools.debug.camera.is_none());
    let before = source(&host.playback);
    let old_tick = host.rewind.tick;
    let (rewind, mut state) = host.state();
    let error = rewind
        .load(&bytes[..bytes.len() - 1], &mut state)
        .unwrap_err();
    assert!(!error.drawing_changed);
    assert_eq!(source(&host.playback), before);
    assert_eq!(host.rewind.tick, old_tick);
    assert_eq!(host.rewind.history.entries.len(), 21);
    host.action(Action::RecordedVisuals(true));
    assert_eq!(host.settings.graphics, graphics);
    host.action(Action::Resume);
    assert_eq!(host.rewind.history.entries.back().unwrap().tick, 25);
    let expected = host.expected[&26];
    host.step();
    assert_eq!(source(&host.playback), expected);
}
