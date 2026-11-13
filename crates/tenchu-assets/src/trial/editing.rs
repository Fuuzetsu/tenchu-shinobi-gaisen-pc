//! Shared authoring rules from TRIAL's terrain, character and patrol editors.
use super::{Cell, Mission, Placement, Point, Theme, Waypoint};
use crate::Result;
#[cfg(test)]
mod tests;

fn index(point: Point) -> Result<usize> {
    if !point.valid() {
        return Err("Choose a cell inside the 32 × 32 map".into());
    }
    Ok(point.z as usize * 32 + point.x as usize)
}

pub fn paint(
    mission: &mut Mission,
    theme: &Theme,
    point: Point,
    ornament: bool,
    tile: u8,
) -> Result<()> {
    let at = index(point)?;
    if tile != 255 && theme.tile(tile).is_none_or(|t| t.is_ornament() != ornament) {
        return Err("Choose a tile in this theme and layer".into());
    }
    if mission.placements.iter().any(|p| p.point == point)
        && (ornament || !theme.tile(tile).is_some_and(|t| t.supports_actor()))
    {
        return Err("Move the character before removing its supporting ground".into());
    }
    let supports = |ground, decoration| {
        theme.tile(ground).is_some_and(|t| {
            t.supports_actor()
                || t.kind() == 8
                    && theme
                        .tile(decoration)
                        .is_some_and(|t| t.flags & 0x2000 != 0)
        })
    };
    if ornament {
        if tile != 255 {
            if mission.map[at].ornament == 255
                && mission.map.iter().filter(|c| c.ornament != 255).count()
                    >= mission.limits.ornaments
            {
                return Err(format!(
                    "Ornament limit reached ({}). Adjust project Limits in Level Studio to raise it.",
                    mission.limits.ornaments
                ));
            }
            if !supports(mission.map[at].ground, tile) {
                return Err(
                    "Ornaments need level ground; disguised covers can go over pits".into(),
                );
            }
        }
        mission.map[at].ornament = tile;
    } else {
        mission.map[at].ground = tile;
        if !supports(tile, mission.map[at].ornament) {
            mission.map[at].ornament = 255;
        }
    }
    for placement in &mut mission.placements {
        let end = placement.route.windows(2).position(|p| {
            p[0].wait == 0 && blocked(theme, &mission.map, p[0].point, p[1].point, p[0].facing)
        });
        if let Some(end) = end {
            placement.route.truncate(end + 1);
            if placement.route.len() < 2 {
                placement.route.clear();
            }
        }
    }
    Ok(())
}

pub fn clear_ground(mission: &Mission, theme: &Theme, point: Point) -> Result<()> {
    let at = index(point)?;
    if mission.map[at].ornament != 255
        || !theme
            .tile(mission.map[at].ground)
            .is_some_and(|t| t.supports_actor())
    {
        return Err("Characters need clear, level ground".into());
    }
    if mission.placements.iter().any(|p| p.point == point) {
        return Err("A character already occupies this cell".into());
    }
    Ok(())
}

pub fn add_actor(mission: &mut Mission, theme: &Theme, point: Point, kind: i16) -> Result<usize> {
    if mission.placements.len() >= mission.limits.characters {
        return Err(format!(
            "Character limit reached ({} including player and objectives). Adjust project Limits in Level Studio to raise it.",
            mission.limits.characters
        ));
    }
    clear_ground(mission, theme, point)?;
    mission.placements.push(Placement {
        kind,
        point,
        route: vec![],
    });
    Ok(mission.placements.len() - 1)
}

pub fn move_actor(mission: &mut Mission, theme: &Theme, actor: usize, point: Point) -> Result<()> {
    if mission
        .placements
        .get(actor)
        .is_some_and(|p| p.point == point)
    {
        return Ok(());
    }
    clear_ground(mission, theme, point)?;
    let placement = mission
        .placements
        .get_mut(actor)
        .ok_or("Select a character")?;
    placement.point = point;
    placement.route.clear();
    Ok(())
}

pub fn patrol(
    mission: &mut Mission,
    theme: &Theme,
    actor: usize,
    waypoint: Waypoint,
) -> Result<()> {
    if actor == 0 {
        return Err("The player has no patrol".into());
    }
    index(waypoint.point)?;
    if !(0..4).contains(&waypoint.facing) || !(0..=9).contains(&waypoint.wait) {
        return Err("Invalid patrol wait or facing".into());
    }
    let placement = mission
        .placements
        .get_mut(actor)
        .ok_or("Select a character")?;
    let previous = placement.route.last().map_or(placement.point, |p| p.point);
    let delta = (waypoint.point.x - previous.x, waypoint.point.z - previous.z);
    let facing = match delta {
        (0, -1) => 0,
        (1, 0) => 1,
        (0, 1) => 2,
        (-1, 0) => 3,
        (0, 0) => waypoint.facing,
        _ => return Err("Add adjacent tiles one step at a time".into()),
    };
    if placement.route.len() >= mission.limits.patrol {
        return Err(format!(
            "Patrol limit reached ({} points)",
            mission.limits.patrol
        ));
    }
    if delta != (0, 0) && blocked(theme, &mission.map, previous, waypoint.point, facing) {
        return Err("Patrol step blocked: use connected ground or an aligned ramp".into());
    }
    if placement.route.is_empty() {
        placement.route.push(Waypoint {
            point: placement.point,
            ..Default::default()
        });
    }
    let last = placement.route.last_mut().unwrap();
    last.facing = facing;
    if delta == (0, 0) {
        last.wait = waypoint.wait.max(1);
    }
    placement.route.push(waypoint);
    Ok(())
}

pub fn blocked(theme: &Theme, map: &[Cell], from: Point, to: Point, facing: i16) -> bool {
    let (Ok(a), Ok(b)) = (index(from), index(to)) else {
        return true;
    };
    let (Some(a), Some(b)) = (map.get(a), map.get(b)) else {
        return true;
    };
    if b.ornament != 255 {
        return true;
    }
    let (Some(a_kind), Some(b_kind)) = (
        theme.tile(a.ground).map(|t| t.kind()),
        theme.tile(b.ground).map(|t| t.kind()),
    ) else {
        return true;
    };
    if matches!(a_kind, 0 | 5 | 6) && matches!(b_kind, 5 | 6) && facing == i16::from(b.ground >> 6)
        || ((a_kind == 5 && b_kind == 1) || (a_kind == 6 && b_kind == 2))
            && facing == i16::from(a.ground >> 6)
        || ((b_kind == 5 && a_kind == 1) || (b_kind == 6 && a_kind == 2))
            && (facing + 2) % 4 == i16::from(b.ground >> 6)
        || matches!(b_kind, 0 | 5 | 6)
            && matches!(a_kind, 5 | 6)
            && (facing + 2) % 4 == i16::from(a.ground >> 6)
    {
        return false;
    }
    a_kind != b_kind
}
