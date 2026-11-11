use super::*;
use tenchu_core::{effects::Effect, game::Game};

impl Contact {
    pub fn tick(
        &mut self,
        game: &Game,
        surfaces: &Surfaces,
        graphics: &Graphics,
        weather: crate::visuals::weather::SurfaceWeather,
        eye: Vec3,
    ) {
        if !self.begin_tick(game.tick, surfaces, graphics, eye) {
            return;
        }
        let options = &graphics.effects;
        let mut splash_slots = BTreeMap::new();
        let mut wire_slots = BTreeMap::new();
        let mut clash_slots = BTreeMap::new();
        for (slot, effect) in game
            .effects
            .active_slots()
            .iter()
            .enumerate()
            .filter_map(|(i, e)| e.as_ref().map(|e| (i, e)))
        {
            match effect {
                Effect::Impact(impact)
                    if options.sparks
                        && options.spark_density > 0
                        && impact.sprite == 1
                        && impact.time == 9
                        && impact.count == 1
                        && point(impact.position).distance_squared(eye) < 35000.0_f32.powi(2) =>
                {
                    self.flash(point(impact.position));
                }
                Effect::Bleed(spark) if options.sparks && spark.color == [0, 127, 255] => {
                    clash_slots.insert(slot, spark.time);
                    if options.spark_density > 0
                        && point(spark.position).distance_squared(eye) < 35000.0_f32.powi(2)
                        && self
                            .clashes
                            .get(&slot)
                            .is_none_or(|&time| spark.time >= time)
                    {
                        self.flash(point(spark.position));
                    }
                }
                Effect::Splash(s) if options.splashes => {
                    let position = point(s.position);
                    splash_slots.insert(slot, position);
                    if self.splashes.get(&slot) != Some(&position)
                        && position.distance_squared(eye) < 35000.0_f32.powi(2)
                    {
                        let strength = (f32::from(s.size[0]) / 8192.0).clamp(0.5, 3.0);
                        self.splash(position, strength);
                    }
                }
                Effect::FlyWire(w) if options.sparks => {
                    wire_slots.insert(slot, w.mode);
                    if w.mode == 1 && self.wires.get(&slot) != Some(&1) {
                        let hit = surfaces.cast_excluding(
                            point(w.start),
                            point(w.end),
                            render_mesh::FOLIAGE,
                        );
                        let position = hit
                            .map_or_else(|| point(w.end), |hit| hit.position + hit.normal * 40.0);
                        let normal = hit.map_or_else(
                            || {
                                (point(w.start) - point(w.end))
                                    .try_normalize()
                                    .unwrap_or(Vec3::NEG_Y)
                            },
                            |hit| hit.normal,
                        );
                        self.spark(position, normal, options.spark_density, [1.0, 0.64, 0.2]);
                    }
                }
                _ => {}
            }
        }
        self.splashes = splash_slots;
        self.wires = wire_slots;
        self.clashes = clash_slots;
        self.scatter(steps::gaisen(game), surfaces, graphics, weather, eye);
    }
}
