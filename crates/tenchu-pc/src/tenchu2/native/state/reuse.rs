use super::*;

impl Owned {
    pub(in crate::tenchu2::native) fn finish_reuse(&self, session: &mut Session) {
        if let (Some(previous), Some(current)) = (&self.gaisen, &mut session.gaisen) {
            previous.finish_reuse(current);
        }
    }

    pub(in crate::tenchu2::native) fn reusable(&self, session: &Session) -> Result<bool> {
        self.validate()?;
        let a = &self.core.1;
        let b = &session.assets;
        Ok(a.hero == b.hero
            && a.sequence == b.sequence
            && a.level == b.level
            && a.items == b.items
            && a.sprites == b.sprites
            && a.models == b.models
            && a.weapons == b.weapons
            && a.weapon_vram == b.weapon_vram
            && a.weapon_imports == b.weapon_imports
            && a.model_sources == b.model_sources
            && a.object_sources == b.object_sources
            && self
                .core
                .0
                .environment
                .texture_scrolls
                .iter()
                .map(|s| s.destination)
                .eq(session
                    .world
                    .environment
                    .texture_scrolls
                    .iter()
                    .map(|s| s.destination))
            && self.models.same_body(&session.models)?
            && match (&self.gaisen, &session.gaisen) {
                (None, None) => true,
                (Some(saved), Some(current)) => saved.reusable(current)?,
                _ => false,
            })
    }

    pub(in crate::tenchu2::native) fn swap(&mut self, session: &mut Session) {
        let Self {
            signature: _,
            core: (world, assets, frame),
            feedback,
            contact,
            bank,
            actor_voices,
            pending,
            interpolation,
            navigation,
            circle_action,
            models,
            gaisen,
        } = self;
        let previous = &mut session.assets;
        // Keep the geometry allocations referenced by the current renderer.
        std::mem::swap(&mut assets.level, &mut previous.level);
        std::mem::swap(&mut assets.items, &mut previous.items);
        std::mem::swap(&mut assets.sprites, &mut previous.sprites);
        std::mem::swap(&mut assets.models, &mut previous.models);
        std::mem::swap(&mut assets.weapons, &mut previous.weapons);
        std::mem::swap(&mut assets.weapon_imports, &mut previous.weapon_imports);
        std::mem::swap(world, &mut session.world);
        std::mem::swap(assets, &mut session.assets);
        std::mem::swap(frame, &mut session.frame);
        std::mem::swap(feedback, &mut session.feedback);
        std::mem::swap(contact, &mut session.contact);
        std::mem::swap(bank, &mut session.bank);
        std::mem::swap(actor_voices, &mut session.actor_voices);
        std::mem::swap(pending, &mut session.pending);
        std::mem::swap(interpolation, &mut session.interpolation);
        std::mem::swap(navigation, &mut session.navigation);
        std::mem::swap(circle_action, &mut session.circle_action);
        std::mem::swap(models, &mut session.models);
        if let (Some(saved), Some(current)) = (gaisen, &mut session.gaisen) {
            saved.swap(current);
        }
    }
}
