mod compare;
mod observation;

use crate::source::DiscSource;
use observation::{Observation, Ram, Reader};
use std::path::Path;
use tenchu_assets::{Result, executable::Executable, game::GameAssets, stage::MissionAssets};
use tenchu_core::{drawing::Step, game::Game};

fn initial_game(assets: &GameAssets<DiscSource>, row: &Observation, ram: &Ram) -> Result<Game> {
    if row.clock != 0 {
        return Err("Replay must begin at the initial mission boundary".into());
    }
    let character = i16::from(ram.byte(0x80010004));
    let language = match ram.byte(0x8001005e) {
        0 => tenchu_assets::script::Language::English,
        1 => tenchu_assets::script::Language::French,
        2 => tenchu_assets::script::Language::Italian,
        3 => tenchu_assets::script::Language::Japanese,
        _ => return Err("Invalid recorded language".into()),
    };
    let inventory = ram.bytes(0x80010007, 20);
    let mission = MissionAssets::load_equipped(
        assets,
        row.stage as usize,
        character,
        usize::from(ram.byte(0x80010006)) + 1,
        language,
        inventory[19],
    )?;
    let mut game = Game::new(
        mission,
        &Executable::new(&assets.executable)?,
        character,
        row.seed,
    )?;
    game.random.seed = row.seed;
    game.intelligence.difficulty = match ram.byte(0x80010058) {
        0 => tenchu_core::ai::Difficulty::Easy,
        1 => tenchu_core::ai::Difficulty::Normal,
        2 => tenchu_core::ai::Difficulty::Hard,
        _ => return Err("Invalid recorded difficulty".into()),
    };
    game.combat.armor = inventory[19] != 0;
    game.roster
        .get_mut(game.player_id)
        .inventory
        .copy_from_slice(ram.bytes(ram.word(0x80097c60) + 0xb4, 26));
    game.scene.epoch = row.epoch;
    let terrain = ram.word(0x800976e4);
    for index in 0..game.roster.order.len() {
        let address = ram.word(0x800be7b8 + index as u32 * 4);
        let actor = game.roster.get_mut(game.roster.order[index]);
        // These cached terrain pointers retain the loader's search history.
        actor.map.area = ram.word(address + 0x30).wrapping_sub(terrain) as usize;
        actor.map.row = (ram.word(address + 0x34).wrapping_sub(terrain) / 16) as usize;
    }
    for (index, object) in game.misc.objects.iter_mut().enumerate() {
        if matches!(object.kind, tenchu_assets::misc::Kind::Sound { .. }) {
            object.count = ram.word(0x800c0958 + index as u32 * 36 + 24) as i32;
        }
    }
    Ok(game)
}

#[derive(serde::Serialize)]
struct MissionReport {
    mission: i32,
    stage: usize,
    entry_field: u32,
    initial_rng: u32,
    updates: u64,
    skipped: u64,
    completed: bool,
    result: Option<serde_json::Value>,
}

fn complete(game: &mut Game, report: &mut MissionReport) -> Result<()> {
    while game.cinematic.tail > 0 {
        game.step(0)?;
    }
    game.step(0)?;
    if game.events.outcome != tenchu_core::game::events::Outcome::Complete {
        return Err(format!(
            "Mission {} did not complete",
            game.mission.stage.order
        ));
    }
    report.completed = true;
    report.result = Some(serde_json::json!({
        "clock": game.clock, "rng": game.random.seed, "health": game.player().health,
        "score": game.score().total, "grade": game.score().grade,
        "found": game.intelligence.found, "murders": game.statistics.murders,
        "criticals": game.statistics.criticals, "friend_hits": game.statistics.friend_hits,
    }));
    eprintln!(
        "Mission {} completed; {} updates validated",
        report.mission, report.updates
    );
    Ok(())
}

pub fn check(assets: &GameAssets<DiscSource>, path: &Path, limit: Option<u64>) -> Result<()> {
    let mut reader = Reader::open(path)?;
    let mut game: Option<Game> = None;
    let mut checked = 0;
    let mut before_epoch = None;
    let mut reports: Vec<MissionReport> = Vec::new();
    while let Some(row) = reader.next()? {
        if row.pc == 0x8004df58
            && row.snapshot
            && game
                .as_ref()
                .is_none_or(|game| game.mission.stage.id != row.stage as usize)
        {
            if let Some(previous) = &mut game {
                complete(previous, reports.last_mut().unwrap())?;
            }
            let native = initial_game(assets, &row, &reader.ram)?;
            eprintln!(
                "Mission {} initialized at field {} seed {:08x}",
                native.mission.stage.order, row.field, row.seed
            );
            compare::state(&native, &reader.ram).map_err(|e| format!("Initial state: {e}"))?;
            reports.push(MissionReport {
                mission: native.mission.stage.order,
                stage: native.mission.stage.id,
                entry_field: row.field,
                initial_rng: row.seed,
                updates: 0,
                skipped: 0,
                completed: false,
                result: None,
            });
            game = Some(native);
        }
        let Some(native) = &mut game else {
            continue;
        };
        if row.pc == 0x800170c8 && matches!(row.caller, 0x800164d8 | 0x80050f78) {
            let step = match row.skip {
                0 => Step::Normal,
                1 => Step::Skipped,
                2 => Step::AfterLoad,
                _ => return Err("Invalid recorded draw cadence".into()),
            };
            while native.cinematic.tail > 0 {
                native.step(0)?;
            }
            if !native.begin_recorded_update(row.pad as u16, step)? {
                return Err(format!(
                    "Native simulation stopped at field {} (caller {:08x})",
                    row.field, row.caller
                ));
            }
            compare::state(native, &reader.ram)
                .map_err(|e| format!("Field {} clock {}: {e}", row.field, row.clock))?;
            before_epoch = Some(row.epoch);
            checked += 1;
            let report = reports.last_mut().unwrap();
            report.updates += 1;
            report.skipped += u64::from(step == Step::Skipped);
            if limit.is_some_and(|limit| checked >= limit) {
                break;
            }
        } else if row.pc == 0x80017234 && matches!(row.caller, 0x800164d8 | 0x80050f78) {
            let epoch = before_epoch
                .take()
                .ok_or("Display boundary has no preceding update")?;
            match row.epoch.wrapping_sub(epoch) {
                0 => native.finish_recorded_update(false)?,
                1 => native.finish_recorded_update(true)?,
                _ => return Err("Unexpected recorded display epoch".into()),
            }
        }
    }
    if checked == 0 {
        return Err("No gameplay updates validated".into());
    }
    if limit.is_none() {
        if before_epoch.is_some() {
            return Err("Recording ends during an update".into());
        }
        complete(game.as_mut().unwrap(), reports.last_mut().unwrap())?;
    }
    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "matched_updates": checked, "reached_end_of_recording": limit.is_none(), "missions": reports,
        "state_corrections_after_initialization": 0,
    })).map_err(|e| e.to_string())?);
    Ok(())
}
