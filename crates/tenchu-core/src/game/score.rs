use super::Game;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Score {
    pub criticals: i16,
    pub murders: i16,
    pub friends: i16,
    pub stealth: i16,
    pub total: i16,
    pub grade: i16,
}

impl Game {
    pub fn score(&self) -> Score {
        if u16::from(self.statistics.bosses as u8) + u16::from(self.statistics.enemies as u8) == 0 {
            return Score::default();
        }
        let spots = i16::from(self.intelligence.found as u8);
        let mut score = Score {
            criticals: i16::from(self.statistics.criticals as u8) * 20,
            murders: i16::from(self.statistics.murders as u8) * 5,
            friends: i16::from(self.statistics.friend_hits as u8) * -30,
            stealth: ((if spots == 0 { 400 } else { 300 })
                - spots * if self.mission.stage.id == 8 { 40 } else { 20 })
            .max(0),
            ..Score::default()
        };
        score.total = score
            .criticals
            .wrapping_add(score.murders)
            .wrapping_add(score.friends)
            .wrapping_add(score.stealth);
        score.grade = (score.total.max(0) / 100).min(4);
        score
    }
}

#[cfg(test)]
mod tests {
    use crate::oracle;
    use std::fmt::Write;

    #[test]
    #[ignore = "requires TENCHU_DISC and TENCHU_DECOMP; compares original score packing, penalties and rank thresholds"]
    fn score_components_match_original_c() {
        let source = oracle::source("STAGE.C");
        let routine = oracle::function(&source, "ScoreResult *calculate_score(");
        let constants = oracle::source("tuning.h")
            .lines()
            .filter(|s| {
                s.starts_with("#define SCORE_PER_")
                    || s.starts_with("#define STEALTH_BASE")
                    || s.starts_with("#define SPOT_PENALTY")
            })
            .collect::<Vec<_>>()
            .join("\n");
        let harness = format!(
            r#"
#include <stdint.h>
#include <stdio.h>
#include <string.h>
typedef uint8_t u8;typedef uint16_t u16;typedef int32_t s32;typedef int16_t packed_stage_id;
typedef struct {{u8 stageBosses,stageEnemies,findEnemies,murders,criticals,friendHits;}} ScoreStats;
typedef struct {{u16 criticalScore,murderScore;int16_t friendPenalty,spottedScore,score,grade;}} ScoreResult;
static ScoreResult STAGE_SCORE_COMPONENTS;
#define STAGE_ID_TRAINING 8
#define RANK_GRAND_MASTER 4
{constants}
{routine}
int main(void){{int a[7];while(scanf("%d%d%d%d%d%d%d",a,a+1,a+2,a+3,a+4,a+5,a+6)==7){{
ScoreStats s={{a[0],a[1],a[2],a[3],a[4],a[5]}};ScoreResult *r=calculate_score(&s,a[6]);
printf("%d %d %d %d %d %d\n",r->criticalScore,r->murderScore,r->friendPenalty,r->spottedScore,r->score,r->grade);
}}}}
"#
        );
        let mut game = oracle::game(0, 0);
        let mut random = crate::random::Random::new(73522);
        let mut input = String::new();
        let mut expected = Vec::new();
        for case in 0..512 {
            let mut values: [i16; 6] = std::array::from_fn(|_| (random.next15() - 16384) as i16);
            if case % 4 == 0 {
                values[2] = 0;
            }
            if case % 7 == 0 {
                values[0] = 256;
                values[1] = 0;
            }
            let [bosses, enemies, found, murders, criticals, friends] = values;
            game.statistics.bosses = bosses;
            game.statistics.enemies = enemies;
            game.intelligence.found = found;
            game.statistics.murders = murders;
            game.statistics.criticals = criticals;
            game.statistics.friend_hits = friends;
            game.mission.stage.id = if case % 2 == 0 { 0 } else { 8 };
            for value in values {
                write!(input, "{value} ").unwrap();
            }
            writeln!(input, "{}", game.mission.stage.id).unwrap();
            let score = game.score();
            expected.push(
                [
                    score.criticals,
                    score.murders,
                    score.friends,
                    score.stealth,
                    score.total,
                    score.grade,
                ]
                .map(i32::from)
                .to_vec(),
            );
        }
        assert_eq!(expected, oracle::run(&harness, &input));
    }
}
