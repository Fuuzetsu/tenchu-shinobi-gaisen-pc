use super::*;
use crate::{game::Game, oracle};
use std::fmt::Write;

fn with_actor<T>(
    game: &mut Game,
    id: usize,
    f: impl FnOnce(&mut Actor, &mut Context, &mut ActionContext) -> T,
) -> T {
    let mut actor = game.roster.take(id);
    let mut ctx = Context {
        mission: &game.mission,
        math: &game.math,
        body: &game.body,
        cursor: &mut game.cursor,
        random: &mut game.random,
        effects: &mut game.effects,
        scene: &mut game.scene,
        collision: &mut game.collision,
        camera: &mut game.camera,
        drawing: &mut game.drawing,
    };
    let mut world = ActionContext {
        roster: &mut game.roster,
        intelligence: &mut game.intelligence,
        statistics: &mut game.statistics,
        combat: &mut game.combat,
        items: &mut game.items,
        player: game.player_id,
        clock: game.tick as i32,
    };
    let result = f(&mut actor, &mut ctx, &mut world);
    game.roster.restore(actor);
    result
}

fn constants() -> String {
    let mut constants = String::new();
    for file in [
        "game_types.h",
        "sound.h",
        "humanoid.h",
        "model.h",
        "tuning.h",
        "action.h",
        "appear.h",
    ] {
        for line in oracle::source(file).lines() {
            let line = line.trim();
            let pair = if let Some(line) = line.strip_prefix("#define ") {
                line.split_once(char::is_whitespace)
            } else {
                line.split_once('=')
            };
            let Some((name, value)) = pair else {
                continue;
            };
            let name = name.trim();
            if name.is_empty()
                || !name
                    .bytes()
                    .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
            {
                continue;
            }
            let value = value
                .split([',', '/', ' '])
                .find(|v| !v.is_empty())
                .unwrap_or("");
            let number = if let Some(hex) = value.strip_prefix("0x") {
                i64::from_str_radix(hex, 16)
            } else {
                value.parse()
            };
            if let Ok(number) = number {
                writeln!(constants, "#ifndef {name}\n#define {name} {number}\n#endif").unwrap();
            }
        }
    }
    constants
}

#[test]
#[ignore = "requires TENCHU_DISC; preserves the weapon-clash collision-index bug"]
fn weapon_clash_uses_collision_index_for_stun_and_keeps_the_slot() {
    let mut game = oracle::game(0, 0);
    let player = game.player_id;
    let enemy = game
        .roster
        .order
        .iter()
        .copied()
        .find(|&id| id != player)
        .unwrap();
    for model in &mut game.scene.models {
        model.conflict = None;
    }
    game.collision = Collision::default();
    let hand = game.player().parts[13];
    let other = game.roster.get(enemy).parts[0];
    let own_slot = game.collision.insert(&mut game.scene, hand).unwrap();
    game.collision.slots[own_slot].owner = Some(player);
    let hit = game.collision.insert(&mut game.scene, other).unwrap();
    game.collision.slots[hit].owner = Some(enemy);
    game.collision.slots[hit].class = crate::collision::HIT;
    game.collision.slots[own_slot].results[hit] = 0x81;
    game.collision.slots[own_slot].count = 1;
    game.scene.models[hand].attribute |= crate::scene::CONFLICT;
    let attack = with_actor(&mut game, player, |a, ctx, _| a.attack_id(ctx, MOT_ATTACK));
    assert_ne!(attack, hit);
    game.mission.weapons.attacks[hit].power = 27;
    game.mission.weapons.attacks[attack].power = 60;
    let start = game.mission.weapons.attacks[attack].start;
    assert!(start + 1 < game.mission.weapons.attacks[attack].end);
    let before = game.random.calls;
    with_actor(&mut game, player, |actor, ctx, world| {
        actor.enter(ctx, MOT_ATTACK, true);
        actor.attack = attack;
        actor.motion.count = start + 1;
        actor.target = None;
        actor.pad.trigger = 0;
        actor.attack(ctx, world, 0).unwrap();
    });
    assert_eq!(game.player().motion.loops, -10);
    assert_eq!(game.random.calls - before, 40);
    assert_eq!(game.collision.len, 2);
    assert_eq!(game.scene.models[hand].conflict, Some(own_slot));
    assert_eq!(game.scene.models[hand].attribute & crate::scene::COLLIDE, 0);
    assert_eq!(game.collision.slots[own_slot].results[hit], 0xc1);
    assert_eq!(game.intelligence.voices, [(player, 0x36)]);
}

