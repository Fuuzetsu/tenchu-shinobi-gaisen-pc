use super::{Custom, Override};
use tenchu_assets::checkpoint::Capture;

impl Custom {
    pub(super) fn capture<C: Capture>(&self, out: &mut C) -> Result<(), C::Error> {
        let Self { id, pack } = self;
        out.value(id)?;
        out.shared_arc(pack, true)
    }
}

impl Override {
    pub(in crate::tenchu2::native) fn capture<C: Capture>(
        &self,
        out: &mut C,
    ) -> Result<(), C::Error> {
        let Self {
            applied,
            custom,
            voice,
            weapon_voice,
            original,
            weapon_pool,
            voice_id,
        } = self;
        out.value(applied)?;
        out.value(&custom.is_some())?;
        if let Some(custom) = custom {
            custom.capture(out)?;
        }
        for voice in [voice, weapon_voice] {
            out.value(&voice.is_some())?;
            if let Some(voice) = voice {
                out.shared_arc(voice, true)?;
            }
        }
        out.value(original)?;
        out.value(weapon_pool)?;
        out.value(voice_id)
    }
}
