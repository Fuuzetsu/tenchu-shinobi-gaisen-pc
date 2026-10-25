use super::*;

impl Application {
    pub(super) fn advance_demo_tools(&mut self) -> Result<()> {
        let player = self.demo.as_ref().unwrap();
        if player.clip().feature != Feature::Experiment {
            return Ok(());
        }
        let frame = player.frame;
        let length = u64::from(player.clip().seconds) * 60;
        let start = player.scene.player_start;
        self.menu.page = 17;
        self.menu.open = frame < length * 3 / 8;
        self.menu.row = if frame < length / 8 {
            3
        } else if frame < length / 4 {
            0
        } else {
            2
        };
        if frame == length / 8 {
            self.apply_item(Item::GiveItems, 1)?;
            self.set_inventory(4, 255)?;
            self.set_god_mode(true)?;
        }
        if frame == length / 4 {
            self.set_stealth(true)?;
            self.apply_item(Item::ClickTeleport, 1)?;
        }
        if frame >= length * 3 / 8 {
            let offset = if frame < length * 5 / 8 { 2500 } else { 5000 };
            self.debug.target =
                tenchu_core::math::Position::new(start.x + offset, start.y, start.z);
        }
        if frame == length / 2 || frame == length * 3 / 4 {
            let view = self.view.as_ref().unwrap();
            let size = Vec3::new(view.size[0] as f32, view.size[1] as f32, 1.0);
            let target = self
                .debug
                .matrix(&self.campaign.game, size.x / size.y)
                .project_point3(Vec3::from(self.debug.target.map(|v| v as f32).to_array()));
            self.pointer = [
                f64::from((target.x + 1.0) * size.x * 0.5),
                f64::from((1.0 - target.y) * size.y * 0.5),
            ];
            let before = self.campaign.game.player().position;
            self.pick_debug_target();
            if self.campaign.game.player().position == before {
                return Err(format!(
                    "Experiment demo could not teleport: {}",
                    self.menu.notice
                ));
            }
            self.view
                .as_mut()
                .unwrap()
                .reset_interpolation(&self.campaign.game);
        }
        Ok(())
    }
}
