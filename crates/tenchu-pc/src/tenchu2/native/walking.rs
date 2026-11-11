use super::*;

impl Session {
    pub fn configure_walking(&mut self, disc: &Disc<impl ReadAt>, enabled: bool) -> Result<()> {
        let clip = if enabled {
            let kind = if self.world.actors[self.world.player].actor.kind == 1 {
                0x22
            } else {
                0x20
            };
            let registry = &self.assets.resources.registry;
            let id = registry
                .clip(kind, 3)
                .ok_or("Missing native T2 ninja walk")?;
            if let std::collections::btree_map::Entry::Vacant(entry) =
                self.assets.resources.clips.entry(id)
            {
                let mut clips = tenchu_assets::tenchu2::motions::collect(disc, [id].into())?;
                let clip = clips
                    .remove(&id)
                    .ok_or("Missing native T2 walking motion")?;
                entry.insert(clip.at_gameplay_rate(registry.edition));
            }
            Some(id)
        } else {
            None
        };
        self.world.set_player_walk(clip, &self.assets)
    }
}

#[cfg(test)]
mod tests;
