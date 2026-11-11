use super::Controls;
use crate::settings::Vibration;
use gilrs::ff::{BaseEffect, BaseEffectType, EffectBuilder, Envelope, Repeat, Replay, Ticks};
use tenchu_core::{effects::Rumble, game::Game};

impl Controls {
    pub fn stop_rumble(&mut self) {
        self.native_rumble = None;
        if let Some(effect) = self.rumble.take() {
            let _ = effect.stop();
        }
    }

    pub fn feedback(&mut self, game: &Game, enabled: bool, vibration: &Vibration) {
        if !enabled || vibration.intensity == 0 || game.cinematic.active {
            self.stop_rumble();
            return;
        }
        let Some(request) = game
            .effects
            .rumble
            .iter()
            .rev()
            .find(|r| r.actor == game.player_id)
        else {
            return;
        };
        self.stop_rumble();
        let gain = vibration.gain(request.action);
        if gain == 0.0 {
            return;
        }
        self.play_feedback(effect(request, gain));
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn native_feedback(
        &mut self,
        feedback: Option<&tenchu_core::tenchu2::feedback::Feedback>,
        enabled: bool,
        vibration: &Vibration,
    ) {
        let motors = if enabled {
            native_motors(feedback, vibration)
        } else {
            [0; 2]
        };
        if motors == [0; 2] {
            self.stop_rumble();
            return;
        }
        let selection = (self.pad.active, motors);
        if self.native_rumble == Some(selection) && self.rumble.is_some() {
            return;
        }
        self.stop_rumble();
        let mut effect = EffectBuilder::new();
        for kind in [
            BaseEffectType::Weak {
                magnitude: motors[0],
            },
            BaseEffectType::Strong {
                magnitude: motors[1],
            },
        ] {
            effect.add_effect(BaseEffect {
                kind,
                scheduling: Replay {
                    play_for: Ticks::from_ms(1000),
                    ..Default::default()
                },
                ..Default::default()
            });
        }
        // The source input poll expires this constant signal in logic ticks.
        effect.repeat(Repeat::Infinitely);
        self.play_feedback(effect);
        if self.rumble.is_some() {
            self.native_rumble = Some(selection);
        }
    }

    fn play_feedback(&mut self, mut builder: EffectBuilder) {
        let Some(gilrs) = &mut self.gilrs else {
            return;
        };
        let Some((id, _)) = gilrs
            .gamepads()
            .find(|(id, pad)| Some(usize::from(*id)) == self.pad.active && pad.is_ff_supported())
        else {
            return;
        };
        match builder.gamepads(&[id]).finish(gilrs).and_then(|effect| {
            effect.play()?;
            Ok(effect)
        }) {
            Ok(effect) => {
                self.rumble = Some(effect);
                self.vibration_error = None;
            }
            Err(error) => {
                eprintln!("Controller vibration: {error}");
                self.vibration_error = Some(error.to_string());
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn native_motors(
    feedback: Option<&tenchu_core::tenchu2::feedback::Feedback>,
    vibration: &Vibration,
) -> [u16; 2] {
    let Some(feedback) = feedback else {
        return [0; 2];
    };
    let Some(action) = feedback.action else {
        return [0; 2];
    };
    let gain = vibration.gain(action);
    [
        u16::from(feedback.motors[0] != 0) * u16::MAX,
        u16::from(feedback.motors[1]) * 257,
    ]
    .map(|value| {
        (f32::from(value) * gain)
            .round()
            .clamp(0.0, f32::from(u16::MAX)) as u16
    })
}

fn effect(request: &Rumble, gain: f32) -> EffectBuilder {
    // PadProc advances the source envelope twice per 60 Hz callback. The host
    // force-feedback driver quantizes these durations to its supported cadence.
    let duration = |n: u16| Ticks::from_ms((u32::from(n) * 1000).div_ceil(120));
    let attack = duration(request.attack);
    let release = duration(request.release);
    let total = attack + release + Ticks::from_ms(1);
    let mut effect = EffectBuilder::new();
    // Scale the motors directly; gilrs' effect-level gain mutes values below 5%.
    effect.add_effect(BaseEffect {
        kind: BaseEffectType::Strong {
            magnitude: u16::from(request.power) * 257,
        } * gain,
        scheduling: Replay {
            play_for: total,
            ..Default::default()
        },
        envelope: Envelope {
            attack_length: attack,
            fade_length: release,
            ..Default::default()
        },
    });
    if request.attack != 0 {
        effect.add_effect(BaseEffect {
            kind: BaseEffectType::Weak {
                magnitude: u16::MAX,
            } * gain,
            scheduling: Replay {
                play_for: attack,
                with_delay: total,
                ..Default::default()
            },
            ..Default::default()
        });
    }
    effect.repeat(Repeat::For(total));
    effect
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use tenchu_core::{effects::RumbleAction, tenchu2::feedback::Feedback};

    #[test]
    fn native_motor_gains_mute_independently_and_clamp_to_driver_range() {
        let mut feedback = Feedback::default();
        feedback.pulse(RumbleAction::Damage, 4, 1);
        let mut vibration = Vibration::default();
        assert_eq!(native_motors(Some(&feedback), &vibration), [65535, 64764]);
        vibration.intensity = 50;
        vibration.actions.insert(RumbleAction::Damage, 50);
        assert_eq!(native_motors(Some(&feedback), &vibration), [16384, 16191]);
        vibration.actions.insert(RumbleAction::Damage, 0);
        assert_eq!(native_motors(Some(&feedback), &vibration), [0; 2]);
        feedback.pulse(RumbleAction::Attack, 1, 2);
        assert_eq!(native_motors(Some(&feedback), &vibration), [0, 8096]);
        vibration.intensity = 200;
        vibration.actions.insert(RumbleAction::Attack, 200);
        assert_eq!(native_motors(Some(&feedback), &vibration), [0, 64764]);
        feedback.drowning(255);
        assert_eq!(native_motors(Some(&feedback), &vibration), [65535; 2]);
        for _ in 0..5 {
            feedback.poll();
        }
        assert_eq!(native_motors(Some(&feedback), &vibration), [0; 2]);
        assert_eq!(native_motors(None, &vibration), [0; 2]);
        feedback.pulse(RumbleAction::Attack, 4, 2);
        vibration.intensity = 0;
        assert_eq!(native_motors(Some(&feedback), &vibration), [0; 2]);

        let mut controls = Controls::default();
        controls.native_feedback(Some(&feedback), true, &Vibration::default());
        assert!(controls.rumble.is_none() && controls.native_rumble.is_none());
        controls.native_feedback(None, false, &vibration);
        assert!(controls.rumble.is_none() && controls.native_rumble.is_none());
    }
}
