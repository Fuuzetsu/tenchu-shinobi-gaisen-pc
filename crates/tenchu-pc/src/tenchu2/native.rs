use std::{collections::BTreeSet, path::Path, sync::Arc};
use tenchu_assets::audio::voice::{Dialect, Sourced, VoiceSet};
use tenchu_assets::{
    ReadAt, Result,
    disc::Disc,
    tenchu2::{audio::Audio, layout::Layout},
};
use tenchu_core::{
    math::Position,
    random::Random,
    tenchu2::{
        audio::{Listener, Sound},
        campaign::Run,
        control::CombatCue,
        environment,
        inventory::Inventory,
        world::{
            Assets, Frame, Options, World,
            controls::{Event, EventSource, Events},
        },
    },
};

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod campaign;
mod contact;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod diagnostics;
#[cfg(not(target_arch = "wasm32"))]
mod equipment;
#[cfg(not(target_arch = "wasm32"))]
mod film;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod gaisen;
#[cfg(test)]
mod gaisen_tests;
mod inspection;
mod interpolation;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod models;
mod navigation;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod preferences;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod presentation;
mod render;
#[cfg(not(target_arch = "wasm32"))]
mod results;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod save;
#[cfg(not(target_arch = "wasm32"))]
mod selection;
mod story;
#[cfg(test)]
mod tests;
#[cfg(not(target_arch = "wasm32"))]
mod tools;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod transport;
#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
mod window;

pub struct Session {
    pub world: World,
    pub assets: Assets,
    pub frame: Frame,
    contact: contact::History,
    pub bank: Arc<tenchu_assets::audio::bank::Bank>,
    pub pending: BTreeSet<String>,
    pub interpolation: Option<crate::visuals::interpolation::History>,
    pub navigation: Option<navigation::Input>,
    pub circle_action: tenchu_core::input::CircleAction,
    #[cfg(not(target_arch = "wasm32"))]
    pub gaisen: Option<gaisen::Scene>,
    #[cfg(not(target_arch = "wasm32"))]
    pub models: models::Override,
}

impl Session {
    pub fn presentation_tick(&self) -> u64 {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(scene) = &self.gaisen {
            return scene.director.source.tick;
        }
        u64::from(self.world.tick)
    }

    pub fn load(disc: &Disc<impl ReadAt>, sequence: usize, seed: u32) -> Result<Self> {
        let hero = if sequence >= 185 {
            2
        } else {
            usize::from(sequence >= 101)
        };
        let campaign = tenchu_assets::tenchu2::campaign::Campaign::load(disc)?;
        let mut run = Run::new(&campaign, hero, sequence, 2, seed)?;
        run.prepare_area(&campaign)?;
        run.carry.inventory[0] = 255;
        Self::for_run(disc, &run)
    }

    pub fn for_run(disc: &Disc<impl ReadAt>, run: &Run) -> Result<Self> {
        let mut session = Self::with_options(
            disc,
            run.sequence,
            Options {
                hero: run.hero,
                difficulty: run.difficulty,
                seed: run.random.seed,
                inventory: Inventory::from_amounts(run.carry.inventory),
            },
            run.carry.alternate,
        )?;
        run.restore_area(&mut session.world, &session.assets, true)?;
        Ok(session)
    }

    fn with_options(
        disc: &Disc<impl ReadAt>,
        sequence: usize,
        options: Options,
        alternate: bool,
    ) -> Result<Self> {
        let layout = Layout::available(disc)?
            .into_iter()
            .find(|l| l.sequence == sequence)
            .ok_or("No Japanese Tenchu 2 layout with that sequence")?;
        let hero = options.hero;
        let assets = Assets::equipped(disc, hero, &layout, alternate)?;
        let world = World::new(&layout, options, &assets)?;
        let bank = Arc::new(Audio::load(disc, hero, layout.map)?.bank);
        Ok(Self::from_parts(world, assets, bank))
    }

    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub fn gaisen(
        disc: &Disc<impl ReadAt>,
        source: &tenchu_assets::stage::MissionAssets,
        options: Options,
        alternate: bool,
    ) -> Result<Self> {
        Self::gaisen_with_water_depth(
            disc,
            source,
            options,
            alternate,
            tenchu_assets::tenchu2::collision::Gaisen::DEFAULT_WATER_DEPTH,
        )
    }

    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub fn gaisen_with_water_depth(
        disc: &Disc<impl ReadAt>,
        source: &tenchu_assets::stage::MissionAssets,
        options: Options,
        alternate: bool,
        water_depth: i16,
    ) -> Result<Self> {
        let (mut assets, layout) = Assets::gaisen(disc, source, options.hero, alternate)?;
        assets.level.collision.set_gaisen_water_depth(water_depth)?;
        let bank = gaisen_bank(disc, &assets, options.hero)?;
        let world = World::new(&layout, options, &assets)?;
        Ok(Self::from_parts(world, assets, Arc::new(bank)))
    }

