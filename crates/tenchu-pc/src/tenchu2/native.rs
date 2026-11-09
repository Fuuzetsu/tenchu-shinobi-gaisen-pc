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
#[cfg(not(target_arch = "wasm32"))]
mod equipment;
#[cfg(not(target_arch = "wasm32"))]
mod film;
mod inspection;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod models;
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
    pub bank: Arc<tenchu_assets::audio::bank::Bank>,
    pub pending: BTreeSet<String>,
    #[cfg(not(target_arch = "wasm32"))]
    pub models: models::Override,
}

impl Session {
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
        Ok(Self {
            world,
            assets,
            frame: Frame::default(),
            bank,
            pending: BTreeSet::new(),
            #[cfg(not(target_arch = "wasm32"))]
            models: Default::default(),
        })
    }

    pub fn step(&mut self, buttons: u16) -> Result<Vec<Sourced<Sound>>> {
        if matches!(
            self.world.mission.phase,
            tenchu_core::tenchu2::mission::Phase::Finished { .. }
        ) {
            return Ok(Vec::new());
        }
        let mut events = Output {
            player: self.world.player,
            #[cfg(not(target_arch = "wasm32"))]
            voice: self.models.voice.as_ref(),
            #[cfg(target_arch = "wasm32")]
            voice: None,
            listener: self.world.listener(),
            requests: Vec::new(),
            pending: &mut self.pending,
        };
        let tick = self.world.tick;
        self.frame = self
            .world
            .step([buttons, 0], &mut self.assets, &mut events)?;
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
        sounds.extend(
            self.world
                .advance_mission(&self.assets)?
                .into_iter()
                .map(Sourced::from),
        );
        self.frame.transition_fade = self.world.mission.fade;
        Ok(sounds)
    }
}

struct Output<'a> {
    player: usize,
    voice: Option<&'a Arc<VoiceSet>>,
    listener: Listener,
    requests: Vec<SoundRequest>,
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
            "tenchu-pc play-t2-jp TENCHU2.cue [--menu | --sequence ID] [--seed N] [--loadout 4:20]\ntenchu-pc render-t2-jp TENCHU2.cue OUTPUT.png [--sequence ID] [--ticks N] [--buttons HEX]\nStandalone native JP integration preview. Modern lighting and some effects are still being connected; F1 shows the current boundaries. Normal play-t2 and F6 also use native JP by default for Japanese discs."
        );
        return Ok(());
    }
    let mut sequence = 9;
    let mut seed = 1;
    let mut ticks = 30;
    let mut buttons = 0;
    let mut loadout = Vec::new();
    let menu = args[first..].iter().any(|a| a == "--menu");
    let options: Vec<_> = args[first..].iter().filter(|a| *a != "--menu").collect();
    for pair in options.chunks(2) {
        if pair.len() != 2 {
            return Err("Native JP options require a value".into());
        }
        let value = pair[1].to_str().ok_or("Invalid native JP option")?;
        match pair[0].to_str() {
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
    render::capture(&session, &settings.graphics, Path::new(&args[2]))
}
