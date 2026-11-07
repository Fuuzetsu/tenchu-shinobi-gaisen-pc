use super::*;

impl Library {
    pub fn load_tenchu2(&mut self, disc: &tenchu_assets::disc::Disc<impl ReadAt>) -> Result<()> {
        let edition = tenchu_assets::tenchu2::Edition::detect(disc)?;
        let models = tenchu_assets::tenchu2::characters::load(disc, None)?;
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
            let body = Body {
                id: id.clone(),
                name: format!("{label} · {}", edition.name()),
                resource: format!("tenchu2/actors/{}", model.name),
                parts: model.parts,
                vram: Arc::new(model.vram),
                character: behavior.character.clone(),
                clips: Arc::new(clips),
                idle,
                think: behavior.think,
                voice: behavior.voice,
            };
            self.aliases.insert(id, self.bodies.len());
            self.bodies.push(body);
        }
        Ok(())
    }
}
