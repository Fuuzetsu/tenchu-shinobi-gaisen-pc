use super::*;
use crate::{game::Game, oracle};
use std::fmt::Write;

pub(super) fn with_actor<T>(
    game: &mut Game,
    id: usize,
    f: impl FnOnce(&mut Actor, &mut Context, &mut ActionContext) -> T,
) -> T {
    let mut actor = game.roster.take(id);
    let mut ctx = Context {
        mission: &mut game.mission,
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
        selected_item: game.hud.selected,
        clock: game.tick as i32,
    };
    let result = f(&mut actor, &mut ctx, &mut world);
    game.roster.restore(actor);
    result
}

pub(super) fn arena(game: &mut Game, kind: i16) -> usize {
    oracle::flat_ground(game);
    let player = game.player_id;
    with_actor(game, player, |actor, ctx, world| {
        for id in world.roster.order.clone() {
            if id != player {
                world.roster.remove(id, ctx);
            }
        }
        for item in &mut world.items.slots {
            ctx.collision.remove(ctx.scene, item.model);
            item.processor = None;
        }
        actor.position = [0; 3];
        actor.rotation = [0, 2048, 0];
        actor.velocity = [0; 3];
        actor.map =
            terrain::vector(&ctx.mission.terrain, ctx.cursor, actor.position, 500, 0).unwrap();
        actor.enter(ctx, MOT_NORMAL, true);
        ctx.scene.update(actor.root_model, actor.root(ctx.math));
        let id = world.roster.spawn(kind, [0, 0, 4000], 0, ctx).unwrap();
        world.roster.get_mut(id).target = Some(actor.root_model);
        id
    })
}

