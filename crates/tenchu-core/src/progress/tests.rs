use super::*;
use crate::oracle;
use std::fmt::Write;

#[test]
#[ignore = "requires TENCHU_DECOMP; compares stock awards, leftover returns and RNG with original C"]
fn rewards_and_returns_match_original_c() {
    let source = oracle::source("award_stage_items_.c");
    let award = oracle::function(&source, "void award_stage_items_(");
    let source = oracle::source("StageEndScreen.c");
    let start = source.find("    for (item_index = ITEM_SHURIKEN;").unwrap();
    let end = start
        + source[start..]
            .find("    for (i = 0; i < N_LOADOUT_ITEMS;")
            .unwrap();
    let returns = &source[start..end];
    let harness = format!(
        r#"
#include <stdint.h>
#include <stdio.h>
#include <string.h>
typedef int16_t s16; typedef uint16_t u16; typedef uint8_t u8;
typedef s16 stage_award_tier;
enum {{ITEM_KAGINAWA,ITEM_SHURIKEN,ITEM_MAKIBISHI,ITEM_KUSURI,ITEM_FIRE,ITEM_SMOKE,ITEM_JIRAI,ITEM_DOKUDANGO,ITEM_GOSHIKIMAI,ITEM_NEMURI,ITEM_KAWARIMI,ITEM_HENSHIN,ITEM_GOSIN,ITEM_SHINSOKU,ITEM_NINGYO,ITEM_HAPPOU,ITEM_NINKEN,ITEM_KAENGEKI,ITEM_MANEBUE,ITEM_ARMOUR,N_LOADOUT_ITEMS}};
enum {{STAGE_AWARD_GRAND_MASTER,STAGE_AWARD_MASTER_NINJA,STAGE_AWARD_NINJA,STAGE_AWARD_NOVICE,STAGE_AWARD_THUG}};
#define ITEM_LOCKED 254
#define ITEM_INFINITE 255
#define RANK_GRAND_MASTER 4
typedef struct {{u8 gItem[2][32],selItem[32];int CharType,StageNo;}} TLinkInfo;
typedef struct {{s16 grade;}} ScoreResult;
static uint32_t seed;static int calls;
static int rand(void){{seed=seed*0x41c64e6du+12345;calls++;return (seed>>16)&32767;}}
static int StageItem[11];
static TLinkInfo state;
static struct Actor {{u8 item[26];}} actor;
static struct {{void *unused;struct Actor *Owner;}} CamState;
#define PSTATE (&state)
{award}
int main(void){{int grade,reward,value;unsigned initial;
while(scanf("%d%d%u",&grade,&reward,&initial)==3){{
memset(&state,0,sizeof(state));seed=initial;calls=0;StageItem[0]=reward;ScoreResult result={{grade}};
for(int i=0;i<20;i++){{scanf("%d",&value);state.gItem[0][i]=value;}}
for(int i=0;i<20;i++){{scanf("%d",&value);actor.item[i]=value;}}
CamState.Owner=&actor;award_stage_items_(&state,&result);int item_index;
{returns}
for(int i=0;i<20;i++)printf("%d ",state.gItem[0][i]);printf("%d %d\n",(int32_t)seed,calls);
}}}}
"#
    );
    let mut input = String::new();
    let mut expected: Vec<Vec<i32>> = Vec::new();
    let mut values = Random::new(734102);
    for case in 0..2048 {
        let grade = (case % 5) as i16;
        let reward = 9 + values.next15() as usize % 11;
        let seed = values.seed;
        let mut stock = std::array::from_fn(|_| {
            let value = values.next15() as u8;
            if value < 80 { LOCKED } else { value }
        });
        let remaining = std::array::from_fn(|_| {
            let value = values.next15();
            match value % 4 {
                0 => 0,
                1 => INFINITE,
                _ => value as u8,
            }
        });
        write!(input, "{grade} {reward} {seed} ").unwrap();
        for value in stock.into_iter().chain(remaining) {
            write!(input, "{value} ").unwrap();
        }
        writeln!(input).unwrap();
        let mut random = Random::new(seed);
        super::award(&mut stock, grade, reward, &mut random);
        return_items(&mut stock, &remaining);
        expected.push(
            stock
                .map(i32::from)
                .into_iter()
                .chain([random.seed as i32, random.calls as i32])
                .collect(),
        );
    }
    assert_eq!(expected, oracle::run(&harness, &input));
}

