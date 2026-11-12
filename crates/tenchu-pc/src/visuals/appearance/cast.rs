use super::*;
use tenchu_assets::weapon::Equipment;

pub(crate) struct Actor {
    pub body: usize,
    pub equipment: Equipment,
}

#[derive(Default)]
pub(crate) struct Cast {
    pub bodies: Vec<Appearance>,
    pub actors: BTreeMap<usize, Actor>,
    geometry: BTreeMap<usize, usize>,
}

impl Cast {
    pub fn new(bodies: Vec<Appearance>, actors: BTreeMap<usize, Actor>) -> Self {
        let geometry = bodies
            .iter()
            .enumerate()
            .flat_map(|(index, body)| {
                body.parts
                    .iter()
                    .map(move |part| (part.geometry.as_ptr() as usize, index))
            })
            .collect();
        Self {
            bodies,
            actors,
            geometry,
        }
    }

    pub fn source(&self, geometry: usize) -> Option<&Meshes> {
        self.geometry
            .get(&geometry)
            .map(|&index| &self.bodies[index].meshes)
    }

    pub fn frame<'a>(
        &'a self,
        game: &'a Game,
        inspection: bool,
        frame: Option<&'a Frame>,
    ) -> CastFrame<'a> {
        CastFrame {
            cast: self,
            game,
            inspection,
            frame,
            poses: BTreeMap::new(),
            current: None,
        }
    }
}

struct Pose<'a> {
    actor: Cow<'a, tenchu_core::actor::Actor>,
    matrices: Vec<Matrix>,
}

pub(crate) struct CastFrame<'a> {
    cast: &'a Cast,
    game: &'a Game,
    inspection: bool,
    frame: Option<&'a Frame>,
    poses: BTreeMap<usize, Pose<'a>>,
    current: Option<usize>,
}

impl CastFrame<'_> {
    pub fn visit(
        &mut self,
        key: Part,
        include: &impl Fn(Part) -> bool,
        visit: &mut impl FnMut(Part, &[model::Geometry], Matrix),
    ) -> bool {
        let Part::Actor { root, model, .. } = key else {
            return false;
        };
        let Some(replacement) = self.cast.actors.get(&root) else {
            return false;
        };
        let body = &self.cast.bodies[replacement.body];
        let posed = match self.poses.entry(root) {
            std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::btree_map::Entry::Vacant(entry) => {
                let Some(actor) = self
                    .game
                    .roster
                    .actors
                    .iter()
                    .flatten()
                    .find(|a| a.root_model == root)
                else {
                    return false;
                };
                let actor = rendered_actor(self.game, actor, self.inspection);
                let source = &self.game.mission.models[&actor.rendered_mesh_kind];
                let matrices = body.retarget(
                    source,
                    &rendered_matrices(&actor, self.inspection, self.frame),
                    self.game.scene.models[actor.parts[0]].local,
                    actor.severed.as_ref().map(|part| part.index),
                );
                entry.insert(Pose { actor, matrices })
            }
        };
        if self.current != Some(root) {
            body.meshes.pose_skin(
                &posed.matrices,
                attached_severed_pose(self.game, &posed.actor, &body.parts, &posed.matrices),
            );
            self.current = Some(root);
        }
        if include(key)
            && let Some(index) = posed.actor.parts.iter().position(|&id| id == model)
        {
            visit(key, &body.parts[index].geometry, posed.matrices[index]);
        }
        true
    }

    pub fn equipment(
        &self,
        include: &impl Fn(Part) -> bool,
        visit: &mut impl FnMut(Part, &[model::Geometry], Matrix),
    ) {
        for (&root, pose) in &self.poses {
            visit_equipment(
                &pose.actor,
                &self.cast.actors[&root].equipment,
                &pose.matrices,
                self.inspection,
                include,
                &mut *visit,
            );
        }
    }
}
