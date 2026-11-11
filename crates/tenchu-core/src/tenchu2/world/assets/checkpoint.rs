use super::Assets;
use tenchu_assets::checkpoint::Capture;

impl Assets {
    pub fn capture<C: Capture>(&self, out: &mut C) -> Result<(), C::Error> {
        let Self {
            campaign,
            ambient_interval,
            items,
            sprites,
            effect_rules,
            camera,
            hero,
            sequence,
            draw_distance,
            resources,
            cast,
            level,
            models,
            weapons,
            weapon_vram,
            weapon_imports,
            model_sources,
            object_sources,
            gaisen_scrolls,
            gaisen_pickups,
            loot,
            terrain_rules,
            combat_rules,
            spline,
        } = self;
        out.value(campaign)?;
        out.value(ambient_interval)?;
        items.capture(out)?;
        let tenchu_assets::tenchu2::effects::Sprites {
            vram,
            effects,
            scenery,
            items,
        } = sprites;
        out.shared(vram)?;
        out.shared(effects)?;
        out.shared(scenery)?;
        out.shared(items)?;
        out.value(effect_rules)?;
        out.value(camera)?;
        out.value(hero)?;
        out.value(sequence)?;
        out.value(draw_distance)?;
        let crate::tenchu2::actor::Resources {
            registry,
            clips,
            weapons: weapon_rules,
            commands,
            math,
        } = resources;
        out.value(registry)?;
        out.shared(clips)?;
        out.value(weapon_rules)?;
        out.value(commands)?;
        out.shared_arc(math, true)?;
        out.value(cast)?;
        level.capture(out)?;
        out.value(&models.len())?;
        for (name, model) in models {
            out.value(name)?;
            let tenchu_assets::tenchu2::characters::Rig { parts, vram } = model;
            out.shared(parts)?;
            out.shared(vram)?;
        }
        out.shared(weapons)?;
        out.shared(weapon_vram)?;
        out.value(&weapon_imports.len())?;
        for (name, model) in weapon_imports {
            out.value(name)?;
            out.shared_arc(model, true)?;
        }
        out.value(model_sources)?;
        out.value(object_sources)?;
        out.value(gaisen_scrolls)?;
        out.value(gaisen_pickups)?;
        out.value(loot)?;
        out.value(terrain_rules)?;
        out.value(combat_rules)?;
        for row in spline {
            out.value(row)?;
        }
        Ok(())
    }
}
