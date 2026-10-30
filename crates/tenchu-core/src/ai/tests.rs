use super::*;
use crate::actor::State as ActorState;
use crate::math::Position;
use crate::{oracle, random::Random};
use std::fmt::Write;

#[test]
#[ignore = "requires TENCHU_DISC; replacement controller and roster ordering"]
fn reinforcement_updates_the_new_guard_and_returns_to_the_old_actor() {
    let mut game = oracle::game(0, 0);
    let id = game
        .roster
        .order
        .iter()
        .copied()
        .find(|&id| game.roster.get(id).kind & 0xf0 == 0x90)
        .unwrap();
    let index = game.roster.order.iter().position(|&i| i == id).unwrap();
    let len = game.roster.order.len();
    let mut actor = game.roster.take(id);
    let old_kind = actor.kind;
    let player = game.player();
    actor.position = Position::new(
        player.position.x + 18500,
        player.position.y,
        player.position.z,
    );
    actor.attribute = ATTR_CUSTOMAI | ATTR_WEAPON_DRAWN | ATTR_SEARCH | 2;
    actor.setup_think(0x4344).unwrap();
    actor.pad.held = LEFT;
    actor.pad.previous = CROSS;
    let old_pad = actor.pad.clone();
    let mut state = Intelligence::default();
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
    let player = Player::new(game.roster.get(game.player_id), &ctx);
    Controller::new(
        &mut actor,
        &mut game.roster,
        &mut game.items,
        &mut state,
        crate::ai::Input {
            player,
            clock: 37,
            buttons: 0,
            commands: &game.commands,
        },
        &mut ctx,
    )
    .update()
    .unwrap();
    assert_eq!((actor.id, actor.kind), (id, old_kind));
    assert_eq!(actor.pad, old_pad);
    assert!(actor.weapons.iter().all(Option::is_none));
    assert!(ctx.scene.models[actor.parts[0]].conflict.is_none());
    assert_eq!(game.roster.order.len(), len);
    assert_ne!(game.roster.order[index], id);
    let guard = game.roster.get(game.roster.order[index]);
    assert!(ctx.mission.reinforcements.contains(&guard.kind));
    assert_eq!(guard.think.source_ids(), [4; 4]);
    assert_eq!(
        guard.attribute & (ATTR_CUSTOMAI | ATTR_WEAPON_DRAWN | ATTR_PHASE),
        ATTR_CUSTOMAI | ATTR_WEAPON_DRAWN | 2
    );
    assert_eq!(guard.pad.time, 1);
    assert_eq!(state.population_delta, [0; 2]);
    game.roster.restore(actor);
    assert_eq!(game.roster.get(id).kind, old_kind);
    game.roster.begin_tick();
    assert!(game.roster.actors[id].is_none());
}

