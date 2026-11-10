use std::collections::BTreeSet;
use tenchu_assets::tenchu2::level::Level;
use tenchu_core::tenchu2::{
    environment::{
        GAISEN_PITFALL,
        scenery::{Frame, Model, Source},
    },
    world::World,
};

pub(super) fn distant_doors<'a>(
    world: &'a World,
    level: &'a Level,
    frame: &Frame,
) -> impl Iterator<Item = Model> + 'a {
    let drawn: BTreeSet<_> = frame
        .models
        .iter()
        .filter_map(|model| match model.source {
            Source::Object(index) => Some(index),
            Source::Static(_) => None,
        })
        .collect();
    world
        .environment
        .objects
        .iter()
        .enumerate()
        .filter_map(move |(index, object)| {
            (matches!(
                level.scene.dynamic[index].kind as i16,
                0..=2 | 6 | 7 | GAISEN_PITFALL
            ) && level.grid.contains(object.cell)
                && !drawn.contains(&index))
            .then(|| Model::object(index, object))
            .flatten()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires Japanese TENCHU2_DISC; checks dormant doors, live poses, removal and source-state independence"]
    fn native_distant_doors_preserve_poses_and_activation() {
        let disc =
            crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
        let layouts = tenchu_assets::tenchu2::layout::Layout::available(&disc).unwrap();
        let mut hidden = 0;
        for map in [1, 4, 12, 15] {
            let sequence = layouts.iter().find(|l| l.map == map).unwrap().sequence;
            let mut session = super::super::super::Session::load(&disc, sequence, 1).unwrap();
            for _ in 0..30 {
                session.step(0).unwrap();
            }
            let level = &session.assets.level;
            let before = postcard::to_allocvec(&session.world).unwrap();
            let models: Vec<_> =
                distant_doors(&session.world, level, &session.frame.scenery).collect();
            hidden += models.len();
            assert!(postcard::to_allocvec(&session.world).unwrap() == before);
            for model in &models {
                let Source::Object(index) = model.source else {
                    panic!("non-object door")
                };
                assert!(
                    session
                        .world
                        .collision
                        .index(session.world.environment.objects[index].coordinate)
                        .is_none()
                );
                let mut moved = session.world.clone();
                let object = &mut moved.environment.objects[index];
                object.kind = -1;
                object.yaw = object.yaw.wrapping_add(512) & 4095;
                object.position.x += 500;
                let expected = (object.position, object.yaw);
                let pose = distant_doors(&moved, level, &session.frame.scenery)
                    .find(|pose| pose.source == model.source)
                    .unwrap();
                assert_eq!((pose.position, pose.rotation[1]), expected);
                moved.environment.objects[index].model = None;
                assert!(
                    !distant_doors(&moved, level, &session.frame.scenery)
                        .any(|pose| pose.source == model.source)
                );
            }
            session.frame.scenery.models.extend(models);
            assert_eq!(
                distant_doors(&session.world, level, &session.frame.scenery).count(),
                0
            );
            assert!(postcard::to_allocvec(&session.world).unwrap() == before);
        }
        assert!(hidden > 0, "disc fixtures must contain dormant doors");
    }
}
