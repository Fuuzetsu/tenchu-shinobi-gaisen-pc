use super::*;

#[test]
#[ignore = "requires TENCHU_DISC; independent attacks preserve a single guard's behavior and allow a pair to fight both heroes"]
fn independent_attacks_preserve_duels_and_allow_group_combat() {
    let run = |character, count, independent| {
        let mut game = oracle::game(0, character);
        let first = arena(&mut game, 0x10);
        let mut guards = vec![first];
        let player = game.player_id;
        with_actor(&mut game, player, |actor, ctx, world| {
            actor.health = 999;
            actor.max_health = 999;
            for _ in 1..count {
                guards.push(
                    world
                        .roster
                        .spawn(0x10, Position::new(900, 0, 2200), 0, ctx)
                        .unwrap(),
                );
            }
            for &id in &guards {
                let guard = world.roster.get_mut(id);
                guard.setup_think(0x4444).unwrap();
                guard.target = Some(actor.root_model);
            }
        });
        game.intelligence.independent_attacks = independent;
        let mut overlap = 0;
        let mut attacked = vec![false; count];
        let mut frames = Vec::new();
        for _ in 0..900 {
            game.step(0).unwrap();
            let mut attacking = 0;
            for (i, &id) in guards.iter().enumerate() {
                if game.roster.get(id).status == ActorState::Attack {
                    attacked[i] = true;
                    attacking += 1;
                }
            }
            overlap += usize::from(attacking > 1);
            frames.push((
                game.random.seed,
                game.roster
                    .order
                    .iter()
                    .map(|&id| {
                        let a = game.roster.get(id);
                        (
                            a.position,
                            a.health,
                            a.status,
                            a.motion.action,
                            a.motion.count,
                            a.brain.clone(),
                        )
                    })
                    .collect::<Vec<_>>(),
            ));
        }
        assert!(attacked.into_iter().all(|a| a));
        assert!(game.player().health < 999, "No guard landed a hit");
        (frames, overlap)
    };
    for character in 0..2 {
        assert_eq!(run(character, 1, false), run(character, 1, true));
        assert!(run(character, 2, true).1 > 0, "No overlapping attacks");
    }
}