    fn from_parts(
        world: World,
        assets: Assets,
        bank: Arc<tenchu_assets::audio::bank::Bank>,
    ) -> Self {
        Self {
            world,
            assets,
            frame: Frame::default(),
            contact: Default::default(),
            bank,
            pending: BTreeSet::new(),
            interpolation: None,
            navigation: None,
            circle_action: Default::default(),
            #[cfg(not(target_arch = "wasm32"))]
            gaisen: None,
            #[cfg(not(target_arch = "wasm32"))]
            models: Default::default(),
        }
    }

    pub fn step(&mut self, buttons: u16) -> Result<Vec<Sourced<Sound>>> {
        if matches!(
            self.world.mission.phase,
            tenchu_core::tenchu2::mission::Phase::Finished { .. }
        ) {
            return Ok(Vec::new());
        }
        self.step_world(buttons, false)
    }

    fn step_world(&mut self, buttons: u16, observing_death: bool) -> Result<Vec<Sourced<Sound>>> {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(scene) = &mut self.gaisen {
            let was_presenting = scene.director.presenting;
            let presenting = scene
                .director
                .update(&mut self.world, &self.assets, buttons)?;
            if presenting {
                scene.audio.step(&scene.director.source, None)?;
                self.navigation = None;
                return Ok(Vec::new());
            }
            if was_presenting {
                self.contact = Default::default();
                self.reset_interpolation()?;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        let t2_moveset = self.models.uses_t2_moveset();
        #[cfg(target_arch = "wasm32")]
        let t2_moveset = true;
        let buttons = if self.circle_action.guards(t2_moveset) {
            buttons
        } else {
            tenchu_core::input::swap_stance_buttons(buttons.swap_bytes()).swap_bytes()
        };
        let buttons = navigation::apply(self, buttons)?;
        let mut events = Output {
            player: self.world.player,
            #[cfg(not(target_arch = "wasm32"))]
            voice: self.models.voice.as_ref(),
            #[cfg(target_arch = "wasm32")]
            voice: None,
            listener: self.world.listener(),
            requests: Vec::new(),
            wounds: Vec::new(),
            melee_hits: BTreeSet::new(),
            pending: &mut self.pending,
        };
        let tick = self.world.tick;
        self.frame = self
            .world
            .step([buttons, 0], &mut self.assets, &mut events)?;
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(scene) = &mut self.gaisen {
            scene.director.advance_ambient(&mut self.world)?;
            let listener = self.world.listener();
            scene.audio.step(
                &scene.director.source,
                Some(tenchu_core::audio::Listener {
                    position: listener.position,
                    yaw: listener.yaw,
                    direction: None,
                }),
            )?;
        }
        self.contact
            .record(u64::from(self.world.tick), &self.frame.effects);
        self.contact.record_player(&self.world);
        self.contact.wounds = events.wounds;
        self.contact.melee_hits = events.melee_hits;
        for &position in &self.frame.effects.blood_sounds {
            events.requests.push(SoundRequest {
                source: None,
                id: 29,
                listener: self.world.listener(),
                position: Some(position),
                volume: None,
            });
        }
        let mut sounds: Vec<_> = self.frame.ambience.into_iter().map(Sourced::from).collect();
        for request in events.requests {
            if let Some((direction, volume)) = request
                .listener
                .parameters(request.position, &self.assets.resources.math)?
                && let Some(sound) = self.world.audio.play(
                    request.id,
                    direction,
                    request.volume.unwrap_or(volume),
                    tick,
                    &self.assets.resources.math,
                )
            {
                let mut sound = sound;
                if let Some((_, id)) = &request.source {
                    sound.id = *id;
                }
                sounds.push(Sourced {
                    sound,
                    bank: request.source.map(|(bank, _)| bank),
                });
            }
        }
        if !observing_death {
            sounds.extend(
                self.world
                    .advance_mission(&self.assets)?
                    .into_iter()
                    .map(Sourced::from),
            );
            self.frame.transition_fade = self.world.mission.fade;
        }
        if let Some(history) = &mut self.interpolation {
            history.record(interpolation::gameplay(&self.world, &self.assets)?);
        }
        Ok(sounds)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn observe_death(&mut self) -> Result<Vec<Sourced<Sound>>> {
        use tenchu_core::tenchu2::mission::{Outcome, Phase};
        let mission = &mut self.world.mission;
        let (Phase::Ending {
            outcome: Outcome::Dead,
            finished_at,
            ..
        }
        | Phase::Finished {
            outcome: Outcome::Dead,
            finished_at,
        }) = mission.phase
        else {
            return Err("The native player has not finished dying".into());
        };
        let (phase, fade) = (mission.phase, mission.fade);
        // Keep source actors/items/effects running before the failure fade and cleanup.
        mission.phase = Phase::Ending {
            outcome: Outcome::Dead,
            remaining: 100,
            finished_at,
        };
        mission.fade = None;
        self.navigation = None;
        let result = self.step_world(0, true);
        self.world.mission.phase = phase;
        self.world.mission.fade = fade;
        result
    }

    pub fn interpolate(&mut self, enabled: bool) -> Result<()> {
        if enabled && self.interpolation.is_none() {
            let mut history = crate::visuals::interpolation::History::default();
            history.reset_to(interpolation::gameplay(&self.world, &self.assets)?);
            self.interpolation = Some(history);
        } else if !enabled {
            self.interpolation = None;
        }
        Ok(())
    }

    pub fn reset_interpolation(&mut self) -> Result<()> {
        if let Some(history) = &mut self.interpolation {
            history.reset_to(interpolation::gameplay(&self.world, &self.assets)?);
        }
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn gaisen_bank(
    disc: &Disc<impl ReadAt>,
    assets: &Assets,
    hero: usize,
) -> Result<tenchu_assets::audio::bank::Bank> {
    let mut bank = Audio::load(disc, hero, 8)?.bank;
    bank.programs.truncate(5);
    let voices =
        tenchu_assets::tenchu2::audio::voices_for(disc, &assets.resources.registry, &assets.cast)?;
    for &kind in &assets.cast {
        let voice = voices
            .get(&kind)
            .ok_or_else(|| format!("Missing imported guard voice {kind:#x}"))?;
        bank.import_program(&voice.bank, 0)?;
    }
    Ok(bank)
}

struct Output<'a> {
    player: usize,
    voice: Option<&'a Arc<VoiceSet>>,
    listener: Listener,
    requests: Vec<SoundRequest>,
    wounds: Vec<contact::Wound>,
    melee_hits: BTreeSet<usize>,
    pending: &'a mut BTreeSet<String>,
}

struct SoundRequest {
    source: Option<(Arc<tenchu_assets::audio::bank::Bank>, u8)>,
    id: u8,
    listener: Listener,
    position: Option<Position>,
    volume: Option<u8>,
}

impl Output<'_> {
    fn pending(&mut self, event: impl std::fmt::Debug) {
        let description = format!("{event:?}");
        let kind = description.split([' ', '(']).next().unwrap_or(&description);
        if self.pending.insert(kind.into()) {
            eprintln!("Native JP integration pending: {description}");
        }
    }
}

impl Events for Output<'_> {
    fn melee_hit(&mut self, attacker: usize) {
        self.melee_hits.insert(attacker);
    }

    fn blood(&mut self, owner: usize, joint: usize, position: Position) {
        self.wounds.push(contact::Wound {
            owner,
            joint,
            position,
        });
    }

    fn emit(&mut self, source: EventSource<'_>, event: Event, _: &mut Random) -> Result<()> {
        self.listener = source.listener;
        let position = (source.index != source.listener.actor).then_some(source.actor.position);
        let mut volume = None;
        let mut bank = None;
        let (id, position) = match event {
            Event::Voice(id) | Event::Combat(CombatCue::Voice(id)) => {
                if id < 16
                    && source.index == self.player
                    && let Some(voice) = self.voice
                {
                    bank = Some((
                        voice.bank.clone(),
                        voice.cue(Dialect::Tenchu2, id, source.actor.health == 0) as u8,
                    ));
                }
                (if id < 16 { id | source.voice << 4 } else { id }, position)
            }
            Event::Sound(id) => (id, None),
            Event::ItemSound(tenchu_core::tenchu2::control::ItemSound::EmptyInventory) => {
                (6, position)
            }
            Event::ItemSound(tenchu_core::tenchu2::control::ItemSound::Unavailable) => (6, None),
            Event::SoundVolume { id, volume: gain } => {
                volume = Some(gain);
                (id, None)
            }
            Event::SoundPosition { id, position } => (id, Some(position)),
            Event::SoundAt { id, positional } => (id, if positional { position } else { None }),
            Event::ShowHealth
            | Event::Pickup(_)
            | Event::StealthTint
            | Event::Rumble { .. }
            | Event::Combat(CombatCue::Rumble { .. })
            | Event::Landing
            | Event::WallJump => return Ok(()),
            _ => {
                self.pending(event);
                return Ok(());
            }
        };
        self.requests.push(SoundRequest {
            source: bank,
            id,
            listener: source.listener,
            position,
            volume,
        });
        Ok(())
    }
}

impl environment::Events for Output<'_> {
    fn emit(&mut self, event: environment::Event, _: &mut Random) -> Result<()> {
        if let environment::Event::Sound { position, id } = event {
            self.requests.push(SoundRequest {
                source: None,
                id,
                listener: self.listener,
                position: Some(position),
                volume: None,
            });
        } else if !matches!(event, environment::Event::Rumble { .. }) {
            self.pending(event);
        }
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn command(args: &[std::ffi::OsString]) -> Result<()> {
    let capture = args[0] == "render-t2-jp";
    let first = if capture { 3 } else { 2 };
    if args.len() < first || args[1] == "--help" {
        println!(
            "tenchu-pc play-t2-jp TENCHU2.cue [--menu | --sequence ID] [--seed N] [--loadout 4:20]\ntenchu-pc render-t2-jp TENCHU2.cue OUTPUT.png [--sequence ID] [--ticks N] [--buttons HEX]\nExperimental T1 stage mode: --gaisen GAISEN.cue --stage 0 --hero 0 --layout 1\nT1 scenery, doors, pitfalls and hot floors with T2 actors, AI, movement, HUD and results. Water supports native swimming/diving with added underwater collision (default depth 4000 game units; --water-depth 0–16000 in multiples of 10, 0 disables it). Original objectives and cutscenes retain T1 dialogue and models; Tatsumaru follows Rikimaru's story. Pickups supply their T2 equivalents. Requires both discs.\nF1 shows current native JP feature availability. Normal play-t2 and F6 also use native JP by default for Japanese discs."
        );
        return Ok(());
    }
    let mut sequence = 9;
    let mut seed = 1;
    let mut ticks = 30;
    let mut buttons = 0;
    let mut loadout = Vec::new();
    let mut gaisen = None;
    let mut stage = 0;
    let mut hero = 0;
    let mut layout = 1;
    let mut water_depth = tenchu_assets::tenchu2::collision::Gaisen::DEFAULT_WATER_DEPTH;
    let menu = args[first..].iter().any(|a| a == "--menu");
    let options: Vec<_> = args[first..].iter().filter(|a| *a != "--menu").collect();
    for pair in options.chunks(2) {
        if pair.len() != 2 {
            return Err("Native JP options require a value".into());
        }
        let value = pair[1].to_str().ok_or("Invalid native JP option")?;
        match pair[0].to_str() {
            Some("--gaisen") => gaisen = Some(Path::new(value).to_owned()),
            Some("--stage") => stage = value.parse::<usize>().map_err(|e| e.to_string())?,
            Some("--hero") => hero = value.parse::<usize>().map_err(|e| e.to_string())?,
            Some("--layout") => layout = value.parse::<usize>().map_err(|e| e.to_string())?,
            Some("--water-depth") => {
                water_depth = value.parse::<i16>().map_err(|e| e.to_string())?
            }
            Some("--sequence") => {
                sequence = value
                    .parse()
                    .map_err(|e: std::num::ParseIntError| e.to_string())?
            }
            Some("--seed") => {
                seed = value
                    .parse()
                    .map_err(|e: std::num::ParseIntError| e.to_string())?
            }
            Some("--ticks") => {
                ticks = value
                    .parse()
                    .map_err(|e: std::num::ParseIntError| e.to_string())?
            }
            Some("--buttons") => {
                buttons = u16::from_str_radix(value.trim_start_matches("0x"), 16)
                    .map_err(|e| e.to_string())?
            }
            Some("--loadout") => {
                for entry in value.split(',') {
                    let (item, count) = entry
                        .split_once(':')
                        .ok_or("Use --loadout ITEM:COUNT,ITEM:COUNT")?;
                    let item: usize = item
                        .parse()
                        .map_err(|e: std::num::ParseIntError| e.to_string())?;
                    let count: u8 = count
                        .parse()
                        .map_err(|e: std::num::ParseIntError| e.to_string())?;
                    if item > 20 {
                        return Err("Native JP inventory item must be 0–20".into());
                    }
                    loadout.push((item, count));
                }
            }
            _ => {
                return Err(format!(
                    "Unknown native JP option {}",
                    pair[0].to_string_lossy()
                ));
            }
        }
    }
    if menu && (capture || !loadout.is_empty() || options.chunks(2).any(|p| p[0] == "--sequence")) {
        return Err("Use --menu with play-t2-jp, without --sequence or --loadout".into());
    }
    let disc = crate::source::open_disc(args[1].as_ref())?;
    let settings = if !capture || std::env::var_os("TENCHU_SETTINGS").is_some() {
        crate::settings::Store::new()?.load()?
    } else {
        crate::settings::Settings::default()
    };
    if let Some(path) = gaisen {
        if menu
            || hero > 2
            || stage > 10
            || !(1..=3).contains(&layout)
            || options.chunks(2).any(|p| p[0] == "--sequence")
        {
            return Err("Use --gaisen CUE with --stage 0–10, --hero 0–2 and --layout 1–3, without --menu or --sequence".into());
        }
        let source = gaisen::Source::open(
            &path,
            stage,
            usize::from(hero == 1) as i16,
            layout,
            tenchu_assets::script::Language::English,
        )?;
        let mut items = [0; 21];
        items[0] = 255;
        for (item, count) in loadout {
            items[item] = count;
        }
        if capture {
            let alternate = items[19] != 0;
            items[19] = 0;
            let mut session = source.session(
                &disc,
                Options {
                    hero,
                    difficulty: 2,
                    seed,
                    inventory: Inventory::from_amounts(items),
                },
                alternate,
                water_depth,
            )?;
            for _ in 0..ticks {
                session.step(buttons)?;
            }
            return render::capture(&session, &settings, Path::new(&args[2]));
        }
        #[cfg(target_os = "android")]
        return Err("Use the Android launcher's F6 quick menu to start a stage".into());
        #[cfg(not(target_os = "android"))]
        return window::run(
            campaign::Playback::gaisen(disc, source, hero, 2, seed, Some(items), water_depth)?,
            settings,
        );
    } else if options.chunks(2).any(|p| {
        matches!(
            p[0].to_str(),
            Some("--stage" | "--hero" | "--layout" | "--water-depth")
        )
    }) {
        return Err("--stage, --hero, --layout and --water-depth require --gaisen CUE".into());
    }
    let campaign = tenchu_assets::tenchu2::campaign::Campaign::load(&disc)?;
    let descriptor = campaign
        .sequences
        .get(sequence)
        .ok_or("Missing Tenchu 2 sequence")?;
    if !capture {
        #[cfg(target_os = "android")]
        return Err("Use the Android launcher's F6 quick menu to start Japanese Tenchu 2".into());
        #[cfg(not(target_os = "android"))]
        {
            crate::tenchu2::menu::remember(
                &std::fs::canonicalize(&args[1]).map_err(|e| e.to_string())?,
            )?;
            let specified = !loadout.is_empty();
            let mut items = [0; 21];
            items[0] = 255;
            for (item, count) in loadout {
                items[item] = count;
            }
            return window::run(
                if menu {
                    campaign::Playback::menu(disc, 2, seed, None)?
                } else {
                    campaign::Playback::load(
                        disc,
                        sequence,
                        2,
                        seed,
                        specified.then_some(items),
                        None,
                    )?
                },
                settings,
            );
        }
    }
    if descriptor.kind == tenchu_assets::tenchu2::campaign::Kind::Event {
        return render::capture_story(
            &disc,
            sequence,
            descriptor.resource,
            ticks,
            seed,
            &settings.graphics,
            Path::new(&args[2]),
        );
    }
    let mut session = Session::load(&disc, sequence, seed)?;
    for (item, count) in loadout {
        let inventory = &mut session.world.actors[session.world.player].actor.inventory;
        if item == 20 {
            inventory.resurrections = count;
        } else {
            inventory.counts[item] = count;
        }
    }
    for _ in 0..ticks {
        session.step(buttons)?;
    }
    render::capture(&session, &settings, Path::new(&args[2]))
}
