use super::*;
use crate::input::*;

fn wall_position(game: &mut Game) -> ([i32; 3], i16) {
    let width = i32::from(game.mission.characters[game.player().definition].width);
    let mut positions = Vec::new();
    for row in &game.mission.terrain.rows {
        for i in 0..row.count {
            let node = game.mission.terrain.node(row.list + i * 16).unwrap();
            if node.attributes & 0xc304 != 0 {
                continue;
            }
            let [x1, z1, x2, z2] = node.bounds.map(|v| i32::from(v) * 10);
            for x in [x1 + width + 50, x2 - width - 50, (x1 + x2) / 2] {
                for z in [z1 + width + 50, z2 - width - 50, (z1 + z2) / 2] {
                    positions.push([x, i32::from(node.y) * 10, z]);
                }
            }
        }
    }
    let start = game.player().position;
    positions
        .sort_by_key(|p| (i64::from(p[0] - start[0])).pow(2) + (i64::from(p[2] - start[2])).pow(2));
    for position in positions {
        let body = terrain::vector(
            &game.mission.terrain,
            &mut game.cursor,
            position,
            width,
            terrain::STEP_DOWN,
        )
        .unwrap();
        let wall = terrain::vector(
            &game.mission.terrain,
            &mut game.cursor,
            position,
            width + 100,
            terrain::STEP_DOWN | terrain::ALLOW_DEEP,
        )
        .unwrap();
        let angle = game.body.reflect_angle[wall.vector as usize];
        if body.height == 0
            && body.vector == 0
            && body.attribute & 0xc304 == 0
            && wall.attribute & 0xc000 == 0
            && angle >= 0
            && angle & 512 == 0
        {
            return (position, angle);
        }
    }
    panic!("No wall position in Mission 1")
}

fn at_wall(game: &mut Game) -> i16 {
    let (position, wall) = wall_position(game);
    println!("Mission 1 wall fixture: {position:?}, wall {wall}");
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
        actor.position = position;
        actor.velocity = [0; 3];
        actor.rotation = [0, (wall + 2048) & 4095, 0];
        actor.map = terrain::vector(
            &ctx.mission.terrain,
            ctx.cursor,
            position,
            500,
            terrain::STEP_DOWN,
        )
        .unwrap();
        actor.enter(ctx, MOT_NORMAL, true);
        ctx.scene.update(actor.root_model, actor.root(ctx.math));
    });
    game.camera.snap_pending = true;
    for _ in 0..35 {
        game.step(CIRCLE).unwrap();
    }
    assert_eq!(
        game.player().motion.action,
        MOT_STICKON,
        "position {position:?}, wall {wall}"
    );
    assert_eq!(game.player().motion.loops, -1);
    wall
}

