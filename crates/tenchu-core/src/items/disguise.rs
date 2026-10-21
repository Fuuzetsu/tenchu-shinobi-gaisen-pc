use super::*;
use crate::{
    ai::Intelligence,
    ids::{MOT_ATTACK_STEALTH_BACK, MOT_ITEM_KAENGEKI},
};

#[derive(Clone, Debug)]
struct Snapshot {
    kind: i16,
    height: i16,
    positions: [[i16; 3]; 15],
}

impl Snapshot {
    fn capture(actor: &Actor, scene: &Scene) -> Result<Self> {
        if actor.parts.len() > 15 {
            return Err("Disguise snapshot exceeds fifteen body parts".into());
        }
        let mut positions = [[0; 3]; 15];
        for (p, &model) in positions.iter_mut().zip(&actor.parts) {
            *p = scene.models[model]
                .local
                .translation
                .map(|v| v as i16)
                .to_array();
        }
        Ok(Self {
            kind: actor.mesh_kind,
            height: actor.motion.pose.root_height,
            positions,
        })
    }

    fn apply(&self, actor: &mut Actor, scene: &mut Scene) -> Result<()> {
        if actor.parts.len() > self.positions.len() {
            return Err("Disguised actor exceeds fifteen body parts".into());
        }
        actor.mesh_kind = self.kind;
        actor.motion.pose.root_height = self.height;
        for (i, &model) in actor.parts.iter().enumerate() {
            let position = self.positions[i].map(i32::from);
            actor.motion.pose.translation[i] = position.into();
            scene.set_translation(model, position.into());
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub(super) struct Disguise {
    snapshots: Option<[Snapshot; 2]>,
    active: Option<usize>,
    remaining: u16,
    player: usize,
}

impl Items {
    pub(crate) fn capture_disguises(
        &mut self,
        player: &Actor,
        target: &Actor,
        scene: &Scene,
    ) -> Result<()> {
        self.disguise.snapshots = Some([
            Snapshot::capture(player, scene)?,
            Snapshot::capture(target, scene)?,
        ]);
        self.disguise.player = player.id;
        Ok(())
    }

    pub(super) fn finish_disguise(
        &mut self,
        id: usize,
        actors: &mut Actors,
        ctx: &mut Context,
    ) -> Result<()> {
        if self.disguise.active == Some(id) {
            let owner = actors.get_mut(self.slots[id].owner.ok_or("Disguise has no owner")?);
            self.disguise
                .snapshots
                .as_ref()
                .ok_or("Disguise snapshots missing")?[0]
                .apply(owner, ctx.scene)?;
            if owner.status == 11 {
                let action = owner.return_normal(ctx, self.disguise.player)?;
                owner.enter(ctx, action, true);
            }
            self.disguise.active = None;
            owner.active_item = 0;
        }
        Ok(())
    }

    pub(super) fn disguise(
        &mut self,
        id: usize,
        actors: &mut Actors,
        intelligence: &mut Intelligence,
        ctx: &mut Context,
    ) -> Result<()> {
        let item = &self.slots[id];
        let owner = actors.get_mut(item.owner.ok_or("Disguise has no owner")?);
        match item.mode {
            0 => {
                owner.enter(ctx, MOT_ITEM_KAENGEKI, true);
                ctx.effects.sounds.push((owner.position, 0x4c));
                self.slots[id].mode = 1;
            }
            1 => {
                if owner.motion.action != MOT_ITEM_KAENGEKI {
                    return self.refund_cast(id, actors, ctx);
                }
                if owner.motion.count != 0 || owner.motion.loops == 0 {
                    return Ok(());
                }
                let action = owner.return_normal(ctx, self.disguise.player)?;
                owner.enter(ctx, action, true);
                let position = ctx.scene.models[owner.root_model].local.translation;
                ctx.effects
                    .smoke(position, [0, -50, 0], 10, 6, ctx.random)?;
                if let Some(previous) = self.disguise.active {
                    self.dispose(previous, actors, ctx)?;
                }
                self.disguise.active = Some(id);
                self.slots[id].mode = 2;
            }
            2 => {
                self.disguise
                    .snapshots
                    .as_ref()
                    .ok_or("Disguise snapshots missing")?[1]
                    .apply(owner, ctx.scene)?;
                self.slots[id].mode = 3;
                self.disguise.remaining = 600;
                intelligence.alert = -600;
                owner.active_item = self.slots[id].kind as i16;
            }
            3 => {
                self.disguise.remaining = self.disguise.remaining.wrapping_sub(1);
                if self.disguise.remaining as i16 > 0
                    && owner.active_item == item.kind as i16
                    && !matches!(owner.status, 16 | 17)
                    && (owner.status != 7
                        || (owner.motion.loops >= 0
                            && owner.motion.action < MOT_ATTACK_STEALTH_BACK))
                {
                    return Ok(());
                }
                let position = ctx.scene.models[owner.root_model].local.translation;
                ctx.effects
                    .smoke(position, [0, -50, 0], 10, 6, ctx.random)?;
                self.dispose(id, actors, ctx)?;
            }
            mode => return Err(format!("Invalid disguise mode {mode}")),
        }
        Ok(())
    }
}
