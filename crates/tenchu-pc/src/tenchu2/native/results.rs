use std::sync::Arc;
use tenchu_assets::{ReadAt, Result, disc::Disc, tenchu2::frontend};
use tenchu_core::tenchu2::progress;

pub struct Results {
    pub resources: frontend::Results,
    pub values: progress::Results,
    pub bank: Arc<tenchu_assets::audio::bank::Bank>,
    pub exit: Option<(super::campaign::Action, u8)>,
}

impl Results {
    pub fn load(disc: &Disc<impl ReadAt>, values: progress::Results) -> Result<Self> {
        Ok(Self {
            resources: frontend::Results::load(disc)?,
            values,
            bank: Arc::new(tenchu_assets::tenchu2::audio::menu(disc)?),
            exit: None,
        })
    }
}
