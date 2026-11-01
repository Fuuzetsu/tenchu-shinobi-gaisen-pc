use super::*;

#[test]
#[ignore = "requires TENCHU_DISC; independent melee/chase cooldowns allow a second attacker without removing the first guard's recovery"]
fn independent_attack_cooldowns_do_not_block_other_guards() {
    let mut game = oracle::game(0, 0);
    let template = game.player().clone();
    let player = Player {
        id: template.id,
        root: template.root_model,
        position: template.position,
        facing: template.rotation[1],
        status: ActorState::Normal,
        motion: MOT_NORMAL,
        height: 2000,
        active_item: 0,
    };
    game.mission.characters[template.definition].weapon = 0x10;
    for routine in 0..3 {
        for independent in [false, true] {
            let mut state = Intelligence {
                difficulty: Difficulty::Hard,
                independent_attacks: independent,
                sight: Sight {
                    distance: 2200,
                    direction: 0,
                },
                ..Default::default()
            };
            let mut guards = [template.clone(), template.clone()];
            for guard in &mut guards {
                guard.status = ActorState::Engage;
                guard.motion.action = MOT_ENGAGE_STANCE;
                guard.motion.count = 0;
                guard.brain.mode = 1;
                guard.attribute = ATTR_CUSTOMAI | ATTR_WEAPON_DRAWN;
                guard.inventory.fill(0);
            }
            let mut decide = |guard: &mut Actor, state: &mut Intelligence, clock| {
                game.random = Random::new(1);
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
                let mut controller = Controller::new(
                    guard,
                    &mut game.roster,
                    &mut game.items,
                    state,
                    Input {
                        player,
                        clock,
                        buttons: 0,
                        commands: &game.commands,
                    },
                    &mut ctx,
                );
                let pad = match routine {
                    0 => controller.short(),
                    1 => controller.general(),
                    _ => controller.attack_chase(),
                }
                .unwrap();
                pad & SQUARE != 0
            };
            assert!(decide(&mut guards[0], &mut state, 1000));
            assert_eq!(guards[0].attack_clock, 1010);
            assert!(!decide(&mut guards[0], &mut state, 1000));
            assert_eq!(decide(&mut guards[1], &mut state, 1000), independent);
            state.independent_attacks = !independent;
            assert!(!decide(&mut guards[0], &mut state, 1000));
            assert!(decide(&mut guards[0], &mut state, 1050));
        }
    }
}
