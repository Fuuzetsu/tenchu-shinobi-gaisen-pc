use super::World;

impl World {
    pub fn capture<C: tenchu_assets::checkpoint::Capture>(
        &self,
        out: &mut C,
    ) -> Result<(), C::Error> {
        let Self {
            audio,
            items,
            effects,
            camera,
            terrain,
            environment,
            hidden_scene,
            actors,
            instances,
            scene,
            collision,
            ai,
            random,
            map,
            view_distance,
            keep_all_active,
            script_hidden,
            sequence,
            player,
            difficulty,
            tick,
            script,
            cursor,
            probe,
            water,
            terrain_options,
            tracking,
            travel,
            effort,
            controls,
            statistics,
            mission,
        } = self;
        out.value(audio)?;
        out.value(items)?;
        out.value(effects)?;
        out.value(camera)?;
        out.shared(terrain)?;
        out.value(environment)?;
        out.value(hidden_scene)?;
        out.value(actors)?;
        out.value(instances)?;
        out.value(scene)?;
        out.value(collision)?;
        out.value(ai)?;
        out.value(random)?;
        out.value(map)?;
        out.value(view_distance)?;
        out.value(keep_all_active)?;
        out.value(script_hidden)?;
        out.value(sequence)?;
        out.value(player)?;
        out.value(difficulty)?;
        out.value(tick)?;
        out.value(script)?;
        out.value(cursor)?;
        out.value(probe)?;
        out.value(water)?;
        out.value(terrain_options)?;
        out.value(tracking)?;
        out.value(travel)?;
        out.value(effort)?;
        out.value(controls)?;
        out.value(statistics)?;
        out.value(mission)
    }
}