#[test]
#[ignore = "requires TENCHU_DISC; six complete authored stealth deaths through the ordinary actor loop"]
fn both_characters_finish_each_stealth_pair_and_leave_a_corpse() {
    for character in 0..=1 {
        for variant in 0..3 {
            let mut game = oracle::game(0, character);
            let player = game.player_id;
            let victim = game
                .roster
                .order
                .iter()
                .copied()
                .find(|&id| (0x10..0x80).contains(&game.roster.get(id).kind))
                .unwrap();
            game.roster.order.retain(|&id| id == player || id == victim);
            let slots: Vec<_> = game
                .roster
                .order
                .iter()
                .map(|&id| {
                    let model = game.roster.get(id).parts[0];
                    game.collision.slots[game.scene.models[model].conflict.unwrap()].clone()
                })
                .collect();
            for model in &mut game.scene.models {
                model.conflict = None;
            }
            game.collision = Collision::default();
            for slot in slots {
                let id = game
                    .collision
                    .insert(&mut game.scene, slot.model.unwrap())
                    .unwrap();
                game.collision.slots[id] = slot;
            }
            with_actor(&mut game, player, |actor, ctx, _| {
                actor.rotation = [0; 3];
                actor.enter(ctx, MOT_ENGAGE_STANCE, true);
                ctx.scene.update(actor.root_model, actor.root(ctx.math));
            });
            let mut position = game.player().position;
            if variant == 2 {
                position[0] += 700;
            } else {
                position[2] -= 700;
            }
            with_actor(&mut game, victim, |actor, ctx, _| {
                actor.position = position;
                actor.rotation = [0, if variant == 1 { 2048 } else { 0 }, 0];
                actor.attribute = 0;
                actor.health = actor.max_health;
                actor.inventory.fill(0);
                actor.enter(ctx, MOT_NORMAL, true);
                ctx.scene.update(actor.root_model, actor.root(ctx.math));
            });
            game.step(crate::input::SQUARE).unwrap();
            let variant = variant + character * 3;
            assert_eq!(
                game.player().motion.action,
                MOT_ATTACK_STEALTH_BACK + variant,
                "character {character}, variant {variant}"
            );
            assert_eq!(
                game.roster.get(victim).motion.action,
                MOT_DEAD_STEALTH_BACK + variant
            );
            let mut critical_camera = false;
            let mut gore = false;
            let mut blood_pool = false;
            for _ in 0..250 {
                game.step(0).unwrap_or_else(|e| {
                    panic!(
                        "character {character}, variant {variant}, tick {}: {e}",
                        game.tick
                    )
                });
                critical_camera |= game.camera.mode == crate::camera::Mode::Critical;
                gore |= game
                    .effects
                    .slots
                    .iter()
                    .flatten()
                    .any(|e| matches!(e, crate::effects::Effect::Blood(b) if b.gore));
                blood_pool |= !game.effects.blood_pools.is_empty();
            }
            let expects_gore = game.mission.death_events[variant as usize]
                .iter()
                .any(|event| matches!(event.action, 3 | 4) && event.payload[0] != -1);
            assert!(
                critical_camera && blood_pool,
                "character {character}, variant {variant}"
            );
            assert_eq!(
                gore, expects_gore,
                "character {character}, variant {variant}"
            );
            assert_eq!(game.statistics.criticals, 1);
            assert_eq!(game.intelligence.action_halt, 0);
            assert_eq!(game.player().status, 5);
            let dead = game.roster.get(victim);
            assert_eq!((dead.health, dead.status, dead.motion.loops), (0, 17, -1));
            assert!(game.scene.models[dead.parts[0]].conflict.is_none());
            assert_eq!(game.collision.len, 1);
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC, TENCHU_DECOMP and a C compiler; original DamageControl"]
fn human_damage_decisions_match_original_c() {
    let mut game = oracle::game(0, 0);
    let template = game.player().clone();
    let id = template.id;
    let attacker_id = game
        .roster
        .order
        .iter()
        .copied()
        .find(|&other| other != id)
        .unwrap();
    let other_id = game
        .roster
        .order
        .iter()
        .copied()
        .find(|&other| other != id && other != attacker_id)
        .unwrap();
    let mut attacker = template.clone();
    attacker.id = attacker_id;
    attacker.root_model = game.scene.add(Some(0), Matrix::default());
    attacker.parts = attacker
        .motion
        .pose
        .bind(&mut game.scene, attacker.root_model, &game.math);
    attacker.motion.action = MOT_ATTACK;
    attacker.status = 7;
    attacker.velocity = [99, 8, 88];
    let mut rows = String::new();
    let actions = [
        MOT_ATTACK,
        MOT_ENGAGE,
        MOT_CHASE_BACK,
        MOT_NORMAL,
        MOT_DAMAGE_GETUP,
        MOT_DAMAGE_DOWNED,
        MOT_SWIM,
        MOT_SWIM_STROKE,
        MOT_DAMAGE,
        MOT_DAMAGE_FRONT_MID,
        MOT_DAMAGE_FRONT_HEAVY,
        MOT_DAMAGE_BACK_LIGHT,
        MOT_DAMAGE_BACK_HEAVY,
        MOT_DAMAGE_LAUNCH_BACK,
        MOT_DAMAGE_LAUNCH_FORE,
        MOT_DEAD,
        MOT_DEAD_ALT,
        MOT_DEAD_DROWN,
    ];
    for &action in &actions {
        let mut motion = template.motion.clone();
        if motion.change(
            action,
            &game.mission.characters[template.definition].motions,
            &game.mission.common_motions,
            &game.mission.clips,
        ) == Change::Missing
        {
            continue;
        }
        let clip = &game.mission.clips[motion.clip.unwrap()];
        writeln!(
            rows,
            "{{{action}, {}, {}, {}, {}}},",
            clip.sweep, clip.forward_speed, clip.side_speed, clip.duration
        )
        .unwrap();
    }
    let mut tables = format!(
        "#define WEAPON_KIND {}\nstatic MotionDataType Motions[] = {{{rows}}};\n#define N_MOTIONS (sizeof(Motions)/sizeof(Motions[0]))\nstatic short Trig[4096][2] = {{",
        game.mission.characters[template.definition].weapon
    );
    for [s, c] in &game.math.tables.trig {
        write!(tables, "{{{s},{c}}},").unwrap();
    }
    tables.push_str("};\n");
    let source = oracle::source("MOTION.C");
    let mut routines =
        oracle::function(&oracle::source("CONFLICT.C"), "void DeleteConflict(").to_string();
    for signature in [
        "static inline void ClearAttackEffects(",
        "static inline void RecordPlayerKill(",
        "static inline void RecoilAttacker(",
        "static inline void RequestDamageFeedback(",
        "static inline void PlayRandomHurtVoice(",
        "static inline void SnapToWaistConflict(",
        "void DamageControl(void)\n{",
    ] {
        routines.push_str(oracle::function(&source, signature));
        routines.push('\n');
    }
    let harness = constants()
        + &include_str!("tests/damage.c")
            .replace(
                "static void MoveHumanoid(",
                &(tables + "static void MoveHumanoid("),
            )
            .replace(
                "/* ORIGINAL_ROUTINES */",
                &(String::from("static void AttackCancelControl(s16 mode);\n") + &routines),
            );
    // A broad, flat ground patch makes every tested source-to-victim passage open.
    let mut bytes = vec![0u8; 48];
    bytes[2..4].copy_from_slice(&1i16.to_le_bytes());
    bytes[4..8].copy_from_slice(&32u32.to_le_bytes());
    for base in [8, 36] {
        for (i, v) in [-3000i16, -3000, 3000, 3000].into_iter().enumerate() {
            bytes[base + i * 2..base + i * 2 + 2].copy_from_slice(&v.to_le_bytes());
        }
    }
    bytes[44..46].copy_from_slice(&0x2000u16.to_le_bytes());
    bytes[46..48].copy_from_slice(&u16::MAX.to_le_bytes());
    game.mission.terrain = tenchu_assets::terrain::AreaMap::parse(bytes).unwrap();
    let scene = game.scene.clone();
    let mut input = String::new();
    let mut expected = Vec::new();
    let mut generator = Random::new(0x9121);
    for case in 0..4096 {
        let seed = generator.next15() as u32;
        let health = [0, 1, 8, 20, 99, 100][case % 6];
        let kind = [0, 0x10, 0x87, 0x88, 0x8a, 0x81, 0x90, 0xa0][case % 8];
        let attribute = [0, 2, ATTR_WEAPON_DRAWN, ATTR_WEAPON_DRAWN | 2][case % 4];
        let action = actions[case % 8];
        let count = case as i16 % 21;
        let loops = if case % 13 == 0 { -1 } else { 0 };
        let height = if case % 11 == 0 { 100 } else { 0 };
        let active_item = if case % 17 == 0 { 12 } else { 0 };
        let power = [1, 7, 8, 15, 16, 23, 24, 31, 45, 100][case % 10];
        let difficulty = case % 3;
        let role = (case / 3) % 3;
        let enemy_kind = if case % 19 == 0 { 0xa9 } else { 0 };
        let enemy_item = if case % 23 == 0 { 12 } else { 0 };
        let armor = case % 29 == 0;
        let cleanup = case % 5 == 0;
        let yaw = generator.next15() as i16;
        let dx = generator.next15() % 4000 - 2000;
        let dz = generator.next15() % 4000 - 2000;
        let direction = game.math.direction(dx, dz, yaw).unwrap();
        writeln!(input, "{seed} {health} {kind} {attribute} {action} {count} {loops} {height} {active_item} {power} {difficulty} {role} {enemy_kind} {enemy_item} {} {} {yaw} {dx} {dz} {direction}", armor as i32, cleanup as i32).unwrap();
        game.scene = scene.clone();
        game.collision = Collision::default();
        game.random = Random::new(seed);
        game.effects = crate::effects::Effects::default();
        game.intelligence = crate::ai::Intelligence::default();
        game.statistics = crate::game::Statistics::default();
        game.combat = Combat::default();
        game.intelligence.difficulty = [
            crate::ai::Difficulty::Easy,
            crate::ai::Difficulty::Normal,
            crate::ai::Difficulty::Hard,
        ][difficulty];
        game.player_id = [id, attacker_id, other_id][role];
        game.combat.armor = armor;
        game.cursor = Cursor::new(&game.mission.terrain);
        let mut actor = template.clone();
        actor.health = health;
        actor.max_health = 100;
        actor.kind = kind;
        actor.attribute = attribute;
        actor.motion.change(
            action,
            &game.mission.characters[actor.definition].motions,
            &game.mission.common_motions,
            &game.mission.clips,
        );
        actor.motion.action = action;
        actor.motion.count = count;
        actor.motion.loops = loops;
        actor.motion.mask = 0x1234;
        actor.status = (action >> 8) as i8 as i16;
        actor.position = [0; 3];
        actor.rotation = [0, yaw, 0];
        actor.velocity = [21, 7, -31];
        actor.map.height = height;
        actor.active_item = active_item;
        actor.pad.time = 19;
        let mut enemy = attacker.clone();
        enemy.kind = enemy_kind;
        enemy.active_item = enemy_item;
        enemy.position = [dx, 0, dz];
        for (model, owner) in [(actor.parts[0], id), (enemy.parts[0], attacker_id)] {
            game.scene.models[model].conflict = None;
            let slot = game.collision.insert(&mut game.scene, model).unwrap();
            game.collision.slots[slot].owner = Some(owner);
        }
        if cleanup {
            for &model in &actor.parts[13..=14] {
                let slot = game.collision.insert(&mut game.scene, model).unwrap();
                game.collision.slots[slot].owner = Some(id);
            }
        }
        let hit = game
            .collision
            .insert(&mut game.scene, enemy.parts[13])
            .unwrap();
        game.collision.slots[hit].owner = Some(attacker_id);
        actor.hit = Some(hit);
        game.roster.actors[id] = Some(actor);
        game.roster.actors[attacker_id] = Some(enemy);
        let attack_index = with_actor(&mut game, attacker_id, |a, ctx, _| {
            a.attack_id(ctx, MOT_ATTACK)
        });
        game.mission.weapons.attacks[attack_index].power = power;
        with_actor(&mut game, id, |a, ctx, w| a.damage_control(ctx, w)).unwrap();
        let a = game.roster.get(id);
        let e = game.roster.get(attacker_id);
        let which = |who: Option<usize>| {
            who.map_or(-1, |v| {
                if v == id {
                    0
                } else if v == attacker_id {
                    1
                } else {
                    2
                }
            })
        };
        let effects = |pred: fn(&crate::effects::Effect) -> bool| {
            game.effects
                .slots
                .iter()
                .flatten()
                .filter(|e| pred(e))
                .count() as i32
        };
        let mut row = vec![
            a.health.into(),
            a.status.into(),
            a.motion.action.into(),
            a.motion.count.into(),
            a.motion.loops.into(),
            a.velocity[0].into(),
            a.velocity[1].into(),
            a.velocity[2].into(),
            a.rotation[1].into(),
            e.motion.loops.into(),
            e.velocity[0].into(),
            e.velocity[2].into(),
            game.random.calls as i32,
            game.intelligence.alert,
            which(game.combat.life_bar),
            game.statistics.criticals.into(),
            game.statistics.murders.into(),
            game.statistics.friend_hits.into(),
            game.collision.len as i32,
            which(game.combat.death_sound),
            a.pad.time.into(),
            a.motion.mask as i32,
            effects(|e| matches!(e, crate::effects::Effect::Blood(_))),
            effects(|e| matches!(e, crate::effects::Effect::Bleed(_))),
            effects(|e| matches!(e, crate::effects::Effect::Impact(_))),
            game.intelligence.voices.len() as i32,
        ];
        row.extend(
            game.intelligence
                .voices
                .iter()
                .map(|&(who, sound)| which(Some(who)) * 256 + i32::from(sound)),
        );
        expected.push(row);
    }
    let actual = oracle::run(&harness, &input);
    assert_eq!(actual.len(), expected.len());
    for (i, (c, rust)) in actual.iter().zip(&expected).enumerate() {
        assert_eq!(
            rust,
            c,
            "damage case {i}: {}",
            input.lines().nth(i).unwrap()
        );
    }
}
