use super::observation::{Observation, Ram};
use crate::{session::Options, source::DiscSource};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs::File, io::Write, path::Path};
use tenchu_assets::{
    Result, executable::Executable, game::GameAssets, script::Language, stage::MissionAssets,
};
use tenchu_core::{ai::Difficulty, drawing::Step, game::Game};

const FIELD_NUMERATOR: u64 = 125_703_417;
const FIELD_DENOMINATOR: u64 = 2_097_152;

pub fn video_frame(field: u32) -> u64 {
    (u64::from(field) * FIELD_DENOMINATOR * 60).div_ceil(FIELD_NUMERATOR)
}

pub fn source_field(frame: u64) -> f64 {
    frame as f64 * FIELD_NUMERATOR as f64 / (60 * FIELD_DENOMINATOR) as f64
}

#[derive(Serialize, Deserialize)]
pub struct Initial {
    stage: usize,
    character: i16,
    layout: usize,
    language: u8,
    difficulty: u8,
    seed: u32,
    epoch: u32,
    armor: u8,
    inventory: [u8; 26],
    terrain_caches: Vec<(usize, usize)>,
    sound_timers: Vec<(usize, i32)>,
}

impl Initial {
    pub(super) fn capture(row: &Observation, ram: &Ram) -> Result<Self> {
        if row.clock != 0 {
            return Err("Replay must begin at the initial mission boundary".into());
        }
        let terrain = ram.word(0x800976e4);
        let mut caches = Vec::new();
        // Actor count and sound-object layout are validated against the loaded assets below.
        let count = ram.short(0x80097724) as usize;
        if count > 256 {
            return Err("Invalid initial actor count".into());
        }
        for index in 0..count {
            let actor = ram.word(0x800be7b8 + index as u32 * 4);
            caches.push((
                ram.word(actor + 0x30).wrapping_sub(terrain) as usize,
                (ram.word(actor + 0x34).wrapping_sub(terrain) / 16) as usize,
            ));
        }
        Ok(Self {
            stage: row.stage as usize,
            character: i16::from(ram.byte(0x80010004)),
            layout: usize::from(ram.byte(0x80010006)) + 1,
            language: ram.byte(0x8001005e),
            difficulty: ram.byte(0x80010058),
            seed: row.seed,
            epoch: row.epoch,
            armor: ram.byte(0x80010007 + 19),
            inventory: ram
                .bytes(ram.word(0x80097c60) + 0xb4, 26)
                .try_into()
                .unwrap(),
            terrain_caches: caches,
            sound_timers: Vec::new(),
        })
    }

    pub fn options(&self, mission: usize) -> Result<Options> {
        Ok(Options {
            mission,
            character: self.character,
            layout: self.layout,
            language: Some(match self.language {
                0 => Language::English,
                1 => Language::French,
                2 => Language::Italian,
                3 => Language::Japanese,
                _ => return Err("Invalid recorded language".into()),
            }),
            difficulty: match self.difficulty {
                0 => Difficulty::Easy,
                1 => Difficulty::Normal,
                2 => Difficulty::Hard,
                _ => return Err("Invalid recorded difficulty".into()),
            },
        })
    }

    pub(super) fn capture_sounds(&mut self, game: &mut Game, ram: &Ram) {
        self.sound_timers = game
            .misc
            .objects
            .iter()
            .enumerate()
            .filter(|(_, object)| matches!(object.kind, tenchu_assets::misc::Kind::Sound { .. }))
            .map(|(index, _)| (index, ram.word(0x800c0958 + index as u32 * 36 + 24) as i32))
            .collect();
        for &(index, timer) in &self.sound_timers {
            game.misc.objects[index].count = timer;
        }
    }