#[test]
#[ignore = "requires TENCHU_DISC, TENCHU_DECOMP and a C compiler; original AI decisions"]
fn combat_decisions_match_original_c() {
    let source = oracle::source("THINK.C");
    let routines: String = [
        "s16 GotoPosition(",
        "static __inline__ s16 SuccessionAttack(",
        "static void ItemUse(void)\n{",
        "static short AttackAnimal(",
        "static short AttackShort(",
        "static short AttackGeneral(",
        "static short AttackLong(",
        "static short AttackIndirect(",
        "s16 Think3chase(",
        "s16 Think3attack(",
        "s16 Think3escape(",
        "s16 Think3area(",
        "s16 Think3hitaway(",
        "s16 Think3firstattack(",
    ]
    .iter()
    .map(|signature| format!("{}\n", oracle::function(&source, signature)))
    .collect();
    let mut constants = String::new();
    for line in oracle::source("game_types.h").lines() {
        let Some((name, value)) = line.trim().split_once('=') else {
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
        let value = value.split(',').next().unwrap().trim();
        let number = if let Some(hex) = value.strip_prefix("0x") {
            i64::from_str_radix(hex, 16)
        } else {
            value.parse()
        };
        if let Ok(number) = number {
            writeln!(constants, "#define {name} {number}").unwrap();
        }
    }
    let mut game = oracle::game(0, 0);
    let template = game.player().clone();
    let player = Player {
        id: game.player_id,
        root: template.root_model,
        position: template.position,
        status: ActorState::Normal,
        motion: MOT_NORMAL,
        height: 2000,
        active_item: 0,
    };
    let mut command_rows = String::new();
    for command in &game.commands {
        write!(
            command_rows,
            "{{{}, {}, {{",
            command.id,
            command.inputs.len()
        )
        .unwrap();
        for value in &command.inputs {
            write!(command_rows, "{value},").unwrap();
        }
        command_rows.push_str("}},");
    }
    let mut motion_rows = String::new();
    for action in [MOT_ATTACK_TAUNT, MOT_SYURI, MOT_ITEM_THROW] {
        let mut actor = template.clone();
        let result = actor.motion.change(
            action,
            &game.mission.characters[actor.definition].motions,
            &game.mission.common_motions,
            &game.mission.clips,
        );
        writeln!(
            motion_rows,
            "{{{action}, {}, {}}},",
            actor.motion.count,
            i32::from(result != crate::motion::Change::Missing)
        )
        .unwrap();
    }
    let harness = format!(
        r#"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
typedef int32_t s32; typedef int16_t s16; typedef uint32_t u32; typedef uint16_t u16; typedef uint8_t u8;
typedef s16 weapon_attack_class; typedef s16 pad_command; typedef s16 motion_id; typedef s16 motion_move_mode; typedef s16 TItemType;
{constants}
#define PADLup 0x1000
#define PADLright 0x2000
#define PADLdown 0x4000
#define PADLleft 0x8000
#define PADRleft 0x80
#define PADRright 0x20
#define PADRdown 0x40
#define PAD_TURN_BUTTONS_SIGNED ((s16)0xa000)
#define ATTR_PHASE 3
#define ATTR_SEARCH 0x10
#define ATTR_WALL 0x400
#define ATTR_HIT 0x4000
#define PAGE_MASK 0xf0
#define AREA_LEVEL_RETURN_DELTA 2
#define AREA_LEVEL_FIRST_HIT 8
#define AREA_LEVEL_REUSE_CACHED 16
#define HUMANOID_HOME_X 0
#define HUMANOID_HOME_Z 1
#define HUMANOID_CHASE_X 0
#define HUMANOID_CHASE_Z 1
#define SR_GONE -2
#define SR_NONE 0
#define PAD_HOLD(p,n) (((u32)(p)<<16)|(u32)(n))
#define WEAPON_ATTACK_CLASS(k) ((k)>>4)
#define N_WEAPON_ATTACK_CLASSES 4
#define MELEE_ATTACK_CLOSING 0
#define MELEE_ATTACK_ENGAGED 1
#define MELEE_ATTACK_DECISION_PERIOD 16
#define SR_CLEAR_RANGE 10000
#define INDIRECT_RANGE 20000
#define ATTACK_COOLDOWN_PER_LEVEL 10
#define LEVEL_NONE INT32_MIN
#define CHAR_VOICE_NOTICE 12
#define ANIMAL_ATTACK_TIMER_RESET 0
#define ANIMAL_ATTACK_RANGE 2000
#define ANIMAL_ATTACK_AIM 200
#define ANIMAL_ATTACK_NOTICE_FRAME 30
#define ANIMAL_ATTACK_FULL_STEER_FRAME 90
typedef struct {{ s32 vx,vy,vz; }} VECTOR;
typedef struct {{ s16 vx,vy,vz; }} SVECTOR;
typedef struct {{ u16 data, stream[4]; s16 time; }} PADtype;
typedef struct {{ s16 count, mid; }} MotionManager;
typedef struct Humanoid {{
    s16 type,status,turn,wpatk,life,lifemax,warid,width;
    u8 item[26],actmode,actflg,actcnt,actscnt;
    s32 point[2], chase[2]; u32 pad_hold;
    PADtype pad; MotionManager *motion; VECTOR *locate; SVECTOR *rotate;
}} Humanoid;
typedef struct {{ s16 contfrm; }} BattleType;
static Humanoid *Me, *StagePlayer;
static s32 Distance, GameClock, AttackActionCount, ProbeLevelLow;
static s16 Degree, SR, Attrib, EngageLevel;
static s16 atkd[4]={{3000,3500,4000,20000}},atkd2[4]={{2000,3000,4000,20000}};
static BattleType BattleDB[1]; static void *GlobalAreaMap;
static uint32_t seed; static int calls, voices;
static int random15(void) {{ seed=seed*0x41c64e6du+12345u; calls++; return (seed>>16)&32767; }}
#define rand random15
static short AttackShort(void),AttackGeneral(void),AttackLong(void),AttackIndirect(void);
static s16 (*AttackFunc[4])(void)={{AttackShort,AttackGeneral,AttackLong,AttackIndirect}};
static s16 ChasetoTarget(s32 length) {{ abort(); }}
static s16 GetDirection(s32 x,s32 z,s32 yaw) {{ abort(); }}
static s32 GetAreaMapLevel(void *m,s32 x,s32 y,s32 z,int mode) {{ abort(); }}
static void GetMoveSpeed(SVECTOR *v,s16 angle,s16 forward,s16 side) {{ abort(); }}
static s32 SquareRoot0(s32 value) {{ return 0; }}
static void Sound(Humanoid *actor,s16 voice) {{ voices++; }}
static void ReqItemDefault(Humanoid *actor,TItemType item) {{ abort(); }}
static s16 SetCommand(PADtype *pad,pad_command id) {{
    static const struct {{ u16 id,len,inputs[4]; }} table[]={{{command_rows}}};
    for (unsigned i=0;i<sizeof(table)/sizeof(table[0]);i++) if(table[i].id==id) {{
        for(unsigned j=1;j<table[i].len;j++) pad->stream[j-1]=table[i].inputs[j];
        pad->time=1; return table[i].inputs[0];
    }}
    return 0;
}}
static s16 SetNowMotion(Humanoid *actor,motion_id id,motion_move_mode move) {{
    static const s16 table[][3]={{{motion_rows}}};
    if(actor->motion->mid!=id) {{
        int found=0;
        for(unsigned i=0;i<sizeof(table)/sizeof(table[0]);i++) if(table[i][0]==id) {{
            if(!table[i][2]) return 0;
            found=1; actor->motion->mid=id; actor->motion->count=table[i][1];
        }}
        if(!found) abort();
    }}
    actor->status=id>>8; return 1;
}}
{routines}
int main(void) {{
    int a[24];
    while(scanf("%d",&a[0])==1) {{
        for(int j=1;j<24;j++) if(scanf("%d",&a[j])!=1) return 1;
        Humanoid actor={{0}},player={{0}}; MotionManager motion={{a[5],a[6]}};
        VECTOR position={{0}}; SVECTOR rotation={{0,a[23],0}};
        actor.type=a[10];actor.status=a[4];actor.turn=a[7];actor.wpatk=a[8]<<4;
        actor.life=100;actor.lifemax=100;actor.motion=&motion;actor.locate=&position;actor.rotate=&rotation;
        actor.actmode=1;actor.actflg=1;actor.actcnt=a[18];actor.actscnt=a[19];
        actor.pad_hold=(u32)a[20];actor.pad.data=a[17];actor.item[1]=a[21];actor.item[4]=a[22];
        actor.chase[0]=1234;actor.chase[1]=5678;
        Me=&actor;StagePlayer=&player;player.status=a[11];
        Distance=a[1];Degree=a[2];SR=a[3];EngageLevel=a[9];
        GameClock=a[12];AttackActionCount=a[13];Attrib=a[14];
        BattleDB[0].contfrm=a[16];seed=(u32)a[15];calls=voices=0;
        s16 (*functions[])(void)={{Think3firstattack,Think3attack,AttackShort,AttackGeneral,AttackLong,AttackIndirect,AttackAnimal,Think3chase,Think3area,Think3hitaway}};
        int pad=(u16)functions[a[0]]();
        printf("%d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d %d\n",
          pad,(u16)Attrib,actor.status,motion.mid,motion.count,actor.actmode,actor.actflg,actor.actcnt,actor.actscnt,
          actor.chase[0],actor.chase[1],(s32)actor.pad_hold,rotation.vy,SR,AttackActionCount,(s32)seed,calls,voices,
          actor.pad.time,actor.pad.stream[0],actor.pad.stream[1],actor.pad.stream[2],actor.pad.stream[3],0);
    }}
}}
"#
    );
    let mut input = String::new();
    let mut expected = Vec::new();
    let mut cases = Random::new(0x5eed);
    let distances = [
        0, 999, 1000, 1499, 1500, 1999, 2000, 2499, 2500, 2999, 3000, 3499, 3500, 3999, 4000, 4999,
        5000, 9999, 10000, 14000, 15000, 16500, 19999, 20000,
    ];
    let angles = [
        -2048, -1501, -1500, -1200, -1001, -1000, -501, -500, -301, -300, -201, -200, -101, -100,
        -51, -50, 0, 50, 51, 100, 101, 200, 201, 300, 301, 500, 501, 1000, 1001, 1200, 1500, 1501,
        2048,
    ];
    for function in 0..10 {
        for _ in 0..8192 {
            let mut actor = template.clone();
            let mut state = Intelligence::default();
            let distance = distances[cases.next15() as usize % distances.len()];
            let degree = angles[cases.next15() as usize % angles.len()];
            let status = [5, 6, 7, 9][cases.next15() as usize % 4];
            let count = [-1, 0, 1, 8, 15, 16, 17][cases.next15() as usize % 7];
            let action = if cases.next15() & 7 == 0 {
                MOT_ENGAGE
            } else {
                MOT_ENGAGE_STANCE
            };
            let turn = [20, 40, 60, 100][cases.next15() as usize % 4];
            let class = cases.next15() as usize % 4;
            let difficulty = match cases.next15() % 3 {
                0 => Difficulty::Easy,
                1 => Difficulty::Normal,
                _ => Difficulty::Hard,
            };
            let kind = if function == 6 { 0xa0 } else { 0x60 };
            let player_status = if cases.next15() & 1 == 0 { 0 } else { 14 };
            let clock = 1000;
            let attack_clock = [0, 900, 1000, 1100][cases.next15() as usize % 4];
            let attribute = ATTR_CUSTOMAI
                | ATTR_WEAPON_DRAWN
                | 2
                | if cases.next15() & 1 != 0 {
                    ATTR_WALL
                } else {
                    0
                };
            let seed = cases.next15() as u32 * 12345;
            let detection = [
                Detection::Gone,
                Detection::Unseen,
                Detection::None,
                Detection::Seen,
                Detection::Glimpse,
            ][cases.next15() as usize % 5];
            actor.kind = kind;
            actor.position = Position::ZERO;
            actor.home = [0; 2];
            actor.status = ActorState::from_source_id(status);
            actor.motion.action = action;
            actor.motion.count = count;
            actor.brain = Brain {
                chase: [1234, 5678],
                mode: 1,
                flag: 1,
                count: cases.next15() as u8,
                timer: cases.next15() as u8,
            };
            actor.pad = Pad::default();
            actor.pad.held = [0, UP, DOWN, LEFT, RIGHT, SQUARE][cases.next15() as usize % 6];
            actor.pad.hold_counter = 0;
            actor.inventory = [0; 25];
            actor.inventory[1] = (cases.next15() & 1) as u8;
            actor.inventory[4] = (cases.next15() & 1) as u8;
            actor.rotation[1] = cases.next15() as i16;
            actor.attribute = attribute;
            actor.attack = 0;
            game.mission.characters[actor.definition].weapon = (class as i16) << 4;
            game.mission.characters[actor.definition].turn = turn;
            game.mission.weapons.attacks[0].combo = 8;
            state.difficulty = difficulty;
            state.sight = Sight {
                distance,
                direction: degree,
            };
            state.detection = detection;
            state.attack_clock = attack_clock;
            let values = [
                function,
                distance,
                i32::from(degree),
                detection.source_value(),
                i32::from(status),
                i32::from(count),
                i32::from(action),
                i32::from(turn),
                class as i32,
                difficulty.reaction_delay(),
                i32::from(kind),
                i32::from(player_status),
                clock,
                attack_clock,
                i32::from(attribute),
                seed as i32,
                8,
                i32::from(actor.pad.held),
                i32::from(actor.brain.count),
                i32::from(actor.brain.timer),
                0,
                i32::from(actor.inventory[1]),
                i32::from(actor.inventory[4]),
                i32::from(actor.rotation[1]),
            ];
            for v in values {
                write!(input, "{v} ").unwrap();
            }
            input.push('\n');
            game.random = Random::new(seed);
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
                &mut actor,
                &mut game.roster,
                &mut game.items,
                &mut state,
                crate::ai::Input {
                    player: Player {
                        status: ActorState::from_source_id(player_status),
                        ..player
                    },
                    clock,
                    buttons: 0,
                    commands: &game.commands,
                },
                &mut ctx,
            );
            let pad = match function {
                0 => controller.first_attack(),
                1 => controller.attack_point(),
                2 => controller.short(),
                3 => controller.general(),
                4 => controller.long(),
                5 => controller.ranged(),
                6 => controller.animal(),
                7 => controller.attack_chase(),
                8 => controller.attack_area(),
                _ => controller.hit_away(),
            }
            .unwrap();
            let Controller {
                actor,
                state,
                ctx,
                attribute,
                ..
            } = controller;
            expected.push(vec![
                i32::from(pad),
                i32::from(attribute),
                i32::from(actor.status.source_id()),
                i32::from(actor.motion.action),
                i32::from(actor.motion.count),
                i32::from(actor.brain.mode),
                i32::from(actor.brain.flag),
                i32::from(actor.brain.count),
                i32::from(actor.brain.timer),
                actor.brain.chase[0],
                actor.brain.chase[1],
                actor.pad.hold_counter as i32,
                i32::from(actor.rotation[1]),
                state.detection.source_value(),
                state.attack_clock,
                ctx.random.seed as i32,
                ctx.random.calls as i32,
                state.voices.len() as i32,
                i32::from(actor.pad.time),
                i32::from(actor.pad.stream[0]),
                i32::from(actor.pad.stream[1]),
                i32::from(actor.pad.stream[2]),
                i32::from(actor.pad.stream[3]),
                0,
            ]);
        }
    }
    let actual = oracle::run(&harness, &input);
    assert_eq!(actual.len(), expected.len());
    for (i, (a, b)) in actual.iter().zip(&expected).enumerate() {
        assert_eq!(a, b, "case {i}: {}", input.lines().nth(i).unwrap());
    }
}
