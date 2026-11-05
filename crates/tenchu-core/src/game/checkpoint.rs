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
