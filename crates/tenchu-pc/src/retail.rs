mod compare;
mod observation;
pub mod tape;

use crate::source::DiscSource;
use observation::Reader;
use std::path::Path;
use tenchu_assets::{Result, game::GameAssets};
use tenchu_core::{drawing::Step, game::Game};

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

pub fn check(
    assets: &GameAssets<DiscSource>,
    path: &Path,
    limit: Option<u64>,
    export: Option<(&Path, &Path)>,
) -> Result<()> {
    let mut tape = tape::Tape::default();
    let mut pending_update = None;
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
            let mut initial = tape::Initial::capture(&row, &reader.ram)?;
            let mut native = initial.load(assets)?;
            initial.capture_sounds(&mut native, &reader.ram);
            tape.missions.push(tape::Mission {
                initial,
                entry_field: row.field,
                result_field: 0,
                result_end: 0,
                updates: Vec::new(),
            });
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
            pending_update = Some(tape::Update {
                field: row.field,
                input: row.pad as u16,
                skip: row.skip as u8,
                swap: false,
                clock: row.clock,
                seed: row.seed,
            });
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
            let mut update = pending_update.take().ok_or("Missing pending input")?;
            update.field = row.field;
            update.swap = row.epoch != epoch;
            tape.missions.last_mut().unwrap().updates.push(update);
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
    if let Some((archive, output)) = export {
        if limit.is_some() {
            return Err("Cannot export an incomplete validation".into());
        }
        tape.save(archive, path, output)?;
    }
    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "matched_updates": checked, "reached_end_of_recording": limit.is_none(), "missions": reports,
        "state_corrections_after_initialization": 0,
    })).map_err(|e| e.to_string())?);
    Ok(())
}
