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
    let mut height = terrain::level(&source.map, &mut original, position, mode)?;
    if source.water_depth != 0
        && original.attribute & 4 != 0
        && height != terrain::NONE
        && height < (position.y / 10) * 10
    {
        height = terrain::NONE;
    }
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
            let mut selected = source.offsets[cell]
                .iter()
                .position(|&at| at == original.area);
            if source.water_depth != 0 {
                let y = position.y / 10;
                for (index, bottom) in map.cells[cell].nodes.iter().enumerate() {
                    if bottom.attributes & 0xf00 != 0x300 {
                        continue;
                    }
                    let water = source.map.node(source.offsets[cell][index])?;
                    if !water.contains(position.x / 10, position.z / 10) {
                        continue;
                    }
                    let surface = terrain::node_level(water, position.x / 10, position.z / 10)?;
                    if surface == terrain::NONE || y <= surface {
                        continue;
                    }
                    let floor = surface + i32::from(source.water_depth / 10);
                    if y > floor || height != terrain::NONE && height <= floor * 10 {
                        continue;
                    }
                    // A water rectangle may be masked by ACM row priority or
                    // another surface. Confirm the original exposed shoreline.
                    let mut water_cursor = terrain::Cursor::new(&source.map);
                    if terrain::level(
                        &source.map,
                        &mut water_cursor,
                        Position::new(position.x, surface * 10, position.z),
                        0,
                    )? != surface * 10
                        || water_cursor.attribute & 4 == 0
                    {
                        continue;
                    }
                    height = if query == Query::Base {
                        i32::from(bottom.y) * 10
                    } else {
                        floor * 10
                    };
                    cursor.material = bottom.attributes;
                    selected = Some(index);
                }
            }
            if let Some(node) = selected {
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

    #[test]
    fn imported_water_retains_masked_shores_and_dry_decks() {
        let water = Node {
            y: 0,
            dy: 0,
            bounds: [0, 0, 199, 199],
            attributes: 0x2004,
            division: u16::MAX - 1,
        };
        let deck = Node {
            y: -100,
            bounds: [100, 100, 149, 149],
            attributes: 0x2008,
            division: u16::MAX,
            ..water
        };
        let original = AreaMap::from_nodes(vec![(-98, vec![deck]), (2, vec![water])]).unwrap();
        let mut map = Collision::from_gaisen(&original).unwrap();
        map.set_gaisen_water_depth(4000).unwrap();
        for (point, expected, material) in [
            ([100, -10, 100], None, 0),
            ([100, 10, 100], None, 0),
            ([1000, -1010, 1000], Some(-1000), 0),
            ([1000, -990, 1000], Some(0), 0x200),
            ([750, -10, 750], Some(0), 0x200),
            ([750, 10, 750], Some(4000), 0x300),
            ([750, 3990, 750], Some(4000), 0x300),
            ([750, 4000, 750], Some(4000), 0x300),
            ([750, 4010, 750], None, 0),
            ([2000, 10, 750], None, 0),
        ] {
            let mut cursor = Cursor::default();
            assert_eq!(
                super::super::level(&map, &mut cursor, point.into(), Query::Uncached).unwrap(),
                expected,
                "{point:?}"
            );
            if expected.is_some() {
                assert_eq!(cursor.material & 0xf00, material, "{point:?}");
                let id = cursor.current.unwrap();
                assert_eq!(
                    map.cells[id.cell].nodes[id.node].attributes,
                    cursor.material
                );
            }
        }
        let expected: Vec<_> = map.cells.iter().map(|c| c.nodes.clone()).collect();
        map.set_gaisen_water_depth(8000).unwrap();
        assert_eq!(
            super::super::level(
                &map,
                &mut Cursor::default(),
                [750, 321, 750].into(),
                Query::Relative
            )
            .unwrap(),
            Some(7680)
        );
        map.set_gaisen_water_depth(0).unwrap();
        assert_eq!(
            super::super::level(
                &map,
                &mut Cursor::default(),
                [750, 10, 750].into(),
                Query::Uncached
            )
            .unwrap(),
            None
        );
        map.set_gaisen_water_depth(4000).unwrap();
        assert_eq!(
            map.cells
                .iter()
                .map(|c| c.nodes.clone())
                .collect::<Vec<_>>(),
            expected
        );
        assert!(map.set_gaisen_water_depth(-10).is_err());
        assert!(map.set_gaisen_water_depth(4001).is_err());
        assert!(map.set_gaisen_water_depth(16010).is_err());
        assert_eq!(map.into_terrain().unwrap(), original);
    }

    #[test]
    fn imported_water_queries_match_equivalent_native_depths() {
        for height in [-1700, 0, 2450] {
            let original = AreaMap::from_nodes(vec![(
                height + 2,
                vec![Node {
                    y: height,
                    dy: 0,
                    bounds: [-100, -100, 99, 99],
                    attributes: 0x2004,
                    division: u16::MAX,
                }],
            )])
            .unwrap();
            for depth in [400, 800, 4000, 8000, 16000] {
                let mut map = Collision::from_gaisen(&original).unwrap();
                map.set_gaisen_water_depth(depth).unwrap();
                let mut native = map.clone();
                native.gaisen = None;
                for dy in (-3000..=i32::from(depth) + 10).step_by(7) {
                    let point = Position::new(-50, i32::from(height) * 10 + dy, 50);
                    for query in [
                        Query::Base,
                        Query::Uncached,
                        Query::Surface,
                        Query::Relative,
                    ] {
                        let mut a = Cursor::default();
                        let mut b = Cursor::default();
                        let actual = super::super::level(&map, &mut a, point, query).unwrap();
                        let expected = super::super::level(&native, &mut b, point, query).unwrap();
                        assert_eq!(actual, expected, "height {height}, depth {depth}, dy {dy}");
                        if actual.is_some() {
                            assert_eq!(a.material, b.material);
                        }
                    }
                }
            }
        }
    }
}
