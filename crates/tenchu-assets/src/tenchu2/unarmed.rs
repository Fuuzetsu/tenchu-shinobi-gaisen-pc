use super::{BONES, Move, Moves, motions, registry::Registry, weapons::Weapons};
use crate::{ReadAt, Result, disc::Disc, weapon};

pub const WANG_KIND: i16 = 0xe3;
pub const WANG_CLIPS: [i16; 8] = [0x3ce, 0x3cf, 0x3d0, 0x3d1, 0x3d2, 0x3d3, 0x3d4, 0x3d5];
pub const WANG_ACTIONS: [i16; 8] = [0x900, 0x901, 0x904, 0x905, 0x906, 0x907, 0x90a, 0x910];

pub fn import_wang(disc: &Disc<impl ReadAt>) -> Result<Moves> {
    let registry = Registry::load(disc)?;
    let actor = registry
        .actor(WANG_KIND)
        .ok_or("Missing Wang Xiaohai definition")?;
    let weapons = Weapons::load(disc)?;
    let idle = registry
        .clip(WANG_KIND, 0x100)
        .ok_or("Missing Wang Xiaohai stance")?;
    let clips = motions::collect(disc, WANG_CLIPS.into_iter().chain([idle]).collect())?;
    let mut attacks = Vec::new();
    for (action, id) in WANG_ACTIONS.into_iter().zip(WANG_CLIPS) {
        if registry.clip(WANG_KIND, action) != Some(id) {
            return Err("Unsupported Wang Xiaohai motion registry".into());
        }
        let source = clips.get(&id).ok_or("Missing Wang Xiaohai animation")?;
        let timing = weapons
            .attack(id)
            .ok_or("Missing Wang Xiaohai hit timing")?;
        let mut contacts = weapons
            .parts(actor.weapon)
            .filter(|p| p.radius > 0 && timing.bones & (1 << p.bone) != 0);
        let contact = contacts
            .next()
            .ok_or("Missing Wang Xiaohai striking limb")?;
        if contact.model.is_some() || contacts.next().is_some() || timing.bones.count_ones() != 1 {
            return Err("Unsupported Wang Xiaohai striking limbs".into());
        }
        let mut clip = source.gaisen_pose();
        clip.sweep = (-i32::from(source.blend))
            .try_into()
            .map_err(|_| "Wang Xiaohai motion blend exceeds Gaisen range")?;
        clip.forward_speed = source.forward_speed;
        clip.side_speed = source.side_speed;
        attacks.push(Move {
            clip,
            attack: weapon::Attack {
                clip: id,
                power: timing.power.into(),
                steering: timing.steering.into(),
                combo: timing.combo.into(),
                start: timing.start.into(),
                end: timing.end.into(),
                trail_start: 0,
                trail_end: 0,
            },
            part: BONES
                .iter()
                .position(|&b| b == contact.bone)
                .ok_or("Unsupported Wang Xiaohai contact bone")?,
            secondary_part: None,
            hitbox: Some(weapon::Hitbox {
                kind: weapon::kind::FIST,
                center: contact.center,
                radius: contact.radius,
                trail: contact.trail,
            }),
        });
    }
    let moves = Moves {
        format: "tenchu2-wang-xiaohai-v1".into(),
        attacks,
        rest_yaw: Some(
            clips
                .get(&idle)
                .ok_or("Missing Wang Xiaohai stance animation")?
                .bones[0][0]
                .value[1],
        ),
    };
    moves.validate()?;
    Ok(moves)
}
