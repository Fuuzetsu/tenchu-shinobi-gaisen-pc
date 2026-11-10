use super::*;

// Tenchu 2's MCD lists are ordered by height, not by Z like Gaisen's ACM rows.
// An occluding entry must stop the search instead of exposing a floor below it.
pub(super) fn level(
    map: &AreaMap,
    cursor: &mut Cursor,
    position: Position,
    mode: u16,
) -> Result<i32> {
    let (x, z) = (position.x / 10, position.z / 10);
    let y = position.y / 10 - if mode & STEP_DOWN != 0 { 150 } else { 0 };
    cursor.last_y = y;
    cursor.attribute = 0x81;
    let Some(row) = map.grid.as_ref().unwrap().cell(x, z) else {
        return Ok(NONE);
    };
    let bounds = &map.rows[row];
    for i in 0..bounds.count {
        let at = bounds.list + i * 16;
        let node = map.node(at)?;
        if !node.contains(x, z) || i32::from(node.y) + i32::from(node.dy).max(0) < y {
            continue;
        }
        let height = if mode & FIRST_HIT != 0 {
            i32::from(node.y)
        } else {
            node_level(node, x, z)?
        };
        if height == NONE {
            if node.attributes & 0x2000 != 0 {
                return Ok(NONE);
            }
            continue;
        }
        if mode & FIRST_HIT == 0 && height < y {
            continue;
        }
        cursor.row = row;
        cursor.area = at;
        cursor.attribute = node.attributes;
        if node.attributes & 2 != 0 {
            return Ok(NONE);
        }
        let floor = height * 10;
        let delta = floor.wrapping_sub(position.y);
        if delta < -1000 && mode & ALLOW_DEEP == 0 {
            return Ok(NONE);
        }
        return Ok(if mode & RETURN_DELTA != 0 {
            delta
        } else {
            floor
        });
    }
    Ok(NONE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tenchu_assets::tenchu2::collision::{Cell, Collision};

    fn cell(nodes: Vec<Node>) -> Cell {
        Cell {
            minimum: i16::MIN,
            maximum: i16::MAX,
            starts: [0; 4],
            nodes,
        }
    }

    #[test]
    fn walls_and_masked_voids_hide_the_floor_from_movement_and_sight() {
        let floor = Node {
            y: 0,
            dy: 0,
            bounds: [0, 0, 1000, 1000],
            attributes: 0,
            division: u16::MAX,
        };
        for attributes in [0x100, 0x4100, 0x8100, 0xc100] {
            let wall = Node {
                y: -300,
                bounds: [400, 300, 600, 700],
                attributes,
                ..floor
            };
            let map = Collision {
                gaisen: None,
                origin: [0, 0],
                dimensions: [1, 1],
                cell_size: 1000,
                cells: vec![cell(vec![wall, floor])],
            }
            .into_terrain()
            .unwrap();
            let mut cursor = Cursor::new(&map);
            // At eye height inside the wall the lower floor must not win.
            assert_eq!(
                super::super::level(&map, &mut cursor, [5000, -5000, 5000].into(), 0).unwrap(),
                NONE
            );
            assert_eq!(
                super::super::level(&map, &mut cursor, [5000, -5000, 2000].into(), 0).unwrap(),
                0
            );
            assert!(
                passage(
                    &map,
                    &mut cursor,
                    [3000, -5000, 5000].into(),
                    [250, 0, 0],
                    20
                )
                .unwrap()
                .is_some()
            );
        }
        let map = Collision {
            gaisen: None,
            origin: [0, 0],
            dimensions: [1, 1],
            cell_size: 1000,
            cells: vec![cell(vec![
                Node {
                    division: 0,
                    ..floor
                },
                Node { y: 100, ..floor },
            ])],
        }
        .into_terrain()
        .unwrap();
        assert_eq!(
            super::super::level(&map, &mut Cursor::new(&map), [5000, -1000, 5000].into(), 0)
                .unwrap(),
            NONE
        );
    }
}
