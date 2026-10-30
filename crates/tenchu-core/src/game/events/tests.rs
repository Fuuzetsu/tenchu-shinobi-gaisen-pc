use super::*;
use crate::math::Position;
use crate::oracle;
use std::fmt::Write;

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU_DECOMP; compares original event routing, death roots and suspended cinematic returns"]
fn event_routes_and_death_continuations_match_original_c() {
    let source = oracle::source("STAGE.C");
    let types = oracle::source("game_types.h");
    let halt_states = oracle::function(&types, "enum action_halt_state");
    let types = &types[types
        .find("typedef struct EventSeqType EventSeqType;")
        .unwrap()
        ..types.find("/* Weapon kinds use signed halfword").unwrap()];
    let header = oracle::source("stage.h");
    let update = oracle::function(&source, "void UpdateEvent(");
    let stage = oracle::function(&source, "StageSequenceResult StageSequence(void)");
    let harness = format!(
        r#"
#include <stdint.h>
#include <stdio.h>
#include <assert.h>
#include <stdlib.h>
typedef uint8_t u8;typedef uint16_t u16;typedef int16_t s16;typedef int32_t s32;
typedef uint16_t HumanoidAttribute;typedef s16 character_status;typedef s16 motion_id;typedef s16 humanoid_life;
typedef struct {{s32 vx,vy,vz;}} VECTOR;
typedef struct {{s16 mid,loop,count;}} Motion;
typedef struct Humanoid {{s16 type,life,status;u16 attribute;VECTOR *locate;Motion *motion;}} Humanoid;
{types}
{header}
static Humanoid actors[2],*StagePlayer;static Motion motions[2];static VECTOR positions[2];
EventSeqType table[7],*StageEvent=table,*Event[2];Humanoid *eTarget[2];
static s32 GameClock,StageTime,ActionHalt;static int music[10],music_count;
static struct {{int Mode;}} CamState;
typedef struct {{int unused;}} ScoreStats;typedef struct {{int grade;}} ScoreResult;
#define SystemFlag 0
#define SYSFLAG_DEBUGMODE 2
#define SkipFrame 0
#define FntPrint(...) ((void)0)
#define STAT_DEAD 17
#define STAT_ACTION 1
#define STAT_ATTACK 7
#define EVENT_TABLE_END (-1)
#define EVENT_ID_NONE 255
#define EVENT_CVA_NONE 255
#define EVENT_TARGET_PLAYER 255
#define MOTION_LOOP_DISABLED (-1)
#define CMODE_FALL 14
#define CMODE_CRITICAL_HIT 4
{halt_states};
#define StageID 0
#define STAGE_ID_TRAINING 8
#define RIKIMARU_0 0
#define HIKONE 0x8d
static void SetCameraMode(int m){{CamState.Mode=m;}}
static void PlayMusicFormID(s16 m){{assert(music_count<10);music[music_count++]=m;}}
static ScoreStats *init_score_stats(ScoreStats *s){{return s;}}
static ScoreResult *calculate_score(ScoreStats *s,s16 stage){{static ScoreResult r;return &r;}}
static Humanoid *GetHumanoid(s16 kind){{return kind==0?actors:kind==0x81?actors+1:0;}}
static s16 CVAsequence(s16 id){{if(id!=12)return 0;GameClock++;if(ActionHalt!=ACTION_HALT_STAGE_END)ActionHalt=0;CamState.Mode=0;music[music_count++]=-1;return 1;}}
{update}
{stage}
int main(void){{
 int a[31];while(scanf("%d",a)==1){{for(int i=1;i<31;i++)assert(scanf("%d",a+i)==1);
 for(int i=0;i<2;i++){{positions[i]=(VECTOR){{a[3+i*9],a[4+i*9],a[5+i*9]}};motions[i]=(Motion){{a[6+i*9],a[7+i*9],a[8+i*9]}};actors[i]=(Humanoid){{i?0x81:0,a[9+i*9],a[10+i*9],a[11+i*9],positions+i,motions+i}};}}
 StagePlayer=actors;GameClock=a[21];StageTime=a[22];ActionHalt=a[23];CamState.Mode=14;music_count=0;
 table[0]=(EventSeqType){{.header.route={{0,a[1],4,5}},.target=0x81,.mode=a[0],.trigger.raw={{a[2],{{-2,2}},{{-1,0}},{{-3,3}}}}}};
 table[1]=(EventSeqType){{.header.route={{4,255,1,255}},.target=255,.mode=EVTRIG_TIME,.trigger.time=2}};
 table[2]=(EventSeqType){{.header.route={{5,255,1,255}},.target=0x81,.mode=EVTRIG_LIFE,.trigger.life=0}};
 table[3]=(EventSeqType){{.header.route={{1,255,255,255}},.target=255,.mode=EVTRIG_ALWAYS}};
 table[4]=(EventSeqType){{.header.route={{2,a[25],255,255}},.target=0x81,.mode=a[26],.trigger.raw={{a[27],{{-2,2}},{{-1,0}},{{-3,3}}}}}};
 table[5]=(EventSeqType){{.header.route={{3,255,255,255}},.target=0x81,.mode=a[28],.trigger.raw={{a[29],{{-2,2}},{{-1,0}},{{-3,3}}}}}};
 table[6].header.word=-1;
 UpdateEvent(0,a[30]);UpdateEvent(1,a[24]);
 int result=StageSequence();
 printf("%d %d %d %d %d %d %d %d %d",result,GameClock,StageTime,actors[0].status,ActionHalt,CamState.Mode,Event[0]?Event[0]->header.route.id:-1,Event[1]?Event[1]->header.route.id:-1,music_count);
 for(int i=0;i<music_count;i++)printf(" %d",music[i]);puts("");
 }}
}}
"#
    );
    let mut game = oracle::game(0, 0);
    let boss = game.roster.find(0x81).unwrap();
    for &id in &game.roster.order {
        game.roster.actors[id].as_mut().unwrap().attribute |= crate::ids::ATTR_SUSPEND;
    }
    let mut commands = Vec::new();
    for row in [
        [0i16, 12, 0, 0, 0, 0],
        [1, 1, 0, 0, 0, 0],
        [1, 0, 0, 0, 0, 0],
        [0, 14, 0, 0, 0, 0],
        [1, 0, 0, 0, 0, 0],
    ] {
        commands.extend(row.into_iter().flat_map(i16::to_le_bytes));
    }
    commands.extend((-1i16).to_le_bytes());
    game.mission.scripts.cutscenes = tenchu_assets::script::Cutscenes::parse(commands).unwrap();
    let mut random = crate::random::Random::new(91725);
    let mut input = String::new();
    let mut expected = Vec::new();
    for case in 0..576 {
        let mode = case % 9;
        let sequence = [12, 13, 14, 255][case / 9 % 4];
        let value = [0, 1, 2, 17, -1, 0x501, 0x100, 31][case / 36 % 8];
        let mut row = vec![mode as i32, sequence, value];
        for (i, id) in [game.player_id, boss].into_iter().enumerate() {
            let actor = game.roster.get_mut(id);
            actor.position = Position::new(
                random.next15() % 6000 - 3000,
                random.next15() % 2000 - 1999,
                random.next15() % 6000 - 3000,
            );
            actor.motion.action = 0x501;
            actor.motion.loops = [-1, 0, 1][case / (i + 1) % 3];
            actor.motion.count = [0, 29, 30, 31][case / 3 % 4];
            actor.health = if case % 5 < 2 { 0 } else { 100 };
            actor.status = [1, 7, 17][case / (i + 1) % 3];
            actor.attribute = crate::ids::ATTR_SUSPEND | (case as u16 & 3);
            row.extend(actor.position);
            row.extend([
                i32::from(actor.motion.action),
                i32::from(actor.motion.loops),
                i32::from(actor.motion.count),
                i32::from(actor.health),
                i32::from(actor.status),
                i32::from(actor.attribute),
            ]);
        }
        let root_mode = (case / 9 % 9) as u8;
        let secondary = if case % 3 == 0 { 5 } else { 255 };
        let first = if case % 11 == 0 { 255 } else { 0 };
        game.mission.scripts.events = vec![
            Event {
                id: 0,
                sequence: sequence as u8,
                next: [4, 5],
                target: 0x81,
                mode: mode as u8,
                value: value as i16,
                bounds: [[-2, 2], [-1, 0], [-3, 3]],
            },
            Event {
                id: 4,
                sequence: 255,
                next: [1, 255],
                target: 255,
                mode: 7,
                value: 2,
                bounds: [[0; 2]; 3],
            },
            Event {
                id: 5,
                sequence: 255,
                next: [1, 255],
                target: 0x81,
                mode: 5,
                value: 0,
                bounds: [[0; 2]; 3],
            },
            Event {
                id: 1,
                sequence: 255,
                next: [255; 2],
                target: 255,
                mode: 0,
                value: 0,
                bounds: [[0; 2]; 3],
            },
            Event {
                id: 2,
                sequence: sequence as u8,
                next: [255; 2],
                target: 0x81,
                mode: root_mode,
                value: value as i16,
                bounds: [[-2, 2], [-1, 0], [-3, 3]],
            },
            Event {
                id: 3,
                sequence: 255,
                next: [255; 2],
                target: 0x81,
                mode: root_mode,
                value: value as i16,
                bounds: [[-2, 2], [-1, 0], [-3, 3]],
            },
        ];
        game.events = Events {
            time: case as i32 % 5 - 2,
            enabled: true,
            ..Events::default()
        };
        game.update_event(0, first);
        game.update_event(1, secondary);
        game.clock = 77;
        game.intelligence.action_halt = if case % 2 == 0 {
            0
        } else {
            crate::ai::HALT_STAGE_END
        };
        game.camera.mode = camera::Mode::Fall;
        game.music.clear();
        row.extend([
            game.clock,
            game.events.time,
            i32::from(game.intelligence.action_halt),
            i32::from(secondary),
            sequence,
            i32::from(root_mode),
            value,
            i32::from(root_mode),
            value,
            i32::from(first),
        ]);
        assert_eq!(row.len(), 31);
        for value in row {
            write!(input, "{value} ").unwrap();
        }
        input.push('\n');
        game.stage_sequence().unwrap();
        while game.cinematic.active {
            game.clock += 1;
            game.advance_cinematic().unwrap();
            game.stage_sequence().unwrap();
        }
        let outcome = match game.events.outcome {
            Outcome::Running => 0,
            Outcome::Complete => 1,
            Outcome::GameOver => -1,
        };
        let mut state = vec![
            outcome,
            game.clock,
            game.events.time,
            i32::from(game.player().status),
            i32::from(game.intelligence.action_halt),
            game.camera.mode.source_id() as i32,
        ];
        state.extend(
            game.events
                .slots
                .map(|s| s.map_or(-1, |s| i32::from(game.mission.scripts.events[s.event].id))),
        );
        state.push(game.music.len() as i32);
        state.extend(game.music.iter().map(|m| m.map_or(-1, i32::from)));
        expected.push(state);
    }
    let actual = oracle::run(&harness, &input);
    for (case, (expected, actual)) in expected.iter().zip(&actual).enumerate() {
        assert_eq!(expected, actual, "case {case}");
    }
    assert_eq!(expected.len(), actual.len());
}
