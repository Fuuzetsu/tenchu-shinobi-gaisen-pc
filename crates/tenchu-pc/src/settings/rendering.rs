#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Rendering {
    pub half_resolution_ao: bool,
    pub adaptive_shadows: bool,
    pub cached_blood: bool,
    pub quarter_resolution_shadows: bool,
    #[cfg(test)]
    pub shared_forward_lighting: bool,
}

impl Default for Rendering {
    fn default() -> Self {
        Self {
            half_resolution_ao: true,
            adaptive_shadows: true,
            cached_blood: true,
            quarter_resolution_shadows: true,
            #[cfg(test)]
            shared_forward_lighting: true,
        }
    }
}
