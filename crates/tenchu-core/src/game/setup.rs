use crate::{
    Result,
    actor::{Context, patrol::Patrol},
    ids::{ATTR_CUSTOMAI, ATTR_SUSPEND, ATTR_TRACE},
    roster::Roster,
    scene::COLLIDE,
    terrain,
};

#[derive(Clone, Debug, Default)]
pub struct Statistics {
    pub citizens: i16,
    pub enemies: i16,
    pub bosses: i16,
    pub criticals: i16,
    pub murders: i16,
    pub friend_hits: i16,
}

pub(super) fn construct(
    roster: &mut Roster,
    items: &mut crate::items::Items,
    misc: &mut crate::misc::Misc,
    character: i16,
    ctx: &mut Context,
) -> Result<(usize, Statistics)> {
    enum Spawn {
        Actor(tenchu_assets::stage::Enemy),
        Pickup(tenchu_assets::stage::Pickup),
        Misc(tenchu_assets::misc::Spawn),
    }
    let mut construction: Vec<_> = ctx
        .mission
        .construction_actors
        .iter()
        .cloned()
        .map(|s| (s.slot, Spawn::Actor(s)))
        .chain(
            ctx.mission
                .construction_pickups
                .iter()
                .cloned()
                .map(|(record, s)| (record, Spawn::Pickup(s))),
        )
        .chain(
            ctx.mission
                .misc
                .spawns
                .iter()
                .cloned()
                .map(|s| (s.record, Spawn::Misc(s))),
        )
        .collect();
    construction.sort_by_key(|(record, _)| *record);
    for (_, spawn) in construction {
        match spawn {
            Spawn::Actor(s) => {
                roster.spawn(s.kind, s.position, s.facing, ctx)?;
            }
            Spawn::Pickup(s) => {
                items.stay(
                    s.kind,
                    s.position,
                    &mut crate::roster::Actors::new(roster),
                    ctx,
                )?;
            }
            Spawn::Misc(s) => misc.spawn(s, ctx),
        }
    }
    let player = roster.spawn(character, [0; 3], 0, ctx)?;
    let actor = roster.get_mut(player);
    actor.setup_think(0x1111);
    actor.position = ctx.mission.stage.start;
    actor.rotation = [0, ctx.mission.stage.facing as i16, 0];
    ctx.scene.set_translation(actor.root_model, actor.position);
    let player_model = actor.root_model;

    let dog = roster.spawn(0xa9, [999000; 3], 0, ctx)?;
    roster.get_mut(dog).attribute |= ATTR_SUSPEND;
    let disguise = roster.spawn(ctx.mission.disguise, [999000; 3], 0, ctx)?;
    items.capture_disguises(roster.get(player), roster.get(disguise), ctx.scene)?;
    roster.remove(disguise, ctx);
    ctx.effects.reset();
    while roster.order.len() > 1 {
        roster.remove(roster.order[1], ctx);
    }
    if roster.order.first() != Some(&player) {
        return Err("Construction actors displaced the stage player".into());
    }
    if let Some(mut layouts) = ctx.mission.random_layouts.take() {
        let index = ctx.random.next15() as usize % 3;
        ctx.mission.select_layout(layouts.swap_remove(index));
    }
    items.restore_layout(roster, ctx)?;
    for spawn in ctx.mission.enemies.clone() {
        let id = roster.spawn(spawn.kind, spawn.position, 0, ctx)?;
        let actor = roster.get_mut(id);
        actor.rotation[1] = spawn.facing;
        actor.attribute |= ATTR_SUSPEND;
        actor.target = Some(player_model);
        ctx.scene.models[actor.parts[0]].attribute &= !COLLIDE;
        actor.setup_think(spawn.think);
        if !spawn.patrol.is_empty() {
            actor.patrol = Some(Patrol::new(&spawn.patrol, actor.position));
            actor.attribute |= ATTR_TRACE;
        }
    }
    for spawn in ctx.mission.story.clone() {
        let id = match roster.find(spawn.kind) {
            Some(id) => id,
            None => roster.spawn(spawn.kind, [0; 3], 0, ctx)?,
        };
        let actor = roster.get_mut(id);
        actor.position = spawn.position;
        actor.home = [spawn.position[0], spawn.position[2]];
        let floor = terrain::level(
            &ctx.mission.terrain,
            ctx.cursor,
            [
                spawn.position[0],
                spawn.position[1].wrapping_sub(1000),
                spawn.position[2],
            ],
            0,
        )?;
        if floor < actor.position[1] {
            actor.position[1] = floor;
        }
        actor.rotation[1] = spawn.facing;
        ctx.scene.update(actor.root_model, actor.root(ctx.math));
        actor.setup_think(spawn.think);
        actor.attribute = (actor.attribute | ATTR_SUSPEND | 2) & !ATTR_CUSTOMAI;
        actor.health = -1;
        actor.target = Some(player_model);
    }
    let actors = &roster.actors;
    roster
        .order
        .sort_by_key(|&id| match actors[id].as_ref().unwrap().kind & 0xf0 {
            0 => 0,
            0x80 => 1,
            _ => 2,
        });
    let mut stats = Statistics::default();
    for &id in &roster.order {
        let actor = roster.get(id);
        if actor.max_health < 0 || id == player || actor.kind == 0xa9 {
            continue;
        }
        match actor.kind & 0xf0 {
            0x80 => stats.bosses += 1,
            0x90 => {
                stats.citizens += 1;
                continue;
            }
            _ => {}
        }
        stats.enemies += 1;
    }
    let adjustment = match ctx.mission.stage.id {
        10 => {
            if character == 0 {
                2
            } else {
                1
            }
        }
        2 | 3 => 1,
        _ => 0,
    };
    stats.bosses -= adjustment;
    stats.enemies -= adjustment;
    Ok((player, stats))
}
