use super::*;
mod moveset;

impl Library {
    pub fn load_tenchu2(&mut self, disc: &tenchu_assets::disc::Disc<impl ReadAt>) -> Result<()> {
        let edition = tenchu_assets::tenchu2::Edition::detect(disc)?;
        let models = tenchu_assets::tenchu2::characters::load(disc, None)?;
        let registry = tenchu_assets::tenchu2::registry::Registry::load(disc)?;
        let voices = tenchu_assets::tenchu2::audio::voices(disc, &registry)?;
        let movesets = moveset::load(disc, &registry, &voices)?;
        let suzaku = models
            .iter()
            .find(|m| m.name == "tatu2")
            .and_then(|m| m.kinds.iter().copied().find(|k| movesets.contains_key(k)));
        for model in models {
            let id = format!("{}{}", edition.model_prefix(), model.name);
            if self.aliases.contains_key(&id) {
                continue;
            }
            let family = match model.name.as_str() {
                "rikimaru" | "rikia" => "RIKIMARU",
                "ayame" | "ayamea" => "AYAME",
                "tatumaru" | "tatu2" | "tatua" | "tatua_m" | "tatu2_m" => "NINJA",
                "cat" => "CAT",
                "inu" => "DOG",
                "wolf" | "ninken" => "WOLF",
                "kuma" | "tora" => "KUMA",
                "crow" | "taka" | "niwatori" => "KARASU",
                _ => "BUSI",
            };
            let behavior = self
                .bodies
                .iter()
                .find(|b| b.resource == format!("HUMAN/{family}.MAD") && b.character.is_some())
                .or_else(|| self.get("disc:16"))
                .ok_or("Missing Gaisen fallback behavior")?;
            let mut clips = (*behavior.clips).clone();
            let idle = model.idle.map(|mut idle| {
                // Preview-only clip: keep Gaisen registrations and combat intact.
                idle.id = i16::MAX;
                clips.push(idle);
                clips.len() - 1
            });
            let label = match model.name.as_str() {
                "rikimaru" => "Rikimaru",
                "ayame" => "Ayame",
                "tatumaru" => "Tatsumaru",
                "rikia" => "Rikimaru (alternate)",
                "ayamea" => "Ayame (alternate)",
                "tatua" => "Tatsumaru (alternate)",
                "tatua_m" => "Tatsumaru (alternate, masked)",
                "tatu2_m" => "Tatsumaru (Suzaku, masked)",
                "tatu2" => "Tatsumaru (Suzaku)",
                "siun" => "Shiunsai",
                "matushin" => "Matsunoshin",
                "seki" => "Sekiya",
                "kei" => "Lady Kei",
                "suzaku" => "Suzaku",
                "kagami" => "Lady Kagami",
                "motohide" => "Motohide",
                "toda" => "Lord Toda",
                name => name,
            };
            let kind = moveset::hero(&model.name)
                .or_else(|| model.kinds.first().copied())
                .or_else(|| (model.name == "tatu2_m").then_some(suzaku).flatten())
                .ok_or("Tenchu 2 model has no source actor definition")?;
            let native = Native {
                kind,
                rig: tenchu_assets::tenchu2::characters::rig(disc, &model.name)?,
            };
            let body = Body {
                id: id.clone(),
                name: format!("{label} · {}", edition.name()),
                resource: format!("tenchu2/actors/{}", model.name),
                parts: model.parts,
                vram: Arc::new(model.vram),
                character: behavior.character.clone(),
                imported_moveset: moveset::hero(&model.name)
                    .or_else(|| (model.name == "tatu2_m").then_some(suzaku).flatten())
                    .or_else(|| {
                        model
                            .kinds
                            .iter()
                            .copied()
                            .find(|kind| movesets.contains_key(kind))
                    })
                    .and_then(|kind| movesets.get(&kind))
                    .cloned(),
                clips: Arc::new(clips),
                idle,
                think: behavior.think,
                voice_set: voices.get(&kind).cloned(),
                native: Some(native),
                voice: crate::tenchu2::voices::donor_kind(&model.name)
                    .and_then(|kind| self.get(&format!("disc:{kind}")))
                    .and_then(|body| body.voice)
                    .or(behavior.voice),
            };
            self.aliases.insert(id, self.bodies.len());
            self.bodies.push(body);
        }
        Ok(())
    }
}
