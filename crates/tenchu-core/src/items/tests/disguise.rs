use super::*;
use crate::math::Position;
use crate::{ai::Intelligence, ids::*, input::TRIANGLE};

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU_DECOMP; original model snapshots truncate offsets to shorts and leave all coordinate caches intact"]
fn disguise_snapshots_match_original_short_offsets_and_preserve_caches() {
    let header = oracle::source("item.h");
    let functions = [
        "static inline void CaptureHenshinModel(",
        "static inline void ApplyHenshinModel(",
    ]
    .map(|s| oracle::function(&header, s))
    .join("\n");
    let harness=[r#"
#include <stdint.h>
#include <stdio.h>
#include <assert.h>
typedef int32_t s32;typedef int16_t s16;
typedef struct {struct {void*tmd;}object;struct {struct {s32 t[3];}coord;int flg;}locate;} ModelType;
typedef struct {struct {s16 pad;}rotate;int n;ModelType*object[15];} ModelArchiveType;
typedef struct {void*tmd;s16 x,y,z;} HenshinModelPart;
typedef struct {s32 waist;HenshinModelPart p[15];} HenshinModelSnapshot;
"#,&functions,r#"
int main(void) {int height;while(scanf("%d",&height)==1) {
ModelType from[15]={0},to[15]={0};ModelArchiveType a={.n=15},b={.n=15};a.rotate.pad=height;
for(int i=0;i<15;i++){a.object[i]=&from[i];b.object[i]=&to[i];from[i].object.tmd=(void*)(intptr_t)(i+1);to[i].locate.flg=73;
for(int j=0;j<3;j++)assert(scanf("%d",&from[i].locate.coord.t[j])==1);}
HenshinModelSnapshot snapshot={0};CaptureHenshinModel(&snapshot,&a);ApplyHenshinModel(&snapshot,&b);
printf("%d",b.rotate.pad);for(int i=0;i<15;i++){assert(to[i].object.tmd==from[i].object.tmd);assert(to[i].locate.flg==73);for(int j=0;j<3;j++)printf(" %d",to[i].locate.coord.t[j]);}puts("");}return 0;}
"#].concat();
    let mut game = oracle::game(0, 0);
    let player = game.player_id;
    let mut input = String::new();
    let mut expected = Vec::new();
    with_items(&mut game, |items, ctx, actors| {
        let donor = actors
            .roster
            .spawn(ctx.mission.disguise, [999000; 3].into(), 0, ctx)
            .unwrap();
        assert_eq!(actors.get(donor).parts.len(), 15);
        let donor_parts = actors.get(donor).parts.clone();
        let player_parts = actors.get(player).parts.clone();
        let mut random = Random::new(71);
        for case in 0..96 {
            items.dispose(1, actors, ctx).unwrap();
            let height = [-32768, 32767, -1000, 0][case % 4];
            actors.get_mut(donor).motion.pose.root_height = height;
            write!(input, "{height}").unwrap();
            for (i, &model) in donor_parts.iter().enumerate() {
                let position = std::array::from_fn(|j| match (case + i + j) % 7 {
                    0 => i32::MIN,
                    1 => i32::MAX,
                    _ => (random.next15() * 73).wrapping_sub(900000),
                });
                ctx.scene.set_translation(model, position.into());
                for v in position {
                    write!(input, " {v}").unwrap();
                }
            }
            input.push('\n');
            items
                .capture_disguises(actors.get(player), actors.get(donor), ctx.scene)
                .unwrap();
            for &model in &player_parts {
                ctx.scene.models[model].flags = 73;
                ctx.scene.models[model].cached.translation = Position::new(111, 222, 333);
            }
            let before: Vec<_> = player_parts
                .iter()
                .map(|&m| ctx.scene.models[m].clone())
                .collect();
            items
                .initialize(
                    1,
                    behavior(Processor::Disguise, Some(player), 0),
                    11,
                    Position::ZERO,
                    ctx,
                )
                .unwrap();
            items.slots[1].set_source_mode(2);
            let mut intelligence = Intelligence::default();
            items
                .process(1, actors, &mut intelligence, 0, 0, ctx)
                .unwrap();
            let actor = actors.get(player);
            assert_eq!(actor.mesh_kind, ctx.mission.disguise);
            assert_eq!(actor.rendered_mesh_kind, 0);
            assert_eq!(actor.kind, 0);
            let mut row = vec![i32::from(actor.motion.pose.root_height)];
            for (i, (&model, old)) in player_parts.iter().zip(&before).enumerate() {
                let coordinate = &ctx.scene.models[model];
                assert_eq!(coordinate.flags, old.flags);
                assert_eq!(coordinate.cached, old.cached);
                assert_eq!(coordinate.local.rotation, old.local.rotation);
                assert_eq!(coordinate.parent, old.parent);
                assert_eq!(
                    coordinate.local.translation,
                    actor.motion.pose.translation[i]
                );
                row.extend(coordinate.local.translation);
            }
            expected.push(row);
        }
    });
    let actual = oracle::run(&harness, &input);
    assert_eq!(actual.len(), expected.len());
    for (case, (a, e)) in actual.iter().zip(&expected).enumerate() {
        assert_eq!(a, e, "case {case}");
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; an interrupted disguise refunds without clearing another active item; replacing a disguise restores before transforming again"]
fn disguise_interruption_replacement_and_forced_eviction_restore_correctly() {
    let mut game = oracle::game(0, 0);
    let player = game.player_id;
    with_items(&mut game, |items, ctx, actors| {
        items
            .initialize(
                1,
                behavior(Processor::Disguise, Some(player), 0),
                11,
                actors.get(player).position,
                ctx,
            )
            .unwrap();
        items.slots[1].set_source_mode(1);
        actors.get_mut(player).enter(ctx, MOT_DAMAGE, true);
        actors.get_mut(player).active_item = 18;
        let mut intelligence = Intelligence::default();
        items
            .process(1, actors, &mut intelligence, 0, 0, ctx)
            .unwrap();
        assert!(items.slots[1].processor().is_none());
        assert_eq!(actors.get(player).active_item, 18);
        assert!(
            items
                .slots
                .iter()
                .any(|i| i.processor() == Some(Processor::Pickup) && i.kind == 11)
        );
        for id in 0..CAPACITY {
            items.dispose(id, actors, ctx).unwrap();
        }
        actors.get_mut(player).active_item = 0;
        actors.get_mut(player).enter(ctx, MOT_NORMAL, true);
        actors.get_mut(player).inventory[11] = 255;
    });
    game.hud.cursor = 11;
    game.step(0).unwrap();
    let mut previous: Option<usize> = None;
    for _ in 0..2 {
        game.step(TRIANGLE).unwrap();
        let id = game
            .items
            .slots
            .iter()
            .enumerate()
            .find(|(_, i)| i.processor() == Some(Processor::Disguise) && i.source_mode() == 1)
            .unwrap()
            .0;
        for _ in 0..120 {
            if game.items.slots[id].source_mode() == 2 {
                break;
            }
            game.step(0).unwrap();
        }
        assert_eq!(game.items.slots[id].source_mode(), 2);
        if let Some(old) = previous {
            assert!(game.items.slots[old].processor().is_none());
            assert_eq!(game.player().active_item, 0);
            assert_eq!(game.player().mesh_kind, 0);
        }
        game.step(0).unwrap();
        assert_eq!(game.items.slots[id].source_mode(), 3);
        assert_eq!(game.player().active_item, 11);
        assert_eq!(game.player().mesh_kind, game.mission.disguise);
        previous = Some(id);
    }
    let active = previous.unwrap();
    with_items(&mut game, |items, ctx, actors| {
        for id in 0..CAPACITY {
            if id != active {
                items
                    .initialize(
                        id,
                        behavior(Processor::Pickup, None, 0),
                        1,
                        Position::ZERO,
                        ctx,
                    )
                    .unwrap();
            }
        }
        items.cursor = (active + 1) % CAPACITY;
        actors.get_mut(player).status = 11;
        let id = items.allocate(actors, ctx).unwrap();
        assert_eq!(id, active);
        assert!(items.slots[id].processor().is_none());
        assert_eq!(actors.get(player).active_item, 0);
        assert_eq!(actors.get(player).mesh_kind, 0);
        assert_ne!(actors.get(player).status, 11);
    });
}

#[test]
#[ignore = "requires TENCHU_DISC; damage, death, active-item replacement and completed/stealth attacks break disguise at the original boundary"]
fn disguise_break_conditions_follow_actor_status_and_motion() {
    let mut game = oracle::game(0, 0);
    let player = game.player_id;
    with_items(&mut game, |items, ctx, actors| {
        for (status, action, loops, active, keep) in [
            (2, MOT_NORMAL, 0, 11, true),
            (16, MOT_DAMAGE, 0, 11, false),
            (17, MOT_DEAD, 0, 11, false),
            (7, MOT_ATTACK, 0, 11, true),
            (7, MOT_ATTACK, -1, 11, false),
            (7, MOT_ATTACK_STEALTH_BACK, 0, 11, false),
            (7, MOT_ATTACK_STEALTH_BACK - 1, 0, 11, true),
            (7, MOT_ATTACK_STEALTH_BACK - 1, -1, 11, false),
            (2, MOT_NORMAL, 0, 12, false),
        ] {
            items.dispose(1, actors, ctx).unwrap();
            items
                .initialize(
                    1,
                    behavior(Processor::Disguise, Some(player), 0),
                    11,
                    actors.get(player).position,
                    ctx,
                )
                .unwrap();
            items.slots[1].set_source_mode(1);
            let owner = actors.get_mut(player);
            owner.status = 2;
            owner.enter(ctx, MOT_ITEM_KAENGEKI, true);
            owner.motion.count = 0;
            owner.motion.loops = 1;
            let mut intelligence = Intelligence::default();
            items
                .process(1, actors, &mut intelligence, 0, 0, ctx)
                .unwrap();
            items
                .process(1, actors, &mut intelligence, 0, 0, ctx)
                .unwrap();
            assert_eq!(actors.get(player).mesh_kind, ctx.mission.disguise);
            let owner = actors.get_mut(player);
            owner.status = status;
            owner.motion.action = action;
            owner.motion.loops = loops;
            owner.active_item = active;
            items
                .process(1, actors, &mut intelligence, 0, 0, ctx)
                .unwrap();
            assert_eq!(
                items.slots[1].processor().is_some(),
                keep,
                "{status} {action:x} {loops} {active}"
            );
            assert_eq!(
                actors.get(player).mesh_kind,
                if keep { ctx.mission.disguise } else { 0 }
            );
            assert_eq!(actors.get(player).active_item, if keep { 11 } else { 0 });
        }
    });
}
