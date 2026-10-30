use super::*;
use crate::actor::State as ActorState;
use crate::math::Position;
use crate::{camera::Mode, ids::*, input};

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU_DECOMP; original fire-breath callback with previously verified motion and rotation calls stubbed"]
fn cast_interruption_and_steering_match_original() {
    let source = oracle::source("ITEM.C");
    let function = oracle::function(&source, "static void ProcItemKaengeki(TItem *item)\n{");
    let harness = [r#"
#include <stdint.h>
#include <stdio.h>
#include <assert.h>
typedef int32_t s32;typedef uint8_t item_mode;typedef int TItemType;
typedef struct {int vx,vy,vz,pad;} VECTOR;
typedef struct {int16_t vx,vy,vz;} SVECTOR;
typedef struct {struct {struct {int t[3];} coord;} locate;SVECTOR rotate;} ModelArchiveType;
typedef struct {int mid,count,loop;struct {int orderspd,sidespd;}*motion;} Motion;
typedef struct {ModelArchiveType*model;Motion*motion;int life,status;struct {uint16_t data;} pad;uint8_t stock;} Humanoid;
typedef struct {VECTOR start,end;uint8_t count;} param_kaengeki;
typedef struct Item {Humanoid*owner;ModelArchiveType*locate;int type;item_mode mode;void(*proc)(struct Item*);union {param_kaengeki kaengeki;} param;} TItem;
typedef struct {int type;Humanoid*user;VECTOR start,end;} PARAM_ITEM_LAUNCH;
enum {ITEM_MODE_DISPOSE=255,ANGLE_FULL=4096,MOT_ITEM_KAENGEKI=0xf04,ACTION_HALT_NONE=0,ATTACK_CANCEL_ALL=3,STAT_ITEM=15,SE_ITEM_USE=0x4c,SE_FIRE=0x28,KAENGEKI_DELAY=40,PADLright=0x2000,PADLleft=0x8000,ITEM_NAPALM=22,CMODE_DIRECTION=1};
static int ActionHalt,clock,rotation[3],direction[3],calls,sounds,sound_id,request,cursor;
static uint32_t seed;static VECTOR viewpoint,reference,start,end;
static struct {Humanoid*Owner;int Mode;} CamState;
static int ViewInfo;
#define CAMERA_VIEWPOINT(v) (&viewpoint)
#define CAMERA_REFERENCE(v) (&reference)
#define MODEL_POSITION(m) ((VECTOR*)(m)->locate.coord.t)
#define copyVector(a,b) do {(a)->vx=(b)->vx;(a)->vy=(b)->vy;(a)->vz=(b)->vz;}while(0)
static int game_rand(void) {seed=seed*0x41c64e6d+12345;calls++;return (seed>>16)&32767;}
#define rand game_rand
static void NowReturnNormal(Humanoid*h) {h->motion->mid=0;h->status=0;}
static void dispose_weapon_data_of_char_(Humanoid*h,int mode) {}
static void UpdateMotion(Motion*m,int mid) {m->mid=mid;}
static void MoveHumanoid(Humanoid*h,int a,int b) {}
static void Sound(Humanoid*h,int id) {sounds++;sound_id=id;}
static void SoundEx(VECTOR*p,int id) {sounds++;sound_id=id;}
static VECTOR*GetAbsolutePosition(ModelArchiveType*m,int x,int y,int z) {return MODEL_POSITION(m);}
static void ReqItemDrop(PARAM_ITEM_LAUNCH*p) {request=1;cursor++;start=p->start;end=p->end;}
static void ReqItemUse(PARAM_ITEM_LAUNCH*p) {cursor++;if(p->user->stock!=0&&p->user->stock!=255)p->user->stock--;if(clock&1){request=2;start=p->start;end.vx=(int16_t)(p->end.vx-p->start.vx);end.vy=(int16_t)(p->end.vy-p->start.vy);end.vz=(int16_t)(p->end.vz-p->start.vz);}}
static void GetVectorRotation(VECTOR*a,VECTOR*b,int*x,int*y) {*x=rotation[0];*y=rotation[1];}
static void RotateVector(VECTOR*v,int x,int y,int z) {assert(x==rotation[0]&&y==rotation[1]&&z==rotation[2]);v->vx=direction[0];v->vy=direction[1];v->vz=direction[2];}
#define DISPOSE_ITEM_WITH_MODE(item,sentinel) do {item->mode=sentinel;item->proc(item);item->owner=0;item->proc=0;}while(0)
#define DISPOSE_ITEM(item) DISPOSE_ITEM_WITH_MODE(item,255)
"#, function, r#"
int main(void) {int a[23];while(scanf("%d",a)==1){for(int i=1;i<23;i++)if(scanf("%d",a+i)!=1)return 2;
ModelArchiveType model={.rotate={0,a[8],0}},locate={.locate.coord.t={400,-1000,2000}};
struct {int orderspd,sidespd;} speed={0};Motion motion={a[2],a[3],a[4],(void*)&speed};Humanoid human={&model,&motion,a[6],a[2]>>8,{a[7]},a[11]},other={0};
TItem item={.owner=&human,.locate=&locate,.type=17,.mode=a[0],.proc=ProcItemKaengeki,.param.kaengeki={{100,-1200,1500,0},{0,0,-60,0},a[1]}};
ActionHalt=a[5];clock=a[9];CamState.Owner=a[10]?&human:&other;CamState.Mode=a[12];seed=(uint32_t)a[13];calls=sounds=request=0;sound_id=-1;cursor=1;start=end=(VECTOR){0};
for(int i=0;i<3;i++){rotation[i]=a[14+i];direction[i]=a[17+i];}
ProcItemKaengeki(&item);
printf("%d %d %d %d %d %d %d %d %d %d %d %d %d",item.mode,item.param.kaengeki.count,item.proc!=0,motion.mid,human.status,model.rotate.vy,human.stock,(int32_t)seed,calls,sounds,sound_id,cursor,request);
printf(" %d %d %d %d %d %d\n",start.vx,start.vy,start.vz,end.vx,end.vy,end.vz);
}return 0;}
"#].concat();
    let mut game = oracle::game(0, 0);
    oracle::flat_ground(&mut game);
    let player = game.player_id;
    let mut input = String::new();
    let mut expected = Vec::new();
    with_items(&mut game, |items, ctx, actors| {
        for case in 0..3072 {
            for id in 0..CAPACITY {
                items.dispose(id, actors, ctx).unwrap();
            }
            items.cursor = 1;
            let mode = case % 3;
            let remaining = [0, 1, 2, 40, 255][case / 3 % 5];
            let action = if case / 15 % 3 == 0 {
                MOT_NORMAL
            } else {
                MOT_ITEM_KAENGEKI
            };
            let count = [0, 1, 2][case / 45 % 3];
            let loops = [0, 1, -1][case / 135 % 3];
            let halt = if case / 7 % 2 == 0 { 0 } else { 10 };
            let health = [0, -1, 100][case / 11 % 3];
            let held = [0, input::LEFT, input::RIGHT, input::LEFT | input::RIGHT][case / 4 % 4];
            let yaw = [0, -2048, i16::MIN, i16::MAX][case / 16 % 4];
            let stock = [0, 1, 9, 255][case / 64 % 4];
            let camera_owner = case % 5 != 0;
            let camera_mode = if case / 2 % 2 == 0 {
                Mode::Normal
            } else {
                Mode::Direction
            };
            let seed = case as u32 * 31 + 7;
            ctx.camera.mode = camera_mode;
            ctx.camera.position = Position::new(0, -3000, 5000);
            ctx.camera.reference = Position::new(1000, -1000, 0);
            let actor = actors.get_mut(player);
            actor.attribute = 0;
            actor.status = ActorState::Normal;
            actor.enter(ctx, action, true);
            actor.motion.count = count;
            actor.motion.loops = loops;
            actor.health = health;
            actor.pad.held = held;
            actor.rotation = [0, yaw, 0];
            actor.inventory[22] = stock;
            let turn = if held & input::RIGHT != 0 {
                32
            } else if held & input::LEFT != 0 {
                -32
            } else {
                0
            };
            let rotation = if camera_owner && camera_mode == Mode::Direction {
                ctx.math
                    .vector_rotation(ctx.camera.position, ctx.camera.reference)
                    .unwrap()
            } else {
                [0, yaw.wrapping_add(turn), 0]
            };
            let direction = Matrix {
                rotation: ctx.math.rotate_yxz(rotation),
                ..Matrix::default()
            }
            .apply_large(Position::new(0, 0, -60));
            items.initialize(
                1,
                behavior(Processor::Breath, Some(player), 0),
                Kind::FireBreath,
                Position::new(400, -1000, 2000),
                ctx,
            );
            items.slots[1].set_source_mode(mode as u8);
            items.slots[1].breath = crate::items::breath::Breath {
                origin: Position::new(100, -1200, 1500),
                remaining,
                player: if camera_owner { player } else { usize::MAX },
            };
            *ctx.random = Random::new(seed);
            ctx.effects.sounds.clear();
            writeln!(input,"{mode} {remaining} {action} {count} {loops} {halt} {health} {held} {yaw} {case} {} {stock} {} {seed} {} {} {} {} {} {} 0 0 0",i32::from(camera_owner),camera_mode.source_id(),rotation[0],rotation[1],rotation[2],direction[0],direction[1],direction[2]).unwrap();
            items
                .process(
                    1,
                    actors,
                    &mut crate::ai::Intelligence {
                        action_halt: if halt == 0 {
                            crate::ai::Halt::Running
                        } else {
                            crate::ai::Halt::Active
                        },
                        ..Default::default()
                    },
                    0,
                    case as i32,
                    ctx,
                )
                .unwrap();
            let item = &items.slots[1];
            let actor = actors.get(player);
            let mut row = vec![
                i32::from(item.source_mode()),
                i32::from(item.breath.remaining),
                i32::from(item.processor().is_some()),
                i32::from(actor.motion.action),
                i32::from(actor.status.source_id()),
                i32::from(actor.rotation[1]),
                i32::from(actor.inventory[22]),
                ctx.random.seed as i32,
                ctx.random.calls as i32,
                ctx.effects.sounds.len() as i32,
                ctx.effects
                    .sounds
                    .last()
                    .map_or(-1, |(_, id)| i32::from(*id)),
                items.cursor as i32,
            ];
            let spawned = &items.slots[2];
            let (kind, position, velocity) = match spawned.processor() {
                Some(Processor::Pickup) => (
                    1,
                    ctx.scene.models[spawned.model].local.translation,
                    spawned.rolling.velocity,
                ),
                Some(Processor::Napalm) => (
                    2,
                    ctx.scene.models[spawned.model].local.translation,
                    spawned.napalm.velocity,
                ),
                None => (0, Position::ZERO, [0; 3]),
                other => panic!("Unexpected spawned item {other:?}"),
            };
            row.push(kind);
            row.extend(position);
            row.extend(velocity.map(i32::from));
            expected.push(row);
        }
    });
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
#[ignore = "requires TENCHU_DISC; both characters cast, steer and recover through ordinary actor/item updates"]
fn fire_breath_runs_through_normal_ticks() {
    for character in 0..2 {
        let mut game = oracle::game(0, character);
        let player = game.player_id;
        with_items(&mut game, |items, ctx, actors| {
            let mut actor = actors.roster.take(player);
            actor.inventory[17] = 1;
            actor.attribute &= !ATTR_WEAPON_DRAWN;
            items
                .request_default(Kind::FireBreath, &mut actor, player, actors.roster, ctx)
                .unwrap();
            actors.roster.restore(actor);
        });
        let mut firing = 0;
        let mut visible = 0;
        for tick in 0..180 {
            game.step(if (40..60).contains(&tick) {
                input::RIGHT
            } else {
                0
            })
            .unwrap();
            firing += usize::from(
                game.items
                    .slots
                    .iter()
                    .any(|i| i.processor() == Some(Processor::Napalm)),
            );
            visible += game
                .items
                .drawings
                .iter()
                .filter(|d| matches!(d.graphic, effects::Graphic::Sprite { image: 7, .. }))
                .count();
        }
        assert!(
            firing > 20 && visible > 10,
            "character {character}: firing {firing}, sprites {visible}"
        );
        assert!(
            !game
                .items
                .slots
                .iter()
                .any(|i| matches!(i.processor(), Some(Processor::Breath | Processor::Napalm)))
        );
        assert_ne!(game.player().motion.action, MOT_ITEM_KAENGEKI);
        assert_eq!(game.player().inventory[17], 0);
    }
}