    pub fn load(&self, assets: &GameAssets<DiscSource>) -> Result<Game> {
        let options = self.options(0)?;
        let mission = MissionAssets::load_equipped(
            assets,
            self.stage,
            self.character,
            self.layout,
            options.language.unwrap(),
            self.armor,
        )?;
        let mut game = Game::new(
            mission,
            &Executable::new(&assets.executable)?,
            self.character,
            self.seed,
        )?;
        game.random.seed = self.seed;
        game.intelligence.difficulty = options.difficulty;
        game.combat.armor = self.armor != 0;
        game.roster.get_mut(game.player_id).inventory = self.inventory;
        game.scene.epoch = self.epoch;
        if self.terrain_caches.len() != game.roster.order.len() {
            return Err("Initial actor count disagrees with mission".into());
        }
        for (index, &(area, row)) in self.terrain_caches.iter().enumerate() {
            let actor = game.roster.get_mut(game.roster.order[index]);
            actor.map.area = area;
            actor.map.row = row;
        }
        for &(index, timer) in &self.sound_timers {
            let object = game
                .misc
                .objects
                .get_mut(index)
                .ok_or("Invalid ambient sound index")?;
            if !matches!(object.kind, tenchu_assets::misc::Kind::Sound { .. }) {
                return Err("Invalid ambient sound timer".into());
            }
            object.count = timer;
        }
        Ok(game)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Update {
    pub field: u32,
    pub input: u16,
    pub skip: u8,
    pub swap: bool,
    pub clock: u32,
    pub seed: u32,
}

impl Update {
    pub fn apply(&self, game: &mut Game) -> Result<()> {
        while game.cinematic.tail > 0 {
            game.step(0)?;
        }
        let step = match self.skip {
            0 => Step::Normal,
            1 => Step::Skipped,
            2 => Step::AfterLoad,
            _ => return Err("Invalid recorded draw cadence".into()),
        };
        if !game.begin_recorded_update(self.input, step)? {
            return Err("Recorded update stopped early".into());
        }
        if game.clock as u32 != self.clock || game.random.seed != self.seed {
            return Err(format!(
                "Replay diverged at field {}: clock {}/{}, RNG {:08x}/{:08x}",
                self.field, game.clock, self.clock, game.random.seed, self.seed
            ));
        }
        game.finish_recorded_update(self.swap)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Mission {
    pub initial: Initial,
    pub entry_field: u32,
    pub result_field: u32,
    pub result_end: u32,
    pub updates: Vec<Update>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Tape {
    version: u32,
    pub fields: u32,
    source_inputs_sha256: String,
    source_updates_sha256: String,
    pub missions: Vec<Mission>,
}

impl Tape {
    pub(super) fn save(
        &mut self,
        archive: &Path,
        observations: &Path,
        output: &Path,
    ) -> Result<()> {
        let archive: serde_json::Value =
            serde_json::from_reader(File::open(archive).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        self.version = 1;
        self.fields = archive["fields"]
            .as_u64()
            .ok_or("Missing archived duration")?
            .try_into()
            .map_err(|_| "Invalid archived duration")?;
        self.source_inputs_sha256 = archive["input_sha256"]
            .as_str()
            .ok_or("Missing input hash")?
            .into();
        let mut hash = Sha256::new();
        std::io::copy(
            &mut File::open(observations).map_err(|e| e.to_string())?,
            &mut hash,
        )
        .map_err(|e| e.to_string())?;
        self.source_updates_sha256 = format!("{:x}", hash.finalize());
        let results: Vec<_> = archive["sections"]
            .as_array()
            .ok_or("Missing archived sections")?
            .iter()
            .filter(|s| !s["results"].is_null())
            .collect();
        if results.len() != self.missions.len() {
            return Err("Archived mission count disagrees with replay".into());
        }
        for (mission, result) in self.missions.iter_mut().zip(results) {
            mission.result_field = result["from_field"]
                .as_u64()
                .ok_or("Missing results field")? as u32;
            mission.result_end = mission.result_field
                + result["fields"]
                    .as_u64()
                    .ok_or("Missing results duration")? as u32;
        }
        self.validate()?;
        let data = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        let mut file = File::options()
            .write(true)
            .create_new(true)
            .open(output)
            .map_err(|e| e.to_string())?;
        file.write_all(&data).map_err(|e| e.to_string())
    }

    pub fn open(path: &Path) -> Result<Self> {
        let tape: Self = serde_json::from_reader(File::open(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        tape.validate()?;
        Ok(tape)
    }

    fn validate(&self) -> Result<()> {
        if self.version != 1 || self.fields == 0 || self.missions.is_empty() {
            return Err("Invalid retail replay".into());
        }
        let mut last = 0;
        for mission in &self.missions {
            if mission.entry_field <= last || mission.updates.is_empty() {
                return Err("Invalid mission boundary".into());
            }
            last = mission.entry_field;
            for update in &mission.updates {
                if update.field < last || update.skip > 2 {
                    return Err("Invalid update timeline".into());
                }
                last = update.field;
            }
            if last > mission.result_field || mission.result_end < mission.result_field {
                return Err("Invalid results boundary".into());
            }
            last = mission.result_end;
        }
        if last > self.fields {
            return Err("Recording ends before results".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires TENCHU_DISC and TENCHU_RETAIL_TAPE; replays every recorded mission without RAM corrections"]
    fn serialized_campaign_completes_with_every_rng_and_clock_check() {
        let path = std::env::var_os("TENCHU_RETAIL_TAPE").expect("TENCHU_RETAIL_TAPE");
        let disc = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
        let assets = GameAssets::open(crate::source::open_disc(Path::new(&disc)).unwrap()).unwrap();
        let tape = Tape::open(Path::new(&path)).unwrap();
        for mission in tape.missions {
            let mut game = mission.initial.load(&assets).unwrap();
            for update in mission.updates {
                update.apply(&mut game).unwrap();
            }
            while game.cinematic.tail > 0 {
                game.step(0).unwrap();
            }
            game.step(0).unwrap();
            assert_eq!(
                game.events.outcome,
                tenchu_core::game::events::Outcome::Complete
            );
        }
    }

    #[test]
    fn ntsc_fields_keep_their_duration_at_exact_sixty_fps() {
        assert_eq!(video_frame(88075), 88164);
        for field in [1, 3943, 12338, 87474, 88075] {
            let frame = video_frame(field);
            assert!(source_field(frame) >= f64::from(field));
            assert!(source_field(frame - 1) < f64::from(field));
        }
    }
}
