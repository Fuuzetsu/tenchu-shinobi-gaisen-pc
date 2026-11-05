use super::Light;
use glam::Vec3;

pub fn phase(position: Vec3) -> f32 {
    position
        .dot(Vec3::new(0.0137, 0.0091, 0.0073))
        .rem_euclid(std::f32::consts::TAU)
}

pub fn motion(light: &Light, time: f32, strength: u16) -> (Vec3, f32) {
    if !light.flame || strength == 0 {
        return (Vec3::ZERO, 1.0);
    }
    let strength = f32::from(strength) / 100.0;
    let phase = phase(light.position);
    let wave = |rate: f32, shift: f32| {
        (time * rate + phase + shift).sin() * 0.6
            + (time * rate * 1.73 + phase * 2.0 - shift).sin() * 0.4
    };
    (
        Vec3::new(
            wave(3.1, 0.0) * 40.0,
            wave(4.3, 2.0) * 15.0,
            wave(2.7, 4.0) * 40.0,
        ) * strength,
        1.0 + wave(5.1, 1.0) * 0.08 * strength,
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn older_settings_remain_steady_and_modern_quality_is_explicit() {
        let mut old: crate::settings::Settings = serde_json::from_str(
            r#"{"graphics":{"lighting":{"enabled":true},"post":{"enabled":true}}}"#,
        )
        .unwrap();
        assert_eq!(old.graphics.lighting.flame_motion, 0);
        assert_eq!(old.graphics.lighting.heat_shimmer, 0);
        for (preset, motion, heat) in [(0, 0, 0), (8, 35, 25), (9, 50, 40), (11, 0, 0), (12, 0, 0)]
        {
            old.graphics.set_preset(preset);
            assert_eq!(old.graphics.lighting.flame_motion, motion);
            assert_eq!(old.graphics.lighting.heat_shimmer, heat);
        }
        old.graphics.lighting.flame_motion = 31;
        old.graphics.lighting.heat_shimmer = 47;
        let restored: crate::settings::Settings =
            serde_json::from_slice(&serde_json::to_vec(&old).unwrap()).unwrap();
        assert_eq!(restored.graphics, old.graphics);
    }
}
