use super::*;
use crate::visuals::{interpolation::Part as PartId, motion::Input};

impl Drawing {
    pub(super) fn tick_motion(
        &mut self,
        session: &super::super::Session,
        options: &crate::settings::Motion,
    ) -> Result<()> {
        if self.motion.current(u64::from(session.world.tick), options) {
            return Ok(());
        }
        let inputs = self.models.motion_inputs(session, options)?;
        self.motion
            .update(u64::from(session.world.tick), &inputs, options);
        Ok(())
    }
}

impl Models {
    pub(super) fn motion_inputs(
        &self,
        session: &super::super::Session,
        options: &crate::settings::Motion,
    ) -> Result<Vec<Input>> {
        let mut inputs = Vec::new();
        if options.actors() && self.has_motion() {
            let mut scene = session.world.scene.clone();
            self.visit_actors(
                &session.world,
                &session.assets,
                &mut scene,
                None,
                |key, meshes, part, matrix, _| {
                    if let Some(rules) = meshes.motion_rules(part.geometry.as_ptr() as usize) {
                        Input::append(
                            &mut inputs,
                            key,
                            part.geometry.as_ptr() as usize,
                            matrix,
                            rules,
                            options,
                        );
                    }
                },
            )?;
        }
        Ok(inputs)
    }

    fn has_motion(&self) -> bool {
        self.bodies
            .values()
            .chain(self.imported_weapons.values())
            .any(Meshes::has_motion)
            || self.weapons.has_motion()
            || self
                .player
                .as_ref()
                .is_some_and(|p| p.appearance.meshes.has_motion())
    }

    pub(super) fn visit_actors<'a>(
        &'a self,
        world: &World,
        assets: &'a Assets,
        scene: &mut tenchu_core::scene::Scene,
        pose: Option<&PoseFrame>,
        mut visit: impl FnMut(PartId, &'a Meshes, &'a Part, Matrix, ModelStyle),
    ) -> Result<Option<Matrix>> {
        let mut player_hand = None;
        for (owner, (agent, instance)) in world.actors.iter().zip(&world.instances).enumerate() {
            let actor = &agent.actor;
            if actor.flags & 8 != 0 {
                continue;
            }
            let definition = assets
                .resources
                .registry
                .actor(actor.definition_kind())
                .ok_or("Missing native actor definition")?;
            let body = &assets.models[&definition.model];
            let meshes = &self.bodies[&definition.model];
            let matrices = instance
                .rig
                .joints
                .iter()
                .map(|&joint| {
                    scene.world(joint).map(|matrix| {
                        PoseFrame::transform(
                            pose,
                            interpolation::part(instance.rig.root, joint, actor.definition_kind()),
                            matrix,
                        )
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let custom = self.player.as_ref().filter(|_| owner == world.player);
            let custom_pose = custom
                .map(|player| player.pose(&matrices, scene.models[instance.rig.joints[0]].local));
            if let (Some(player), Some(target)) = (custom, &custom_pose) {
                let appearance = &player.appearance;
                appearance.meshes.pose_skin(target, None);
                for (index, (part, &matrix)) in appearance.parts.iter().zip(target).enumerate() {
                    visit(
                        interpolation::part(
                            instance.rig.root,
                            instance.rig.joints[tenchu_assets::tenchu2::BONES[index]],
                            actor.definition_kind(),
                        ),
                        &appearance.meshes,
                        part,
                        matrix,
                        ModelStyle::Imported {
                            concealed: instance.translucent,
                        },
                    );
                }
                player_hand = Some(custom::Player::attach(matrices[11], 11, &matrices, target));
            } else {
                meshes.pose_skin(&matrices, None);
                for (index, (part, &matrix)) in body.parts.iter().zip(&matrices).enumerate() {
                    visit(
                        interpolation::part(
                            instance.rig.root,
                            instance.rig.joints[index],
                            actor.definition_kind(),
                        ),
                        meshes,
                        part,
                        matrix,
                        if assets
                            .model_sources
                            .get(&definition.model)
                            .is_some_and(|ids| {
                                ids.iter()
                                    .any(|id| !id.is_empty() && !id.starts_with("tenchu2/"))
                            })
                        {
                            ModelStyle::Imported {
                                concealed: instance.translucent,
                            }
                        } else if instance.translucent {
                            ModelStyle::Concealed
                        } else {
                            ModelStyle::Actor
                        },
                    );
                }
            }
            for root in instance.rig.weapons.iter().take(3).flatten() {
                let name = self
                    .weapon_names
                    .get(root)
                    .ok_or("Missing native weapon instance")?;
                let parts = &assets.weapons[name];
                for (index, part) in parts.iter().enumerate() {
                    let mut matrix = PoseFrame::transform(
                        pose,
                        interpolation::part(
                            instance.rig.root,
                            root + 1 + index,
                            actor.definition_kind(),
                        ),
                        scene.world(root + 1 + index)?,
                    );
                    if let Some(target) = &custom_pose {
                        let mut parent = scene.models[*root].parent;
                        while let Some(id) = parent {
                            if let Some(bone) = instance.rig.joints.iter().position(|&j| j == id) {
                                matrix = custom::Player::attach(matrix, bone, &matrices, target);
                                break;
                            }
                            parent = scene.models[id].parent;
                        }
                    }
                    visit(
                        interpolation::part(
                            instance.rig.root,
                            root + 1 + index,
                            actor.definition_kind(),
                        ),
                        self.imported_weapons.get(name).unwrap_or(&self.weapons),
                        part,
                        matrix,
                        if assets
                            .weapon_imports
                            .get(name)
                            .is_some_and(|m| !m.source_id.starts_with("tenchu2/"))
                        {
                            ModelStyle::Imported {
                                concealed: instance.translucent,
                            }
                        } else {
                            ModelStyle::Actor
                        },
                    );
                }
            }
        }
        Ok(player_hand)
    }
}

#[cfg(test)]
mod tests;
