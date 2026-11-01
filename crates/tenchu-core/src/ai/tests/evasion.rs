use super::*;
use tenchu_assets::terrain::{AreaMap, Node};

#[test]
#[ignore = "requires TENCHU_DISC; optional shuriken evasion uses actual enemy motions and respects aim, cover, recovery and cooldown"]
fn quick_shuriken_evasion_respects_combat_and_cover() {
    let mut kinds = std::collections::BTreeSet::new();
    for stage in [0, 7] {
        let mut game = oracle::game(stage, 0);
        let mut guards: Vec<_> = game
            .roster
            .actors
            .iter()
            .flatten()
            .filter(|a| (0x10..0x90).contains(&a.kind) && a.health > 0)
            .cloned()
            .collect();
        guards.sort_by_key(|a| a.kind);
        guards.dedup_by_key(|a| a.kind);
        let player = Player {
            id: game.player_id,
            root: game.player().root_model,
            position: Position::ZERO,
            facing: 2048,
            status: ActorState::AimShuriken,
            motion: MOT_SYURI,
            height: 2000,
            active_item: 1,
        };
        for template in guards {
            for scenario in 0..9 {
                let blocked = scenario == 5;
                let bounds = if blocked {
                    vec![[-10000, -10000, 10000, 200], [-10000, 400, 10000, 10000]]
                } else {
                    vec![[-10000, -10000, 10000, 10000]]
                };
                game.mission.terrain = AreaMap::from_nodes(vec![(
                    0,
                    bounds
                        .into_iter()
                        .map(|bounds| Node {
                            y: 0,
                            dy: 0,
                            bounds,
                            attributes: 0x2000,
                            division: u16::MAX,
                        })
                        .collect(),
                )])
                .unwrap();
                game.cursor = terrain::Cursor::new(&game.mission.terrain);
                let mut actor = template.clone();
                actor.position = Position::new(0, 0, 6000);
                actor.rotation[1] = 0;
                actor.target = Some(player.root);
                actor.scripted = scenario == 3;
                actor.attribute = ATTR_CUSTOMAI | ATTR_SEARCH | ATTR_WEAPON_DRAWN | 2;
                actor.think.combat = programs::Combat::Point;
                actor.evasion_clock = if scenario == 4 { 1010 } else { 0 };
                let mut state = Intelligence {
                    quick_shuriken_dodge: scenario != 1,
                    sight: Sight {
                        distance: 6000,
                        direction: 0,
                    },
                    ..Default::default()
                };
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
                if !actor.enter(&ctx, MOT_ENGAGE_STANCE, true) {
                    continue;
                }
                let supported = [MOT_CHASE_DASH_LEFT, MOT_CHASE_DASH_RIGHT]
                    .into_iter()
                    .any(|id| {
                        let mut test = actor.clone();
                        test.enter(&ctx, id, true)
                    });
                if !supported {
                    continue;
                }
                kinds.insert(actor.kind);
                if scenario == 2 {
                    actor.enter(&ctx, MOT_ATTACK, true);
                }
                if scenario == 8 {
                    actor.enter(&ctx, MOT_DAMAGE_FRONT_MID, true);
                }
                let mut controller = Controller::new(
                    &mut actor,
                    &mut game.roster,
                    &mut game.items,
                    &mut state,
                    Input {
                        player: Player {
                            facing: if scenario == 6 { 0 } else { player.facing },
                            status: if scenario == 7 {
                                ActorState::Normal
                            } else {
                                player.status
                            },
                            ..player
                        },
                        clock: 1000,
                        buttons: 0,
                        commands: &game.commands,
                    },
                    &mut ctx,
                );
                let seed = controller.ctx.random.seed;
                let action = controller.actor.motion.action;
                assert_eq!(
                    controller.evade_shuriken().unwrap(),
                    scenario == 0,
                    "kind {}, health {}, height {}, scenario {scenario}",
                    controller.actor.kind,
                    controller.actor.health,
                    controller.ctx.mission.characters[controller.actor.definition].height
                );
                if scenario == 0 {
                    assert!(matches!(
                        controller.actor.motion.action,
                        MOT_CHASE_DASH_LEFT | MOT_CHASE_DASH_RIGHT
                    ));
                    assert_eq!(controller.actor.evasion_clock, 1030);
                    assert_ne!(controller.actor.velocity[0], 0);
                    assert_eq!(controller.actor.health, template.health);
                    assert!(!controller.actor.invulnerable);
                    assert!(!controller.evade_shuriken().unwrap());
                } else {
                    assert_eq!(controller.ctx.random.seed, seed);
                    assert_eq!(controller.actor.motion.action, action);
                }
            }
        }
    }
    println!("Verified evasion for actor kinds {kinds:?}");
    assert!(kinds.len() >= 3);
}
