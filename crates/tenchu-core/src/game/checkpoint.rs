use super::Game;

impl Game {
    pub fn capture<C: tenchu_assets::checkpoint::Capture>(
        &self,
        out: &mut C,
    ) -> std::result::Result<(), C::Error> {
        let Self {
            trial,
            mission,
            math,
            roster,
            player_id,
            statistics,
            random,
            effects,
            items,
            misc,
            hud,
            tick,
            clock,
            input,
            rendered_view,
            cinematic,
            events,
            music,
            scene,
            collision,
            camera,
            intelligence,
            combat,
            visible,
            activation,
            drawing,
            construction,
            moveset,
            navigation,
            walking,
            body,
            commands,
            cursor,
            sound_appearance,
        } = self;
        out.value(trial)?;
        mission.capture(out)?;
        out.shared(math)?;
        out.value(roster)?;
        out.value(player_id)?;
        out.value(statistics)?;
        out.value(random)?;
        out.value(effects)?;
        out.value(items)?;
        out.value(misc)?;
        out.value(hud)?;
        out.value(tick)?;
        out.value(clock)?;
        out.value(input)?;
        out.value(rendered_view)?;
        out.value(cinematic)?;
        out.value(events)?;
        out.value(music)?;
        out.value(scene)?;
        out.value(collision)?;
        out.value(camera)?;
        out.value(intelligence)?;
        out.value(combat)?;
        out.value(visible)?;
        out.value(activation)?;
        out.value(drawing)?;
        out.value(construction)?;
        out.value(moveset)?;
        out.value(navigation)?;
        out.value(walking)?;
        out.value(body)?;
        out.value(commands)?;
        out.value(cursor)?;
        out.value(sound_appearance)?;
        Ok(())
    }
}

impl Game {
    pub fn same_scene(&self, previous: &Self) -> bool {
        let a = &self.mission;
        let b = &previous.mission;
        if a.stage != b.stage
            || a.trial.is_some() != b.trial.is_some()
            || self.player().kind != previous.player().kind
        {
            return false;
        }
        if a.scripts.language != b.scripts.language
            || a.placements != b.placements
            || a.map != b.map
            || a.objects != b.objects
            || a.models != b.models
            || a.common_models != b.common_models
            || a.weapons.models != b.weapons.models
            || a.imported_models != b.imported_models
            || a.world_vram != b.world_vram
            || a.imported_voices != b.imported_voices
        {
            return false;
        }
        true
    }

    pub fn reuse_scene(&mut self, previous: &mut Self) {
        let a = &mut self.mission;
        let b = &mut previous.mission;
        std::mem::swap(&mut a.map, &mut b.map);
        std::mem::swap(&mut a.objects, &mut b.objects);
        std::mem::swap(&mut a.models, &mut b.models);
        std::mem::swap(&mut a.common_models, &mut b.common_models);
        std::mem::swap(&mut a.weapons.models, &mut b.weapons.models);
        std::mem::swap(&mut a.world_vram, &mut b.world_vram);
    }
}
