use super::paint::digits;
use super::raster::{Blend, Sprite};
use super::*;

impl Frontend {
    fn paint_training_results(
        &mut self,
        campaign: &Campaign,
        results: &tenchu_core::progress::Results,
    ) {
        self.canvas
            .sprite(Sprite::new(&self.mission_art.training, 0.0, 0.0));
        let record = results.record;
        let score = results.score;
        self.result_time(record.clock, 70.0, -97.0);
        for (count, total, points, y) in [
            (
                record.criticals,
                record.enemies.wrapping_sub(record.bosses),
                score.criticals,
                -71.0,
            ),
            (record.murders, record.enemies, score.murders, -53.0),
        ] {
            digits(
                &mut self.canvas,
                &self.result_numbers,
                count as i32,
                22.0,
                y,
                12.0,
                false,
            );
            digits(
                &mut self.canvas,
                &self.result_numbers,
                total as i32,
                47.0,
                y,
                12.0,
                false,
            );
            digits(
                &mut self.canvas,
                &self.result_numbers,
                i32::from(points),
                102.0,
                y,
                12.0,
                false,
            );
            self.canvas.sprite(
                Sprite::new(
                    &self.result_numbers,
                    37.0,
                    y + self.result_numbers.size[1] as f32 / 2.0,
                )
                .region([12 * 12, 0, 12, self.result_numbers.size[1]])
                .blend(Blend::Add),
            );
        }
        for (value, x, y) in [
            (record.found as i32, 35.0, -36.0),
            (i32::from(score.stealth), 102.0, -36.0),
            (i32::from(score.total), 102.0, -18.0),
        ] {
            digits(
                &mut self.canvas,
                &self.result_numbers,
                value,
                x,
                y,
                12.0,
                false,
            );
        }
        if score.grade == 4 {
            self.canvas.sprite(Sprite::new(
                &self.mission_art.icons[campaign.game.mission.stage.reward],
                138.0,
                -14.0,
            ));
        }
        for (index, row) in campaign.progress.training.iter().take(3).enumerate() {
            let y = index as f32 * 22.0 + 22.0;
            digits(
                &mut self.canvas,
                &self.result_numbers,
                index as i32 + 1,
                -143.0,
                y + 2.0,
                12.0,
                false,
            );
            self.result_time(row.clock, 121.0, y + 2.0);
            let character = &self.mission_art.ranks[5 + usize::from(row.character)];
            self.canvas.sprite(
                Sprite::new(
                    character,
                    -121.0 + character.size[0] as f32 / 2.0,
                    y + character.size[1] as f32 / 2.0,
                )
                .light(if results.training_place == Some(index) {
                    100 + ((self.ticks as f32 / 90.0).sin() * 28.0) as i32
                } else {
                    128
                }),
            );
            let rank = &self.mission_art.ranks[usize::from(row.grade)];
            self.canvas.label(
                Sprite::new(
                    rank,
                    -47.0 + rank.size[0] as f32 * 0.35,
                    y + rank.size[1] as f32 * 0.35,
                )
                .scale(0.7, 0.7)
                .blend(Blend::Add),
                labels::rank(self.language, usize::from(row.grade)),
                labels::rank_color(usize::from(row.grade)),
            );
        }
    }

    fn result_time(&mut self, clock: i32, x: f32, y: f32) {
        let seconds = clock.max(0) / 30;
        for (value, offset) in [
            (seconds / 60, -32.0),
            (seconds % 60 / 10, -12.0),
            (seconds % 10, 0.0),
        ] {
            digits(
                &mut self.canvas,
                &self.result_numbers,
                value,
                x + offset,
                y,
                12.0,
                false,
            );
        }
        self.canvas.sprite(
            Sprite::new(
                &self.result_numbers,
                x - 16.0,
                y + self.result_numbers.size[1] as f32 / 2.0,
            )
            .region([11 * 12, 0, 12, self.result_numbers.size[1]])
            .blend(Blend::Add),
        );
    }

    pub(super) fn paint_results(&mut self, campaign: &Campaign) {
        let crate::campaign::Screen::Results(results) = &campaign.screen else {
            return;
        };
        if campaign.options.mission == 0 {
            self.paint_training_results(campaign, results);
            return;
        }
        let background = if campaign.options.mission == 10 {
            &self.result_background
        } else {
            &self.mission_art.results
        };
        self.canvas.sprite(Sprite::new(background, 0.0, 0.0));
        let record = results.record;
        let score = results.score;
        let best = results.best.score(campaign.game.mission.stage.id);
        let rows = [
            (
                record.criticals as i32,
                record.enemies.wrapping_sub(record.bosses) as i32,
                score.criticals,
                best.criticals,
                -53.0,
            ),
            (
                record.murders as i32,
                record.enemies as i32,
                score.murders,
                best.murders,
                -26.0,
            ),
            (record.found as i32, 0, score.stealth, best.stealth, 1.0),
            (record.friends as i32, 0, score.friends, best.friends, 27.0),
        ];
        for (a, b, current, best, y) in rows {
            digits(
                &mut self.canvas,
                &self.result_numbers,
                a,
                if y < 0.0 { 10.0 } else { 28.0 },
                y,
                12.0,
                false,
            );
            if y < 0.0 {
                digits(
                    &mut self.canvas,
                    &self.result_numbers,
                    b,
                    40.0,
                    y,
                    12.0,
                    false,
                );
            }
            digits(
                &mut self.canvas,
                &self.result_numbers,
                i32::from(current),
                82.0,
                y,
                12.0,
                false,
            );
            digits(
                &mut self.canvas,
                &self.result_numbers,
                i32::from(best),
                127.0,
                y,
                12.0,
                false,
            );
        }
        digits(
            &mut self.canvas,
            &self.result_numbers,
            i32::from(score.total),
            82.0,
            57.0,
            12.0,
            false,
        );
        digits(
            &mut self.canvas,
            &self.result_numbers,
            i32::from(best.total),
            127.0,
            57.0,
            12.0,
            false,
        );
        let time = record.clock.max(0) / 30;
        for (value, x) in [(time / 60, 65.0), (time % 60 / 10, 85.0), (time % 10, 97.0)] {
            digits(
                &mut self.canvas,
                &self.result_numbers,
                value,
                x,
                -93.0,
                12.0,
                false,
            );
        }
        let rank = &self.result_ranks[score.grade as usize];
        let brightness = (self.ticks as f32 * std::f32::consts::TAU / 90.0).sin() * 0.5 + 0.5;
        self.canvas.label(
            Sprite::new(
                rank,
                -29.0 + rank.size[0] as f32 / 2.0,
                78.0 + rank.size[1] as f32 / 2.0,
            )
            .blend(Blend::Add)
            .light(((self.ticks as f32 * std::f32::consts::TAU / 90.0).sin() * 127.0) as i32 + 127),
            labels::rank(self.language, score.grade as usize),
            labels::rank_color(score.grade as usize).gamma_multiply(brightness),
        );
        if score.grade == 4 {
            let mission = &campaign.game.mission;
            let icon = &self.mission_art.icons[mission.stage.reward];
            self.canvas.sprite(
                Sprite::new(
                    icon,
                    -128.0 + icon.size[0] as f32 / 2.0,
                    49.0 + icon.size[1] as f32 / 2.0,
                )
                .light(
                    ((self.ticks as f32 * std::f32::consts::TAU / 90.0).cos() * 80.0) as i32 + 127,
                ),
            );
        }
    }
}
