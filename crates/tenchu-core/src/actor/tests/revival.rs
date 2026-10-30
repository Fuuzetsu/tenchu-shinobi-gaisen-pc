use super::*;
use crate::actor::State as ActorState;
use crate::items::Processor;

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU_DECOMP; original DamageControl tail covers resurrection, launches, swimming and cinematic motion suppression"]
fn resurrection_damage_tail_matches_original() {
    let source = oracle::source("MOTION.C");
    let function = oracle::function(&source, "void DamageControl(void)\n{");
    let tail = &function[function
        .find("    if ((Me_MOTION_C->life == 0) && (Me_MOTION_C->item[ITEM_KAWARIMI] != 0))")
        .unwrap()..];
    let harness = format!(
        r#"
#include <stdint.h>
#include <stdio.h>
{}
#define ITEM_KAWARIMI 10
typedef struct {{int vx,vy,vz;}} SVECTOR;
typedef struct {{int mid;}} Motion;
typedef struct {{int life,lifemax;uint8_t item[26];struct {{int time;}}pad;}} Human;
static Human human,*Me_MOTION_C=&human;static Motion motion,*dtM=&motion;static SVECTOR velocity,*dtV=&velocity;
static int motID,motMODE,scripted,requests;
#define SET_MOTION(id,mode) do {{motID=id;motMODE=mode;}}while(0)
static void ReqItemDefault(Human*h,int kind) {{requests++;if(h->item[kind]!=0&&h->item[kind]!=255)h->item[kind]--;}}
static void SetNowMotionUnlessCva(void) {{if(!scripted)motion.mid=motID;}}
static void finish(void) {{ {}
int main(void) {{int a[7];while(scanf("%d",a)==1){{for(int i=1;i<7;i++)if(scanf("%d",a+i)!=1)return 2;
human=(Human){{.life=a[0],.lifemax=100,.item[10]=a[1],.pad.time=99}};motion.mid=a[2];motID=a[3];motMODE=a[4];scripted=a[5];requests=0;velocity=(SVECTOR){{123,456,-789}};
finish();printf("%d %d %d %d %d %d %d\n",human.life,human.item[10],motion.mid,human.pad.time,requests,a[6]?velocity.vx:0,a[6]?velocity.vz:0);
}}return 0;}}
"#,
        constants(),
        tail
    );
    let mut input = String::new();
    let mut expected = Vec::new();
    for character in 0..2 {
        let mut game = oracle::game(0, character);
        let player = game.player_id;
        for case in 0..144 {
            let health = (case % 2) as i16;
            let stock = [0, 1, 255][case / 2 % 3];
            let action = [MOT_NORMAL, MOT_SWIM, MOT_SWIM_STROKE][case / 6 % 3];
            let next = [
                MOT_DAMAGE,
                MOT_DAMAGE_LAUNCH_BACK,
                MOT_DAMAGE_LAUNCH_FORE,
                MOT_DAMAGE_BACK_LIGHT,
            ][case / 18 % 4];
            let scripted = case >= 72;
            let water = action != MOT_NORMAL;
            writeln!(
                input,
                "{health} {stock} {action} {next} 0 {} {}",
                i32::from(scripted),
                i32::from(water)
            )
            .unwrap();
            with_actor(&mut game, player, |actor, ctx, world| {
                for item in &mut world.items.slots {
                    ctx.collision.remove(ctx.scene, item.model);
                    item.behavior = crate::items::Behavior::Inactive;
                }
                actor.scripted = scripted;
                actor.status = ActorState::Normal;
                actor.motion.loops = 0;
                actor.enter(ctx, action, true);
                actor.health = health;
                actor.max_health = 100;
                actor.inventory[10] = stock;
                actor.pad.time = 99;
                actor.velocity = [123, 456, -789];
                actor.finish_damage(ctx, world, next, false).unwrap();
                expected.push(vec![
                    i32::from(actor.health),
                    i32::from(actor.inventory[10]),
                    i32::from(actor.motion.action),
                    actor.pad.time as i32,
                    world
                        .items
                        .slots
                        .iter()
                        .filter(|i| i.processor() == Some(Processor::Resurrection))
                        .count() as i32,
                    if water {
                        i32::from(actor.velocity[0])
                    } else {
                        0
                    },
                    if water {
                        i32::from(actor.velocity[2])
                    } else {
                        0
                    },
                ]);
            });
        }
    }
    let actual = oracle::run(&harness, &input);
    assert_eq!(actual.len(), expected.len());
    for (case, (actual, expected)) in actual.iter().zip(&expected).enumerate() {
        assert_eq!(
            actual,
            expected,
            "case {case}: {}",
            input.lines().nth(case).unwrap()
        );
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; a lethal arrow consumes a resurrection leaf after damage and its particle item completes through ordinary ticks"]
fn lethal_arrows_revive_both_players_and_finish_the_effect() {
    for character in 0..2 {
        let (mut game, kind) = (0..11)
            .find_map(|stage| {
                let game = oracle::game(stage, character);
                let kind = game
                    .mission
                    .characters
                    .iter()
                    .find(|c| {
                        c.kind >= 0x10
                            && game.mission.models.contains_key(&c.kind)
                            && c.motions
                                .iter()
                                .any(|m| m.action == MOT_ATTACK && m.clip == 0xaa)
                    })
                    .map(|c| c.kind)?;
                Some((game, kind))
            })
            .unwrap();
        let shooter = arena(&mut game, kind);
        let player = game.player_id;
        game.roster.get_mut(player).health = 10;
        game.roster.get_mut(player).inventory[10] = 1;
        game.camera.snap_pending = true;
        with_actor(&mut game, shooter, |actor, ctx, _| {
            actor.attribute = ATTR_WEAPON_DRAWN;
            actor.think = crate::ai::programs::Programs::default();
            actor.enter(ctx, MOT_ATTACK, true);
        });
        let mut restored = false;
        let mut effect = false;
        for _ in 0..180 {
            game.step(0).unwrap();
            restored |= game.player().inventory[10] == 0
                && game.player().health == game.player().max_health;
            effect |= game.effects.slots.iter().any(
                |e| matches!(e,Some(crate::effects::Effect::Bleed(p)) if p.color==[100,200,220]),
            );
        }
        assert!(
            restored && effect,
            "character {character}: revived {restored}, particles {effect}"
        );
        assert_eq!(game.player().health, game.player().max_health);
        assert!(
            !game
                .items
                .slots
                .iter()
                .any(|i| i.processor() == Some(Processor::Resurrection))
        );
    }
}
