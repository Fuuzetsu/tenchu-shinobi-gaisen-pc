use super::*;
use crate::{
    collision::{Collision, DOOR_OWNER},
    scene::{COLLIDE, CONFLICT},
};

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU_DECOMP; compares original door/pitfall transitions and leaf transforms"]
fn hinged_callbacks_match_original() {
    let source = oracle::source("MISC.C");
    let functions = [
        "static void ProcMiscDoor(TMisc *m, TMiscMessage msg)",
        "static void ProcMiscPitfall(TMisc *m, TMiscMessage msg)",
    ]
    .map(|signature| oracle::function(&source, signature))
    .join("\n");
    let harness = [r#"
#include <stdint.h>
#include <stdio.h>
typedef int32_t s32;typedef int16_t s16;typedef int TMiscMessage;
typedef struct {int vx,vy,vz,pad;} VECTOR;
typedef struct {s16 vx,vy,vz,pad;} SVECTOR;
typedef struct Coordinate {struct {int t[3];} coord;struct Coordinate*super;} Coordinate;
typedef Coordinate GsCOORDINATE2;
typedef struct {Coordinate locate;SVECTOR rotate;uint16_t attribute;} ModelType;
typedef struct {ModelType*locate;s16 r,dr;unsigned char type;} TDoor;
typedef struct {ModelType*locate;s16 r;unsigned char type;} TPitfall;
typedef struct {int x,y,z;unsigned char mode;union {struct {int rotation,type,reserved;} hinged_init;TDoor door;TPitfall pitfall;}param;} TMisc;
typedef struct {ModelType*Model[2];s16 HitSize;} HingedData;
typedef struct {VECTOR position;SVECTOR offset,size;void*common;} ConflictObjectType;
static HingedData DoorData[11],PitfallData[3];static ModelType root,leaf[2];
static ConflictObjectType conflicts[3],*ConflictObject=conflicts+1;
static int result,angle,sounds,sound_id;
static char *fmt_unknown_door_type,*fmt_unknown_pitfall_type;
enum {MM_CREATE,MM_DESTROY,MM_PAUSE,MM_RESUME,MM_DO};
enum {N_DOOR_TYPES=11,N_PITFALL_TYPES=3,DOOR_KIND_MON6=0,PITFALL_KIND_OTO_LEFT=0,DOOR_MODE_IDLE=0,DOOR_MODE_OPENING=1,PITFALL_MODE_CLOSED=0,PITFALL_MODE_OPENING=1,PITFALL_MODE_OPEN=2};
enum {ANGLE_FULL=4096,ANGLE_HALF=2048,ANGLE_QUADRANT=1024,DOOR_ANGLE_STEP=64,DOOR_OPEN_ANGLE=960,DOOR_OPENING_OFFSET_DIVISOR=10240,CONFLICT_OWNER_DOOR=2,CONFLICT_SOFT=8,CONFLICT_NONE=-1,MODEL_ATTR_CONFLICT=0x8000,SE_MECHANISM=0x40};
#define MODEL_ARCHIVE_NONE -1
#define MODEL_POSITION(m) ((VECTOR*)&(m)->locate.coord.t)
static int ratan2(int z,int x){return angle;}
static int GetConflictResult(ModelType*m,int index){return result;}
static void SoundEx(VECTOR*p,int id){sounds++;sound_id=id;}
static int InsertConflict(ModelType*m){return 0;}
static void DeleteConflict(ModelType*m){}
static void DisposeModel(ModelType*m){}
static ModelType*LoadModel(int m){return &root;}
static void UpdateCoordinate(ModelType*m){}
static void DrawModel(ModelType*m){}
static void AdtMessageBox(void*m,...){}
"#, &functions, r#"
int main(void) {int a[14];while(scanf("%d",a)==1) {
for(int i=1;i<14;i++)if(scanf("%d",a+i)!=1)return 2;
TMisc m={0};root=(ModelType){0};leaf[0]=leaf[1]=(ModelType){0};
m.mode=a[1];root.rotate.vy=a[4];root.attribute=a[6];result=a[7];angle=a[9];sounds=0;sound_id=-1;
ConflictObject[result].common=(void*)(intptr_t)a[8];
HingedData row={{a[10]?&leaf[0]:(ModelType*)-1,a[11]?&leaf[1]:(ModelType*)-1},a[5]};
if(a[0]){DoorData[0]=row;m.param.door=(TDoor){&root,a[2],a[3],0};ProcMiscDoor(&m,MM_DO);}
else{PitfallData[0]=row;m.param.pitfall=(TPitfall){&root,a[2],0};ProcMiscPitfall(&m,MM_DO);}
printf("%d %d %d %d %d",m.mode,a[0]?m.param.door.r:m.param.pitfall.r,a[0]?m.param.door.dr:a[3],sounds,sound_id);
for(int i=0;i<2;i++)printf(" %d %d",leaf[i].locate.coord.t[0],a[0]?leaf[i].rotate.vy:leaf[i].rotate.vz);
puts("");
}return 0;}
"#].concat();
    let mut game = oracle::game(0, 0);
    game.collision = Collision::default();
    let root = game.scene.add(Some(0), Matrix::default());
    let other = game.scene.add(Some(0), Matrix::default());
    let own = game.collision.insert(&mut game.scene, root).unwrap();
    let contact = game.collision.insert(&mut game.scene, other).unwrap();
    let mut input = String::new();
    let mut expected = Vec::new();
    for case in 0..4096 {
        let door = case & 1 != 0;
        let kind = case / 2 % if door { 11 } else { 3 };
        let mode = (case / 6 % 3) as u8;
        let angle = [
            -1024, -960, -959, -64, 0, 64, 959, 960, 1020, 1024, 32700, -32768,
        ][case / 18 % 12];
        let speed = [-64, 64][case / 17 % 2];
        let yaw = [0, 1024, 2048, 3072, 3073, -1024][case / 7 % 6];
        let attr = [0, COLLIDE, COLLIDE | CONFLICT, CONFLICT][case / 5 % 4];
        let result = if case / 3 % 5 == 0 || attr & COLLIDE == 0 {
            -1
        } else {
            1
        };
        let owner = if result < 0 {
            1048
        } else {
            [0, 2, 100][case / 3 % 3]
        };
        let p = [8024, -500, -43249];
        let q = if result < 0 {
            [0x98040, 0, 0xffffff]
        } else {
            [
                [8024, 0, -42249],
                [9024, 0, -43249],
                [7024, 0, -43249],
                [8024, 0, -44249],
            ][case / 11 % 4]
        };
        let atan = game.math.atan2(q[2] - p[2], q[0] - p[0]).unwrap();
        let data = if door {
            game.mission.misc.doors[kind]
        } else {
            game.mission.misc.pitfalls[kind]
        };
        writeln!(
            input,
            "{} {mode} {angle} {speed} {yaw} {} {attr} {result} {owner} {atan} {} {} 0 0",
            i32::from(door),
            data.width,
            i32::from(data.models[0].is_some()),
            i32::from(data.models[1].is_some())
        )
        .unwrap();
        game.scene.models[root].attribute = attr;
        game.scene.update(
            root,
            Matrix {
                translation: p,
                rotation: game.math.rotate_yxz([0, yaw, 0]),
            },
        );
        game.collision.slots[own].count = if result < 0 { 0 } else { 1 };
        game.collision.slots[own].results.fill(0);
        if result >= 0 {
            game.collision.slots[own].results[contact] = 0x80;
        }
        game.collision.slots[contact].owner = match owner {
            2 => Some(DOOR_OWNER),
            0 => None,
            _ => Some(1),
        };
        game.collision.slots[contact].position = q;
        game.effects.sounds.clear();
        with_misc(&mut game, |misc, ctx| {
            let leaves = if door {
                misc.doors[kind]
            } else {
                misc.pitfalls[kind]
            };
            let mut object = Object {
                position: p,
                kind: if door {
                    Kind::Door {
                        kind,
                        rotation: yaw,
                    }
                } else {
                    Kind::Pitfall {
                        kind,
                        rotation: yaw,
                    }
                },
                paused: false,
                mode,
                count: 0,
                model: Some(root),
                angle,
                speed,
            };
            object.hinged(ctx, leaves, &mut Vec::new()).unwrap();
            let mut row = vec![
                i32::from(object.mode),
                i32::from(object.angle),
                i32::from(object.speed),
                ctx.effects.sounds.len() as i32,
                ctx.effects
                    .sounds
                    .last()
                    .map_or(-1, |(_, id)| i32::from(*id)),
            ];
            for (side, leaf) in leaves.into_iter().enumerate() {
                let sign = if side == 0 { 1 } else { -1 };
                row.extend(leaf.map_or([0, 0], |id| {
                    [
                        ctx.scene.models[id].local.translation[0],
                        i32::from(object.angle.wrapping_mul(sign)),
                    ]
                }));
                if let Some(id) = leaf {
                    let mut rotation = [0; 3];
                    rotation[if door { 1 } else { 2 }] = object.angle.wrapping_mul(sign);
                    assert_eq!(
                        ctx.scene.models[id].local.rotation,
                        ctx.math.rotate_yxz(rotation)
                    );
                    assert_eq!(ctx.scene.models[id].parent, Some(root));
                }
            }
            expected.push(row);
        });
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
#[ignore = "requires TENCHU_DISC; checks activation delay, shared conflict pool, reopening and retained pitfall state"]
fn environmental_activation_and_collision_lifecycle() {
    let mut game = oracle::game(0, 0);
    let initial = game.collision.len;
    let door = game
        .misc
        .objects
        .iter()
        .position(|m| matches!(m.kind, Kind::Door { .. }))
        .unwrap();
    let position = game.misc.objects[door].position;
    game.camera.reference = position;
    for clock in 1..10 {
        with_misc(&mut game, |misc, ctx| misc.step(clock, ctx)).unwrap();
    }
    assert_eq!(game.collision.len, initial);
    with_misc(&mut game, |misc, ctx| misc.step(10, ctx)).unwrap();
    let object = &game.misc.objects[door];
    let model = object.model.unwrap();
    assert!(!object.paused);
    let slot = game.scene.models[model].conflict.unwrap();
    assert_eq!(
        (
            game.collision.slots[slot].offset,
            game.collision.slots[slot].size,
            game.collision.slots[slot].owner
        ),
        ([0, -500, 0], [666, 1000, 666], Some(DOOR_OWNER))
    );
    game.misc.objects[door].angle = 960;
    game.misc.objects[door].mode = 1;
    game.camera.reference[0] = position[0] + 15000;
    with_misc(&mut game, |misc, ctx| misc.step(19, ctx)).unwrap();
    assert!(game.scene.models[model].conflict.is_some());
    with_misc(&mut game, |misc, ctx| misc.step(20, ctx)).unwrap();
    assert!(game.misc.objects[door].paused);
    assert!(game.scene.models[model].conflict.is_none());
    game.camera.reference = position;
    with_misc(&mut game, |misc, ctx| misc.step(30, ctx)).unwrap();
    assert_eq!(game.misc.objects[door].angle, 0);
    with_misc(&mut game, |misc, ctx| {
        misc.spawn(
            Spawn {
                record: 0,
                position,
                kind: Kind::Pitfall {
                    rotation: 0,
                    kind: 2,
                },
            },
            ctx,
        );
        misc.step(40, ctx).unwrap();
        let pit = misc.objects.last_mut().unwrap();
        let model = pit.model.unwrap();
        pit.mode = 2;
        pit.angle = 1024;
        pit.pause(true, ctx).unwrap();
        pit.pause(false, ctx).unwrap();
        assert_eq!((pit.mode, pit.angle), (2, 1024));
        let slot = ctx.scene.models[model].conflict.unwrap();
        assert_eq!(ctx.collision.slots[slot].size, [1000, 666, 666]);
        assert_eq!(ctx.collision.slots[slot].offset, [0; 3]);
    });
}

#[test]
#[ignore = "requires TENCHU_DISC; proves the collision-underflow direction for every authored campaign door"]
fn every_authored_door_has_a_proven_underflow_direction() {
    let mut doors = 0;
    for stage in 0..11 {
        let mut game = oracle::game(stage, 0);
        with_misc(&mut game, |misc, ctx| {
            for object in &mut misc.objects {
                let Kind::Door { kind, rotation } = object.kind else {
                    continue;
                };
                let root = object.model.unwrap();
                ctx.scene.models[root].attribute = CONFLICT;
                object
                    .hinged(ctx, misc.doors[kind], &mut Vec::new())
                    .unwrap();
                assert_eq!(object.mode, 1);
                for x in [0x98040, 0xbc090, 0xc6348, 0xffffff] {
                    for z in [0x98040, 0xbc098, 0xc6358, 0xffffff] {
                        let a = ctx
                            .math
                            .atan2(z - object.position[2], x - object.position[0])
                            .unwrap();
                        let speed = if (i32::from(a) + i32::from(rotation) + 8192) % 4096 <= 2048 {
                            64
                        } else {
                            -64
                        };
                        assert_eq!(object.speed, speed);
                    }
                }
                doors += 1;
            }
        });
    }
    assert_eq!(doors, 43);
}
