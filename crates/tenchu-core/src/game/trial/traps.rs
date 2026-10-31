use super::*;

impl Game {
    pub(in crate::game) fn trial_traps(&mut self) -> Result<()> {
        if self.trial.as_ref().unwrap().phase != Phase::Playing {
            return Ok(());
        }
        if self.clock % 30 == 0 {
            self.trial_arrows()?;
        }
        if self.clock & 1 != 0 {
            return Ok(());
        }
        for &id in &self.roster.order {
            let actor = self.roster.get(id);
            if actor.attribute & ATTR_SUSPEND != 0
                || actor.map.attribute & 0x1000 == 0
                || actor.map.height != 0
            {
                continue;
            }
            let point = tenchu_assets::trial::Point {
                x: (actor.position.x / 2000) as i16,
                z: (actor.position.z / -2000) as i16,
            };
            let setup = self.mission.trial.as_ref().unwrap();
            let Some(&(_, index)) = setup.ornaments.iter().find(|(p, _)| *p == point) else {
                continue;
            };
            let mut node = self.mission.terrain.node(actor.map.area)?;
            let placement = &mut self.mission.placements[index];
            if node.y % 30 == 0 {
                self.effects.sounds.push((placement.position.into(), 0x26));
            }
            node.y += 5;
            if node.y > 100 {
                node.y = 0;
                node.division = 0;
                self.effects.sounds.push((placement.position.into(), 0x27));
                placement.position[1] = 1000000;
            } else {
                placement.position[1] += 50;
            }
            self.mission
                .terrain
                .set_surface(actor.map.area, node.y, node.division)?;
            self.scene.set_translation(
                self.construction.objects[index].model,
                placement.position.into(),
            );
        }
        Ok(())
    }
    fn trial_arrows(&mut self) -> Result<()> {
        let traps = &self.mission.trial.as_ref().unwrap().traps;
        let trigger = self.roster.order.iter().find_map(|&id| {
            let actor = self.roster.get(id);
            let (x, z) = (actor.position.x / 2000, actor.position.z / -2000);
            (actor.status != ActorState::Dead
                && actor.map.height == 0
                && (0..32).contains(&x)
                && (0..32).contains(&z)
                && traps[(z * 32 + x) as usize] == 255)
                .then_some((x as i16, z as i16, actor.position))
        });
        let state = self.trial.as_mut().unwrap();
        let Some((tx, tz, position)) = trigger else {
            state.last_trigger = None;
            return Ok(());
        };
        if state.last_trigger == Some((tx, tz)) {
            return Ok(());
        }
        state.last_trigger = Some((tx, tz));
        self.effects.sounds.push((position, 0x27));
        let mut best = 1024;
        let mut chosen = None;
        for (index, &trap) in traps.iter().enumerate() {
            if trap == 0 || trap == 255 {
                continue;
            }
            let (x, z) = ((index % 32) as i32, (index / 32) as i32);
            let (dx, dz) = (x - i32::from(tx), z - i32::from(tz));
            let distance = self.math.square_root((dx * dx + dz * dz) * 100);
            if distance < best || distance == best && self.random.next15() & 1 != 0 {
                chosen = Some((x, z, trap));
                best = distance;
            }
        }
        let Some((x, z, trap)) = chosen else {
            return Ok(());
        };
        let height = i16::from((trap >> 4) & 7);
        let facing = i16::from(trap & 3);
        let mut start = Position::new(
            x * 2000 + 1000,
            i32::from(height) * -1000 + 500,
            z * -2000 - 1000,
        );
        match facing {
            0 => start.z += 1100,
            1 => start.x += 1100,
            2 => start.z -= 1100,
            _ => start.x -= 1100,
        }
        let angle = facing * 1024 + (self.random.next15() as i16 & 255) - 128;
        let range = self.random.next15() as i16 % (height + 1) + 4;
        let velocity = self.math.velocity(angle, (height + range) * 1000, 0);
        let end = Position::new(
            start.x - i32::from(velocity[0]),
            0,
            start.z - i32::from(velocity[2]),
        );
        self.effects.sounds.push((start, 0x23));
        let mut player = self.roster.take(self.player_id);
        let result = self.items.request(
            crate::items::Request {
                kind: crate::items::Kind::Arrow,
                start,
                end,
            },
            &mut player,
            self.player_id,
            &mut self.roster,
            &mut Context {
                mission: &mut self.mission,
                math: &self.math,
                body: &self.body,
                cursor: &mut self.cursor,
                random: &mut self.random,
                effects: &mut self.effects,
                scene: &mut self.scene,
                collision: &mut self.collision,
                camera: &mut self.camera,
                drawing: &mut self.drawing,
            },
        );
        self.roster.restore(player);
        result
    }
}
