use super::{Collision, Cursor, Position, Query, Result, Surface};
use crate::terrain;
use tenchu_assets::tenchu2::collision::Gaisen;

pub(super) fn level(
    map: &Collision,
    source: &Gaisen,
    cursor: &mut Cursor,
    position: Position,
    query: Query,
) -> Result<Option<i32>> {
    let mut original = terrain::Cursor::new(&source.map);
    let mode = if query == Query::Base {
        terrain::FIRST_HIT
    } else {
        0
    };
    let height = terrain::level(&source.map, &mut original, position, mode)?;
    cursor.current = None;
    cursor.last = None;
    cursor.material = Gaisen::material(original.attribute);
    let x = position.x / 10 - i32::from(map.origin[0]);
    let z = position.z / 10 - i32::from(map.origin[1]);
    let size = i32::from(map.cell_size);
    if x >= 0 && z >= 0 {
        let column = (x / size) as usize;
        let row = (z / size) as usize;
        if column < map.dimensions[0] && row < map.dimensions[1] {
            let cell = row * map.dimensions[0] + column;
            if let Some(node) = source.offsets[cell]
                .iter()
                .position(|&at| at == original.area)
            {
                let surface = Surface { cell, node };
                cursor.index = node;
                cursor.last = Some(surface);
                if height != terrain::NONE {
                    cursor.current = Some(surface);
                }
            }
        }
    }
    Ok((height != terrain::NONE).then(|| {
        height
            - if query == Query::Relative {
                (position.y / 10) * 10
            } else {
                0
            }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tenchu_assets::terrain::{AreaMap, Node};

    #[test]
    fn imported_height_rows_and_masked_floors_keep_their_boundaries() {
        let top = Node {
            y: -1000,
            dy: 500,
            bounds: [0, 0, 99, 99],
            attributes: 0x6001,
            division: u16::MAX,
        };
        let floor = Node {
            y: 0,
            dy: 0,
            attributes: 0x2001,
            ..top
        };
        let original = AreaMap::from_nodes(vec![(-800, vec![top]), (2, vec![floor])]).unwrap();
        let map = Collision::from_gaisen(&original).unwrap();
        let mut cursor = Cursor::default();
        // ACM's authored row boundary takes precedence over this overlapping
        // slope. Flattening all rectangles would incorrectly find -6000 twice.
        for (y, expected) in [(-9000, Some(-6000)), (-7000, Some(0)), (-9000, Some(-6000))] {
            assert_eq!(
                super::super::level(&map, &mut cursor, [800, y, 500].into(), Query::Surface)
                    .unwrap(),
                expected
            );
            assert!(cursor.current.is_some());
        }
        assert_eq!(
            super::super::level(&map, &mut cursor, [990, -9000, 500].into(), Query::Uncached)
                .unwrap(),
            Some(-5050)
        );
        assert_eq!(
            super::super::level(
                &map,
                &mut cursor,
                [1000, -9000, 500].into(),
                Query::Uncached
            )
            .unwrap(),
            None
        );
        assert_eq!(map.into_terrain().unwrap(), original);

        let original = AreaMap::from_nodes(vec![
            (
                2,
                vec![Node {
                    division: u16::MAX - 1,
                    ..floor
                }],
            ),
            (102, vec![Node { y: 100, ..floor }]),
        ])
        .unwrap();
        let map = Collision::from_gaisen(&original).unwrap();
        assert_eq!(
            super::super::level(&map, &mut cursor, [10, -100, 10].into(), Query::Uncached).unwrap(),
            Some(1000)
        );
        assert_eq!(
            super::super::level(&map, &mut cursor, [300, -100, 10].into(), Query::Uncached)
                .unwrap(),
            Some(0)
        );
        let surface = cursor.current.unwrap();
        assert_eq!(
            map.cells[surface.cell].nodes[surface.node].bounds,
            [0, 0, 100, 100]
        );
    }
}
