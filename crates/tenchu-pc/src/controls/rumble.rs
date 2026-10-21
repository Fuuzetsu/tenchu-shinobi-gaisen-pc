use super::Controls;
use gilrs::ff::{BaseEffect, BaseEffectType, EffectBuilder, Envelope, Repeat, Replay, Ticks};
use tenchu_core::{effects::Rumble, game::Game};

impl Controls {
    pub fn stop_rumble(&mut self) {
        if let Some(effect) = self.rumble.take() {
            let _ = effect.stop();
        }
    }

    pub fn feedback(&mut self, game: &Game, enabled: bool) {
        if !enabled || game.cinematic.active {
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
        let Some(gilrs) = &mut self.gilrs else {
            return;
        };
        let Some((id, _)) = gilrs
            .gamepads()
            .find(|(id, pad)| Some(usize::from(*id)) == self.pad.active && pad.is_ff_supported())
        else {
            return;
        };
        let mut builder = effect(request);
        match builder.gamepads(&[id]).finish(gilrs).and_then(|effect| {
            effect.play()?;
            Ok(effect)
        }) {
            Ok(effect) => self.rumble = Some(effect),
            Err(error) => eprintln!("Controller vibration: {error}"),
        }
    }
}

fn effect(request: &Rumble) -> EffectBuilder {
    // PadProc advances the source envelope twice per 60 Hz callback. The host
    // force-feedback driver quantizes these durations to its supported cadence.
    let duration = |n: u16| Ticks::from_ms((u32::from(n) * 1000).div_ceil(120));
    let attack = duration(request.attack);
    let release = duration(request.release);
    let total = attack + release + Ticks::from_ms(1);
    let mut effect = EffectBuilder::new();
    effect.add_effect(BaseEffect {
        kind: BaseEffectType::Strong {
            magnitude: u16::from(request.power) * 257,
        },
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
            },
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
