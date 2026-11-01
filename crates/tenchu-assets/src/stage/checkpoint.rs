use super::MissionAssets;

impl MissionAssets {
    pub fn capture<C: crate::checkpoint::Capture>(
        &self,
        out: &mut C,
    ) -> std::result::Result<(), C::Error> {
        let Self {
            trial,
            misc,
            stage,
            map,
            map_image,
            map_placement,
            objects,
            object_names,
            placements,
            terrain,
            balma_terrain,
            vram,
            images,
            characters,
            models,
            imported_models,
            imported_voices,
            clips,
            common_motions,
            weapons,
            death_events,
            common_models,
            enemies,
            construction_actors,
            construction_pickups,
            pickups,
            disguise,
            reinforcements,
            reinforcement_overrides,
            story,
            construction,
            layout,
            layout_index,
            random_layouts,
            scripts,
            font,
            tanka,
        } = self;
        out.value(trial)?;
        out.value(misc)?;
        out.value(stage)?;
        out.shared_arc(map, true)?;
        out.value(map_image)?;
        out.value(map_placement)?;
        out.shared_arc(objects, true)?;
        out.value(object_names)?;
        out.value(placements)?;
        out.shared(terrain)?;
        out.shared(balma_terrain)?;
        vram.capture(out)?;
        out.value(images)?;
        out.value(characters)?;
        out.shared_arc(models, true)?;
        out.shared_arc(imported_models, true)?;
        out.value(imported_voices)?;
        out.shared(clips)?;
        out.value(common_motions)?;
        out.value(weapons)?;
        out.value(death_events)?;
        out.shared_arc(common_models, true)?;
        out.value(enemies)?;
        out.value(construction_actors)?;
        out.value(construction_pickups)?;
        out.value(pickups)?;
        out.value(disguise)?;
        out.value(reinforcements)?;
        out.value(reinforcement_overrides)?;
        out.value(story)?;
        out.value(construction)?;
        out.value(layout)?;
        out.value(layout_index)?;
        out.value(random_layouts)?;
        out.value(scripts)?;
        out.value(font)?;
        out.value(tanka)?;
        Ok(())
    }
}
