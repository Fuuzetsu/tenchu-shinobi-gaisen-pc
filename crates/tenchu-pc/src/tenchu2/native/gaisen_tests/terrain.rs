use super::*;
use tenchu_core::{
    tenchu2::{
        actor::Velocity,
        inventory::Item,
        items::aim::Input,
        terrain::{self, Cursor, Query},
    },
    terrain as original,
};

#[test]
#[ignore = "requires TENCHU_DISC; checks returned roof/slope metadata across all imported ACM stages"]
fn imported_surfaces_retain_the_selected_roof() {
    let game = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    let mut checked = 0;
    for stage in 0..11 {
        let source = MissionAssets::load(&game, stage, 0, 1).unwrap();
        let map =
            tenchu_assets::tenchu2::collision::Collision::from_gaisen(&source.terrain).unwrap();
        for row in &source.terrain.rows {
            for index in 0..row.count {
                let node = source.terrain.node(row.list + index * 16).unwrap();
                for dx in [1, 4, 7] {
                    for dz in [1, 4, 7] {
                        let coordinate = |axis: usize, fraction| {
                            i32::from(node.bounds[axis])
                                + (i32::from(node.bounds[axis + 2]) - i32::from(node.bounds[axis]))
                                    * fraction
                                    / 8
                        };
                        let (x, z) = (coordinate(0, dx), coordinate(1, dz));
                        let y = original::node_level(node, x, z).unwrap();
                        if y == original::NONE {
                            continue;
                        }
                        let point = Position::new(x * 10, y * 10 - 10, z * 10);
                        let mut cursor = Cursor::default();
                        let Some(height) =
                            terrain::level(&map, &mut cursor, point, Query::Uncached).unwrap()
                        else {
                            continue;
                        };
                        let surface = cursor.current.unwrap();
                        let offset =
                            map.gaisen.as_ref().unwrap().offsets[surface.cell][surface.node];
                        let selected = source.terrain.node(offset).unwrap();
                        assert_eq!(
                            original::node_level(selected, x, z).unwrap() * 10,
                            height,
                            "stage {stage} point {point:?}"
                        );
                        assert_eq!(
                            map.cells[surface.cell].nodes[surface.node].attributes, cursor.material,
                            "stage {stage} point {point:?}"
                        );
                        checked += 1;
                    }
                }
            }
        }
    }
    assert!(checked > 50000);
    eprintln!("{checked} imported surface samples retain their returned roof and material");
}

#[test]
#[ignore = "requires both discs; an imported Mission 1 roof accepts a native grapple and pulls the player up"]
fn imported_roof_grapple_uses_its_slope_instead_of_the_underside() {
    let game = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    let source = MissionAssets::load(&game, 0, 0, 1).unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let (mut assets, mut layout) = Assets::gaisen(&disc, &source, 0, false).unwrap();
    layout.actors.clear();
    let mut world = World::new(
        &layout,
        Options {
            hero: 0,
            difficulty: 2,
            seed: 1,
            inventory: Default::default(),
        },
        &assets,
    )
    .unwrap();
    let root = Position::new(28000, -4510, 16500);
    world.teleport_player(root, false, &assets).unwrap();
    world.camera.mode = 20;
    let eye = Position::new(root.x, root.y - 1600, root.z);
    let reticle = world
        .items
        .aim
        .update(
            Input {
                eye,
                look: eye.wrapping_add([-4000, -2000, 0].into()),
                root,
                camera: 20,
                grapple: true,
            },
            &world.terrain,
            &mut world.cursor,
            &assets.resources.math,
        )
        .unwrap()
        .unwrap();
    assert!(reticle.ready);
    assert!(world.items.aim.target.y < root.y - 2500);
    let player = &mut world.actors[world.player].actor;
    player
        .change_motion(0x803, Velocity::Stop, &assets.resources)
        .unwrap();
    player.motion.frame = 4;
    player.inventory.active = Item::GRAPPLE;
    let mut sounds = super::doors::Sounds::default();
    world
        .control_actor(world.player, &mut assets, &mut sounds)
        .unwrap();
    assert!(world.items.grapple().is_some());
    let mut attached = false;
    let mut raised = false;
    for _ in 0..180 {
        world
            .move_actor(world.player, &assets, &mut sounds)
            .unwrap();
        world
            .control_actor(world.player, &mut assets, &mut sounds)
            .unwrap();
        world.animate_actor(world.player, &assets).unwrap();
        world.update_items(&assets, &mut sounds).unwrap();
        world.update_effects(&assets).unwrap();
        world.finish_tick(&assets, 6).unwrap();
        let player = &world.actors[world.player].actor;
        attached |= player.motion.action == 0x804;
        raised |= player.position.y < root.y - 2500;
    }
    assert!(attached && raised, "attached {attached}, raised {raised}");
    assert!(world.items.grapple().is_none());
    let player = &world.actors[world.player].actor;
    assert!(player.health > 0);
    assert!(player.position.y < root.y - 2500);
    assert_eq!(player.motion.action, 0);
}