#[test]
fn completion_keeps_character_records_and_uses_unplayed_layouts() {
    let stage = Stage {
        id: 0,
        order: 1,
        name: String::new(),
        directory: String::new(),
        start: [0; 3],
        facing: 0,
        reward: 13,
    };
    let mut progress = Progress::default();
    let other = progress.stock[1];
    let mut random = Random::new(5);
    let record = Record {
        bosses: 2,
        enemies: 20,
        clock: 2000,
        ..Record::default()
    };
    let result = progress.complete(
        Completion {
            stage: &stage,
            character: 0,
            layout: 0,
            language: Language::English,
            record,
            remaining: [0; ITEMS],
        },
        &mut random,
    );
    assert_eq!(result.score.grade, 4);
    assert_eq!(progress.flags, 1);
    assert_eq!(progress.stock[0][13], 1);
    assert_eq!(progress.stock[1], other);
    assert_eq!(progress.next_layout(0, 0), Some(1));
    assert_eq!(progress.next_layout(1, 0), Some(0));
    let faster = Record {
        clock: 1999,
        ..record
    };
    assert!(
        progress
            .complete(
                Completion {
                    stage: &stage,
                    character: 0,
                    layout: 0,
                    language: Language::English,
                    record: faster,
                    remaining: [0; ITEMS]
                },
                &mut random
            )
            .new_best
    );
    assert!(
        !progress
            .complete(
                Completion {
                    stage: &stage,
                    character: 0,
                    layout: 0,
                    language: Language::English,
                    record,
                    remaining: [0; ITEMS]
                },
                &mut random
            )
            .new_best
    );
    for layout in 1..3 {
        progress.records[0][0][layout] = record;
    }
    assert_eq!(progress.next_layout(0, 0), None);
    assert_eq!(progress.records[0][0][0], faster);
}

#[test]
fn training_ranks_time_within_grade_without_regular_awards() {
    let stage = Stage {
        id: 8,
        order: 0,
        name: String::new(),
        directory: String::new(),
        start: [0; 3],
        facing: 0,
        reward: 15,
    };
    let mut progress = Progress::default();
    let stock = progress.stock;
    let mut random = Random::new(123);
    for (character, found, clock, place) in [
        (0, 0, 2000, 0),
        (1, 1, 1500, 1),
        (1, 0, 1999, 0),
        (0, 0, 2000, 2),
    ] {
        let record = Record {
            enemies: 8,
            found,
            clock,
            ..Record::default()
        };
        let result = progress.complete(
            Completion {
                stage: &stage,
                character,
                layout: 0,
                language: Language::English,
                record,
                remaining: [30; ITEMS],
            },
            &mut random,
        );
        assert_eq!(result.training_place, Some(place));
    }
    assert_eq!(progress.flags, 1 << 31);
    assert_eq!(progress.highest, [0, 0]);
    assert_eq!(random, Random::new(123));
    for (character, stock) in stock.iter().enumerate() {
        for (item, &count) in stock.iter().enumerate() {
            assert_eq!(
                progress.stock[character][item],
                if item == 15 { 1 } else { count }
            );
        }
    }
    assert_eq!(progress.training.map(|row| row.character), [1, 0, 0, 1, 0]);
}

#[test]
fn loadout_limits_stock_and_does_not_refund_a_failed_mission() {
    let mut progress = Progress::default();
    let shop = tenchu_assets::shop::Shop {
        order: std::array::from_fn(|i| i + 1),
        limits: std::array::from_fn(|i| {
            if i == 0 {
                255
            } else if i < 9 {
                99
            } else if i == 18 || i == 19 {
                1
            } else {
                3
            }
        }),
    };
    let mut loadout = Loadout::new(progress.stock[0], &shop, false);
    for _ in 0..6 {
        assert!(loadout.take(1));
    }
    assert!(!loadout.take(1));
    for item in [2, 3, 4] {
        assert!(loadout.take(item));
    }
    assert!(!loadout.take(5));
    assert!(loadout.put_back(4));
    assert!(loadout.take(5));
    let (stock, selected) = loadout.finish();
    assert_eq!(stock[1], LOCKED);
    assert_eq!(selected[1], 6);
    progress.stock[0] = stock;
    let retry = Loadout::new(progress.stock[0], &shop, false);
    assert_eq!(retry.stock[1], LOCKED);
    assert_eq!(retry.selected[1], 0);
    assert_eq!(progress.stock[1][1], 6);
    let training = Loadout::new(stock, &shop, true);
    let (after, selected) = training.finish();
    assert_eq!(after, stock);
    assert_eq!(&selected[..3], &[255, 5, 0]);
}
