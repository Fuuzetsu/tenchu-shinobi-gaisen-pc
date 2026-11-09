use crate::{Result, motion::Pose};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tenchu_assets::{
    model::Part,
    motion::Key,
    tenchu2::{motions::Clip, registry::Registry},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Track {
    clip: i16,
    bone: Option<usize>,
}

impl Track {
    fn keys<'a>(&self, clips: &'a BTreeMap<i16, Clip>) -> Result<(&'a [Key], i16)> {
        let clip = clips.get(&self.clip).ok_or("Missing Tenchu 2 motion")?;
        let keys = match self.bone {
            Some(bone) => clip.bones.get(bone).ok_or("Missing Tenchu 2 joint track")?,
            None => &clip.root,
        };
        Ok((keys, clip.duration))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Motion {
    pub action: i16,
    pub clip: Option<i16>,
    pub frame: i16,
    pub loops: i16,
    pub mask: u32,
    pub pose: Pose,
    tracks: Vec<Track>,
}

impl Motion {
    pub fn new(parts: &[Part]) -> Result<Self> {
        Ok(Self {
            action: -1,
            clip: None,
            frame: 0,
            loops: 0,
            mask: u32::MAX,
            pose: Pose::new(parts)?,
            tracks: Vec::new(),
        })
    }

    pub fn select(
        &mut self,
        action: i16,
        kind: i16,
        registry: &Registry,
        clips: &BTreeMap<i16, Clip>,
    ) -> Result<Option<i16>> {
        let id = if action == -1 {
            self.clip
        } else {
            registry.clip(kind, action)
        };
        let Some(id) = id else {
            return Ok(None);
        };
        self.select_clip(action, id, clips)?;
        Ok(Some(id))
    }

    pub fn select_clip(&mut self, action: i16, id: i16, clips: &BTreeMap<i16, Clip>) -> Result<()> {
        let clip = clips
            .get(&id)
            .ok_or_else(|| format!("Missing Tenchu 2 motion {id}"))?;
        let count = clip.bones.len().min(self.pose.rotation.len());
        self.action = action;
        self.clip = Some(id);
        self.frame = clip.blend.wrapping_neg();
        self.loops = 0;
        self.mask |= 0x8000_0000;
        self.tracks.clear();
        self.tracks.push(Track {
            clip: id,
            bone: None,
        });
        self.tracks.extend((0..count).map(|bone| Track {
            clip: id,
            bone: Some(bone),
        }));
        for rotation in self.pose.rotation.iter_mut().take(count) {
            for angle in rotation {
                let value = i32::from(*angle);
                *angle = (if value.abs() > 2048 {
                    value - value.signum() * 4096
                } else {
                    value
                } % 4096) as i16;
            }
        }
        Ok(())
    }

    pub fn overlay(
        &mut self,
        action: i16,
        mask: u32,
        kind: i16,
        registry: &Registry,
        clips: &BTreeMap<i16, Clip>,
    ) -> Result<Option<i16>> {
        let previous = (self.action, self.clip, self.frame, self.loops);
        let tracks = self.tracks.clone();
        let selected = self.select(action, kind, registry, clips)?;
        (self.action, self.clip, self.frame, self.loops) = previous;
        // The source overlay retains its new final joint even outside the mask.
        let restored = self.tracks.len().saturating_sub(1);
        for (i, (track, old)) in self
            .tracks
            .iter_mut()
            .zip(tracks)
            .take(restored)
            .enumerate()
        {
            let bone = i.saturating_sub(1);
            if mask & 1u32.wrapping_shl(bone as u32) == 0 {
                *track = old;
            }
        }
        Ok(selected)
    }

    pub fn advance(
        &mut self,
        clips: &BTreeMap<i16, Clip>,
        basis: &[[i16; 4]; 33],
        update_pose: bool,
    ) -> Result<i16> {
        if self.loops < 0 {
            return Ok(-1);
        }
        let clip = clips
            .get(&self.clip.ok_or("Tenchu 2 motion not selected")?)
            .ok_or("Missing Tenchu 2 motion")?;
        if update_pose {
            self.update_pose(clips, basis, clip.duration == 0)?;
        }
        self.frame = self.frame.wrapping_add(1);
        if self.frame > clip.duration {
            self.frame = 0;
            self.loops = self.loops.wrapping_add(1);
        }
        Ok(self.frame)
    }

    pub fn publish(
        &self,
        scene: &mut crate::scene::Scene,
        joints: &[crate::scene::ModelId],
        math: &crate::math::Math,
    ) {
        if let Some(&root) = joints.first() {
            scene.set_translation(root, self.pose.translation[0]);
        }
        self.pose.publish(
            scene,
            joints,
            math,
            self.mask,
            self.tracks.len().saturating_sub(1),
        );
    }

    fn update_pose(
        &mut self,
        clips: &BTreeMap<i16, Clip>,
        basis: &[[i16; 4]; 33],
        held: bool,
    ) -> Result<()> {
        let blend = self.frame < 0;
        let target = |track: &Track| -> Result<Option<[i16; 3]>> {
            let (keys, duration) = track.keys(clips)?;
            if blend || held {
                return Ok(Some(keys.first().ok_or("Empty Tenchu 2 track")?.value));
            }
            if self.frame > duration {
                return Ok(None);
            }
            crate::motion::sample(keys, self.frame, duration, basis).map(Some)
        };
        if self.mask & 1 != 0
            && let Some(root) = target(self.tracks.first().ok_or("Missing Tenchu 2 root track")?)?
        {
            let root = [
                i32::from(root[0]),
                (i32::from(root[1]) * i32::from(self.pose.root_height)) >> 12,
                i32::from(root[2]),
            ];
            for (current, target) in self.pose.translation[0].iter_mut().zip(root) {
                *current = if blend {
                    current.wrapping_add(target.wrapping_sub(*current) / -i32::from(self.frame))
                } else {
                    target
                };
            }
        }
        if !blend && !held && self.mask & 0x8000_0000 == 0 {
            self.pose.translation[0].x = 0;
            self.pose.translation[0].z = 0;
        }
        for (bone, (rotation, track)) in self
            .pose
            .rotation
            .iter_mut()
            .zip(self.tracks.iter().skip(1))
            .enumerate()
        {
            if self.mask & 1u32.wrapping_shl(bone as u32) == 0 {
                continue;
            }
            if let Some(target) = target(track)? {
                for (current, target) in rotation.iter_mut().zip(target) {
                    *current = if blend {
                        (i32::from(*current)
                            + (i32::from(target) - i32::from(*current)) / -i32::from(self.frame))
                            as i16
                    } else {
                        target
                    };
                }
            }
        }
        if held && !blend {
            self.loops = -2;
        }
        Ok(())
    }
}