pub(super) fn constants() -> String {
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
fn human_and_item_damage_match_original_c() {
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
        MOT_DAMAGE_MAKIBISHI,
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
    oracle::flat_ground(&mut game);
    let scene = game.scene.clone();
    let mut input = String::new();
    let mut expected = Vec::new();
    let mut generator = Random::new(0x9121);
    for case in 0..8192 {
        let seed = generator.next15() as u32;
        let health = [0, 1, 8, 20, 99, 100][case % 6];
        let kind = [0, 0x10, 0x87, 0x88, 0x8a, 0x81, 0x90, 0xa0][case % 8];
        let attribute = [0, 2, ATTR_WEAPON_DRAWN, ATTR_WEAPON_DRAWN | 2][case % 4];
        let item_kind = if case < 4096 {
            -1
        } else {
            [1, 2, 4, 6, 15, 20, 21, 22, 23, 0][case % 10]
        };
        let water = if case % 31 == 0 { 4 } else { 0 };
        let action = actions[case % if item_kind < 0 { 8 } else { actions.len() }];
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
        writeln!(input, "{seed} {health} {kind} {attribute} {action} {count} {loops} {height} {active_item} {power} {difficulty} {role} {enemy_kind} {enemy_item} {} {} {yaw} {dx} {dz} {direction} {item_kind} {water}", armor as i32, cleanup as i32).unwrap();
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
            MOT_ATTACK,
            &game.mission.characters[actor.definition].motions,
            &game.mission.common_motions,
            &game.mission.clips,
        );
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
        actor.map.attribute = water;
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
        game.collision.slots[hit].owner = (item_kind < 0).then_some(attacker_id);
        game.collision.distance = [dx as i16, 0, dz as i16];
        game.items.slots[0].model = enemy.parts[13];
        game.items.slots[0].kind = item_kind.max(0) as usize;
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
            which(game.combat.life_bars.last().map(|r| r.0)),
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
            i32::from(a.inventory[1]),
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

#[test]
#[ignore = "requires TENCHU_DISC; ranged animations launch items and cause damage through ordinary ticks"]
fn ranged_actors_hit_and_projectiles_retire() {
    use crate::items::Processor;
    for (clip, action, processor, damage) in [
        (Some(0xaa), MOT_ATTACK, Processor::Arrow, 10),
        (Some(0xab), MOT_ATTACK, Processor::Gun, 20),
        (Some(0xac), MOT_ATTACK, Processor::Gun, 20),
        (None, MOT_SYURI, Processor::Shuriken, 20),
    ] {
        let (mut game, kind) = (0..11)
            .find_map(|stage| {
                let game = oracle::game(stage, 0);
                let kind = game
                    .mission
                    .characters
                    .iter()
                    .find(|c| {
                        c.kind >= 0x10
                            && game.mission.models.contains_key(&c.kind)
                            && if let Some(clip) = clip {
                                c.motions
                                    .iter()
                                    .any(|m| m.action == action && m.clip == clip)
                            } else {
                                c.kind == 0x87
                            }
                    })
                    .map(|c| c.kind)?;
                Some((game, kind))
            })
            .expect("No ranged character found on the disc");
        let shooter = arena(&mut game, kind);
        with_actor(&mut game, shooter, |actor, ctx, _| {
            actor.attribute = ATTR_WEAPON_DRAWN;
            actor.think = [0; 4];
            actor.inventory[1] = 1;
            assert!(actor.enter(ctx, action, true));
            actor.map =
                terrain::vector(&ctx.mission.terrain, ctx.cursor, actor.position, 500, 0).unwrap();
            ctx.scene.update(actor.root_model, actor.root(ctx.math));
        });
        let health = game.player().health;
        let mut launched = false;
        let mut attached = false;
        for _ in 0..220 {
            game.step(0)
                .unwrap_or_else(|error| panic!("kind {kind:x}, tick {}: {error}", game.tick));
            launched |= game
                .items
                .slots
                .iter()
                .any(|i| i.processor == Some(processor));
            attached |= game.items.slots.iter().any(|i| {
                i.processor == Some(Processor::Arrow) && game.scene.models[i.model].parent.is_some()
            });
        }
        assert!(launched, "kind {kind:x} did not launch");
        assert_eq!(game.player().health, health - damage, "kind {kind:x}");
        if processor == Processor::Arrow {
            assert!(attached);
        }
        assert!(
            !game
                .items
                .slots
                .iter()
                .any(|i| i.processor == Some(processor)),
            "kind {kind:x} projectile did not retire"
        );
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; Manji's authored attack emits napalm that damages an actor through ordinary updates"]
fn manji_fire_reaches_the_shared_item_damage_path() {
    let mut game = oracle::game(3, 0);
    let shooter = arena(&mut game, 0x30);
    with_actor(&mut game, shooter, |actor, ctx, _| {
        actor.attribute = ATTR_WEAPON_DRAWN;
        actor.think = [0; 4];
        actor.position = [0, 0, 2300];
        assert!(actor.enter(ctx, MOT_ATTACK, true));
        assert_eq!(actor.motion_id(ctx, MOT_ATTACK), 0xf5);
        ctx.scene.update(actor.root_model, actor.root(ctx.math));
    });
    let mut launched = false;
    for _ in 0..140 {
        game.step(0).unwrap();
        launched |= game
            .items
            .slots
            .iter()
            .any(|i| i.processor == Some(crate::items::Processor::Napalm));
    }
    assert!(launched);
    assert!(game.player().health < game.player().max_health);
    assert!(
        !game
            .items
            .slots
            .iter()
            .any(|i| i.processor == Some(crate::items::Processor::Napalm))
    );
}

#[test]
#[ignore = "requires TENCHU_DISC; the original final-stage lava floor spawns damaging napalm without a special host path"]
fn retail_damage_floor_advances_through_napalm() {
    let mut game = oracle::game(7, 0);
    let player = game.player_id;
    with_actor(&mut game, player, |actor, ctx, world| {
        for id in world.roster.order.clone() {
            if id != player {
                world.roster.remove(id, ctx);
            }
        }
        actor.position = [58000, 2000, 164500];
        ctx.scene.update(actor.root_model, actor.root(ctx.math));
        ctx.camera.position = [58000, -1000, 160500];
        ctx.camera.reference = actor.position;
        ctx.camera.snap_pending = true;
    });
    let mut launched = 0;
    for _ in 0..100 {
        game.step(0).unwrap();
        launched += usize::from(
            game.items
                .slots
                .iter()
                .any(|i| i.processor == Some(crate::items::Processor::Napalm)),
        );
    }
    assert!(launched > 20);
    assert_ne!(game.player().map.attribute & 0x100, 0);
    assert!(game.player().health < game.player().max_health);
}

#[test]
#[ignore = "requires TENCHU_DISC; both characters release sleeping gas and put an armed guard to sleep without damage"]
fn sleeping_gas_releases_and_changes_guard_ai() {
    for character in 0..2 {
        let mut game = oracle::game(0, character);
        let enemy = arena(&mut game, 0x10);
        with_actor(&mut game, enemy, |actor, ctx, _| {
            actor.attribute = ATTR_WEAPON_DRAWN;
            actor.think = [0; 4];
            actor.enter(ctx, MOT_NORMAL, true);
        });
        game.camera.snap_pending = true;
        let health = game.roster.get(enemy).health;
        game.roster.get_mut(game.player_id).inventory[9] = 1;
        game.hud.cursor = 9;
        game.step(0).unwrap();
        game.step(crate::input::TRIANGLE).unwrap();
        assert_eq!(game.player().inventory[9], 0);
        assert_eq!(game.player().motion.action, MOT_ITEM_THROW);
        let mut released = false;
        let mut sleeping = false;
        let mut visible = false;
        for _ in 0..160 {
            game.step(0).unwrap();
            released |= game.items.slots.iter().any(|i| {
                i.processor == Some(crate::items::Processor::Sleep)
                    && i.mode == 2
                    && i.collision.size == 1000
            });
            visible |= game
                .items
                .drawings
                .iter()
                .any(|d| matches!(d.graphic, crate::effects::Graphic::Sprite { image: 6, .. }));
            let actor = game.roster.get(enemy);
            sleeping |= actor.think[0] == 7
                && actor.motion.action == MOT_ACTION
                && actor.attribute & (ATTR_WEAPON_DRAWN | ATTR_PHASE) == 0;
        }
        assert!(
            released && visible && sleeping,
            "character {character}: released {released}, visible {visible}, sleeping {sleeping}"
        );
        assert_eq!(game.roster.get(enemy).health, health);
        assert!(
            !game
                .items
                .slots
                .iter()
                .any(|i| i.processor == Some(crate::items::Processor::Sleep))
        );
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; quick/aimed throws, cancellation and interruption through ordinary player input"]
fn both_players_throw_aim_cancel_and_drop_interrupted_shuriken() {
    use crate::{camera::Mode, input::*, items::Processor};
    for character in 0..2 {
        for (path, look) in [(0, false), (1, false), (1, true), (2, false), (3, false)] {
            let mut game = oracle::game(0, character);
            game.roster.get_mut(game.player_id).inventory[1] = 3;
            game.hud.cursor = 1;
            game.step(0).unwrap();
            assert_eq!(game.hud.selected, Some(1));
            game.step(TRIANGLE).unwrap();
            assert_eq!(game.player().motion.action, MOT_SYURI);
            game.step(if path == 0 { 0 } else { TRIANGLE }).unwrap();
            assert_eq!(game.player().inventory[1], 2);
            if path != 0 {
                for _ in 0..20 {
                    game.step(TRIANGLE).unwrap();
                }
                assert_eq!(
                    (game.camera.mode, game.camera.old_mode),
                    (Mode::Direction, Mode::Sight as u8)
                );
                assert!(!game.items.sprites.is_empty());
                assert_eq!(game.player().inventory[25], 1);
                if path == 1 {
                    game.step(0).unwrap();
                    assert_eq!(game.camera.mode, Mode::Lock);
                } else if path == 2 {
                    game.step(TRIANGLE | SQUARE).unwrap();
                    assert_eq!(game.player().inventory[1], 3);
                    assert_eq!(game.camera.old_mode, Mode::Direction as u8);
                } else {
                    let player = game.player_id;
                    with_actor(&mut game, player, |actor, ctx, _| {
                        actor.enter(ctx, MOT_DAMAGE, true);
                    });
                    game.step(0).unwrap();
                    assert_eq!(game.player().inventory[1], 2);
                    assert!(
                        game.items
                            .slots
                            .iter()
                            .any(|i| i.kind == 1 && i.processor == Some(Processor::Pickup))
                    );
                }
            }
            assert_eq!(game.player().inventory[25], 0);
            assert!(
                !game
                    .items
                    .slots
                    .iter()
                    .any(|i| i.processor == Some(Processor::SightShot))
            );
            assert_eq!(
                game.items
                    .slots
                    .iter()
                    .any(|i| i.processor == Some(Processor::Shuriken)),
                path <= 1
            );
            for _ in 0..90 {
                game.step(if look { L1 } else { 0 }).unwrap();
            }
            assert!(!matches!(
                game.player().motion.action,
                MOT_SYURI | MOT_SYURI_RECOVER
            ));
            if path != 3 {
                assert_eq!(
                    game.camera.mode,
                    if look { Mode::Direction } else { Mode::Normal },
                    "character {character}, path {path}"
                );
            }
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; low-health enemy requests and drinks medicine through AI and item updates"]
fn injured_enemy_uses_its_medicine() {
    let mut game = oracle::game(0, 0);
    let enemy = arena(&mut game, 0x10);
    with_actor(&mut game, enemy, |actor, ctx, _| {
        actor.health = 1;
        actor.inventory.fill(0);
        actor.inventory[3] = 1;
        actor.attribute = ATTR_CUSTOMAI | ATTR_WEAPON_DRAWN | ATTR_SEARCH | 2;
        actor.think = [0, 0, 5, 0];
        actor.enter(ctx, MOT_ENGAGE_STANCE, true);
        ctx.scene.update(actor.root_model, actor.root(ctx.math));
    });
    let mut drank = false;
    for _ in 0..200 {
        game.step(0).unwrap();
        drank |= game.roster.get(enemy).motion.action == MOT_ITEM_DRINK;
        if game.roster.get(enemy).health == game.roster.get(enemy).max_health {
            break;
        }
    }
    assert!(drank);
    assert_eq!(
        game.roster.get(enemy).health,
        game.roster.get(enemy).max_health
    );
    assert_eq!(game.roster.get(enemy).inventory[3], 0);
}

#[test]
#[ignore = "requires TENCHU_DISC; ordinary hook aim, flight, pull and cancellation for both characters"]
fn both_players_grapple_and_cancel_through_normal_updates() {
    use crate::input::*;
    for character in 0..2 {
        for path in 0..3 {
            let mut game = oracle::game(0, character);
            game.roster.get_mut(game.player_id).inventory[0] = 255;
            game.step(0).unwrap();
            game.step(TRIANGLE).unwrap();
            for _ in 0..30 {
                game.step(TRIANGLE | if path == 0 { RIGHT } else { 0 })
                    .unwrap();
            }
            assert_eq!(game.player().motion.action, MOT_KAGI);
            assert_eq!(game.player().inventory[25], 1);
            assert!(!game.items.sprites.is_empty());
            game.step(if path == 2 { TRIANGLE | SQUARE } else { 0 })
                .unwrap();
            assert_eq!(game.player().inventory[25], 0);
            let mut fly = false;
            let mut pull = false;
            let mut wire = false;
            for _ in 0..220 {
                game.step(0).unwrap();
                fly |= game.player().motion.action == MOT_KAGI_FLY;
                pull |= game.player().motion.action == MOT_KAGI_PULL;
                wire |= !game.effects.wires.is_empty();
            }
            assert_eq!(game.player().inventory[0], 255);
            assert_ne!(game.player().status, 4);
            assert_eq!((fly, pull, wire), (path == 0, path == 0, path == 0));
        }
    }
}

mod charm;
mod darts;
mod decoy;
mod disguise;
mod dog;
mod explosives;
mod lightning;
mod poison;
mod revival;
mod speed;
mod swim;
mod wall;
