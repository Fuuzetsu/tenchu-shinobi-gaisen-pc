use std::{collections::BTreeSet, path::Path, sync::Arc};
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
        control::CombatCue,
        environment,
        inventory::Inventory,
        world::{
            Assets, Frame, Options, World,
            controls::{Event, EventSource, Events},
        },
    },
};

mod render;
#[cfg(test)]
mod tests;
#[cfg(not(target_arch = "wasm32"))]
mod window;

pub struct Session {
    pub world: World,
    pub assets: Assets,
    pub frame: Frame,
    pub bank: Arc<tenchu_assets::audio::bank::Bank>,
    pub pending: BTreeSet<String>,
}

impl Session {
    pub fn load(disc: &Disc<impl ReadAt>, sequence: usize, seed: u32) -> Result<Self> {
        let layout = Layout::available(disc)?
            .into_iter()
            .find(|l| l.sequence == sequence)
            .ok_or("No Japanese Tenchu 2 layout with that sequence")?;
        let hero = if sequence >= 185 {
            2
        } else {
            usize::from(sequence >= 101)
        };
        let assets = Assets::load(disc, hero, &layout)?;
        let mut inventory = Inventory::default();
        inventory.counts[0] = 255;
        let world = World::new(
            &layout,
            Options {
                hero,
                difficulty: 2,
                seed,
                inventory,
            },
            &assets,
        )?;
        let bank = Arc::new(Audio::load(disc, hero, layout.map)?.bank);
        Ok(Self {
            world,
            assets,
            frame: Frame::default(),
            bank,
            pending: BTreeSet::new(),
        })
    }

    pub fn step(&mut self, buttons: u16) -> Result<Vec<Sound>> {
        let mut events = Output {
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
                id: 29,
                listener: self.world.listener(),
                position: Some(position),
                volume: None,
            });
        }
        let mut sounds: Vec<_> = self.frame.ambience.into_iter().collect();
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
                sounds.push(sound);
            }
        }
        Ok(sounds)
    }
}

struct Output<'a> {
    listener: Listener,
    requests: Vec<SoundRequest>,
    pending: &'a mut BTreeSet<String>,
}

struct SoundRequest {
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
        let (id, position) = match event {
            Event::Voice(id) | Event::Combat(CombatCue::Voice(id)) => {
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
            "tenchu-pc play-t2-jp TENCHU2.cue [--sequence ID] [--seed N] [--loadout 4:20]\ntenchu-pc render-t2-jp TENCHU2.cue OUTPUT.png [--sequence ID] [--ticks N] [--buttons HEX]\nInternal native JP integration preview. Projectiles, story, HUD and campaign progression are still being connected; F1 shows the current boundaries. Existing play-t2 remains the Gaisen sandbox."
        );
        return Ok(());
    }
    let mut sequence = 9;
    let mut seed = 1;
    let mut ticks = 30;
    let mut buttons = 0;
    let mut loadout = Vec::new();
    for pair in args[first..].chunks(2) {
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
                    if item >= 20 {
                        return Err("Native JP inventory item must be 0–19".into());
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
    let disc = crate::source::open_disc(args[1].as_ref())?;
    let mut session = Session::load(&disc, sequence, seed)?;
    for (item, count) in loadout {
        session.world.actors[session.world.player]
            .actor
            .inventory
            .counts[item] = count;
    }
    let settings = if std::env::var_os("TENCHU_SETTINGS").is_some() {
        crate::settings::Store::new()?.load()?
    } else {
        crate::settings::Settings::default()
    };
    if capture {
        for _ in 0..ticks {
            session.step(buttons)?;
        }
        render::capture(&session, &settings.graphics, Path::new(&args[2]))
    } else {
        window::run(session, settings.graphics)
    }
}