#[test]
#[ignore = "requires TENCHU_DISC; both players hug and slide along actual Mission 1 geometry"]
fn wall_hug_slide_and_release_through_input() {
    for character in 0..2 {
        let mut game = oracle::game(0, character);
        let wall = at_wall(&mut game);
        let player = game.player();
        let clip = &game.mission.clips[player.motion.clip.unwrap()];
        assert_eq!(clip.bones[0][0].value[1], 2048);
        assert_eq!(player.motion.pose.rotation[0][1], 2048);
        assert_eq!(
            (player.rotation[1] + player.motion.pose.rotation[0][1]) & 4095,
            wall
        );
        let start = player.position;
        let mut slid = false;
        for _ in 0..20 {
            game.step(CIRCLE | LEFT).unwrap();
            slid |= matches!(
                game.player().motion.action,
                MOT_STICKON_SLIDE_L | MOT_STICKON_SLIDE_R
            );
        }
        assert!(slid);
        assert_ne!(game.player().position, start);
        game.step(0).unwrap();
        assert!(matches!(
            game.player().motion.action,
            MOT_NORMAL | MOT_ENGAGE_STANCE
        ));
        assert_eq!(game.player().motion.mask, 0x7fff);
        assert_eq!(game.camera.mode, crate::camera::Mode::Normal);
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; corner throws keep their latched item after selection changes and crouch release"]
fn corner_throws_keep_selection_and_byte_decrement() {
    use crate::{camera::Mode, items::Processor};
    for character in 0..2 {
        for kind in [2, 4, 5, 7] {
            let mut game = oracle::game(0, character);
            at_wall(&mut game);
            let player = game.player_id;
            game.roster.get_mut(player).inventory[kind] = 2;
            game.hud.selected = Some(kind);
            with_actor(&mut game, player, |actor, ctx, world| {
                ctx.camera.mode = if kind == 5 {
                    Mode::StickRight
                } else {
                    Mode::StickLeft
                };
                actor.pad.update(CIRCLE | TRIANGLE);
                actor.control(ctx, &[], [0; 3], world).unwrap();
            });
            assert_eq!(game.items.wall_item, Some(kind));
            assert_eq!(
                game.player().motion.action,
                if kind == 5 {
                    MOT_STICKON_THROW_R
                } else {
                    MOT_STICKON_THROW_L
                }
            );
            game.hud.selected = Some(3);
            let before = if kind == 2 { 255 } else { 0 };
            game.roster.get_mut(player).inventory[kind] = before;
            let processor = match kind {
                2 => Processor::Caltrops,
                4 => Processor::Fire,
                7 => Processor::Poison,
                _ => Processor::Smoke,
            };
            for _ in 0..100 {
                game.step(0).unwrap();
                if game
                    .items
                    .slots
                    .iter()
                    .any(|i| i.processor == Some(processor))
                {
                    break;
                }
                assert!(matches!(
                    game.player().motion.action,
                    MOT_STICKON_THROW_L | MOT_STICKON_THROW_R
                ));
            }
            let thrown: Vec<_> = game
                .items
                .slots
                .iter()
                .filter(|i| i.processor == Some(processor))
                .collect();
            assert_eq!(thrown.len(), if kind == 2 { 5 } else { 1 });
            assert!(thrown.iter().all(|i| i.kind == kind));
            assert_eq!(game.player().inventory[kind], before.wrapping_sub(1));
            assert_eq!(game.player().motion.action, MOT_STICKON);
            assert_eq!(game.player().motion.mask, 0x7fff);
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; missing-wall probes preserve the captured retail null-map branch"]
fn leaving_a_wall_retains_or_reverts_the_slide_by_direction() {
    for held in [DOWN, LEFT] {
        let mut game = oracle::game(0, 0);
        arena(&mut game, 0x10);
        let player = game.player_id;
        with_actor(&mut game, player, |actor, ctx, world| {
            actor.rotation = [0; 3];
            actor.enter(ctx, MOT_STICKON_SLIDE_L, true);
            actor.motion.count = 1;
            actor.motion.pose.rotation[0] = [-105, 2048, 0];
            actor.motion.mask = !1;
            actor.pad.update(held | CIRCLE);
            actor.control(ctx, &[], [0; 3], world).unwrap();
        });
        if held == DOWN {
            assert_eq!(game.player().position, [0, 0, 30]);
            assert!(matches!(
                game.player().motion.action,
                MOT_NORMAL | MOT_ENGAGE_STANCE
            ));
        } else {
            assert_eq!(game.player().position, [0; 3]);
            assert_eq!(game.player().motion.action, MOT_STICKON);
            assert_eq!(game.player().motion.loops, -1);
        }
        assert_eq!(game.player().motion.mask, 0x7fff);
        assert_eq!(game.player().velocity, [0; 3]);
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; sustained pressure against Hanbe's straight wall must not turn into diagonal recovery or trap the player"]
fn straight_wall_pressure_keeps_ground_contact_and_allows_retreat() {
    for character in 0..2 {
        let mut game = oracle::game(0, character);
        let player = game.player_id;
        with_actor(&mut game, player, |actor, ctx, _| {
            actor.position = [0, -500, 9000];
            actor.previous = actor.position;
            actor.rotation = [0, 1024, 0];
            actor.velocity = [0; 3];
            actor.enter(ctx, MOT_ENGAGE_STANCE, true);
            ctx.scene.update(actor.root_model, actor.root(ctx.math));
        });
        for _ in 0..100 {
            game.step(UP).unwrap();
            assert_eq!(game.player().position[1], -500);
            assert_eq!(game.player().map.height, 0);
            assert_eq!(game.player().rotation[1], 1024);
            assert!(game.player().position[0] > -2000);
        }
        assert!(game.player().position[0] < -1000);
        let wall_x = game.player().position[0];
        for _ in 0..10 {
            game.step(0).unwrap();
        }
        for _ in 0..60 {
            game.step(DOWN).unwrap();
        }
        assert!(game.player().position[0] - wall_x > 1000);
        assert_eq!(game.player().position[1], -500);
        assert_eq!(game.player().map.vector, 0);
    }
}
