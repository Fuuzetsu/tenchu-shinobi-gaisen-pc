use std::sync::Arc;
use tenchu_assets::{ReadAt, Result, audio::stream::Clip, disc::Disc, tenchu2::frontend};
use tenchu_core::tenchu2::introduction::{Introduction as State, Phase};

pub struct Introduction {
    pub resources: frontend::Introduction,
    pub state: State,
    pub audio: Option<Arc<Clip>>,
    vblanks: u8,
}

impl Introduction {
    pub fn load(disc: &Disc<impl ReadAt>, resource: u16) -> Result<Self> {
        let resources = frontend::Introduction::load(disc, resource)?;
        let audio = resources
            .narration
            .as_ref()
            .map(|n| Clip::load(disc, &n.audio).map(Arc::new))
            .transpose()?;
        let state = State::new(audio.is_some());
        Ok(Self {
            resources,
            state,
            audio,
            vblanks: 0,
        })
    }

    pub fn step(&mut self, buttons: u16) -> bool {
        let mut start_audio = false;
        self.vblanks += 2;
        while self.state.phase != Phase::Finished && self.vblanks >= self.state.vblanks() {
            self.vblanks -= self.state.vblanks();
            let previous = self.state.phase;
            let finished = self.audio.as_ref().is_none_or(|a| {
                u64::from(self.state.tick) * u64::from(a.rate) >= a.frames.len() as u64 * 15
            });
            self.state.step(buttons, finished);
            start_audio |= previous == Phase::Title && self.state.phase == Phase::Narration;
        }
        start_audio
    }
}

pub struct Movie {
    pub frame: Option<Arc<tenchu_assets::movie::Frame>>,
    pub audio: Option<Arc<Clip>>,
    pub finished: bool,
    pub quit: bool,
    stream: Option<tenchu_assets::movie::Stream>,
    last_frame: u32,
    ticks: u32,
    previous: u16,
}

impl Movie {
    pub fn seconds(&self) -> f64 {
        f64::from(self.ticks) / 30.0
    }

    pub fn load(disc: &Disc<impl ReadAt>, resource: u16) -> Result<Self> {
        let descriptor = frontend::Movie::load(disc, resource)?.map(|mut descriptor| {
            descriptor.last_frame -= 3;
            descriptor
        });
        Self::open(disc, descriptor)
    }

    pub fn title(disc: &Disc<impl ReadAt>, opening: bool) -> Result<Self> {
        let mut movie = Self::open(disc, Some(frontend::Movie::title(disc, opening)?))?;
        movie.previous = u16::MAX;
        Ok(movie)
    }

    fn open(disc: &Disc<impl ReadAt>, descriptor: Option<frontend::Movie>) -> Result<Self> {
        let mut movie = Self {
            frame: None,
            audio: None,
            finished: descriptor.is_none(),
            quit: false,
            stream: None,
            last_frame: 0,
            ticks: 0,
            previous: 0,
        };
        if let Some(descriptor) = descriptor {
            let mut stream = tenchu_assets::movie::Stream::open(disc, &descriptor.audio.path)?;
            movie.frame = Some(Arc::new(
                stream.next(disc)?.ok_or("Empty Japanese Tenchu 2 movie")?,
            ));
            movie.audio = Some(Arc::new(Clip::load(disc, &descriptor.audio)?));
            movie.stream = Some(stream);
            movie.last_frame = descriptor.last_frame;
        }
        Ok(movie)
    }

    pub fn step(&mut self, disc: &Disc<impl ReadAt>, buttons: u16) -> Result<()> {
        self.quit = buttons == 9;
        self.advance(disc, buttons)
    }

    pub fn step_title(&mut self, disc: &Disc<impl ReadAt>, buttons: u16) -> Result<()> {
        self.advance(disc, buttons)
    }

    fn advance(&mut self, disc: &Disc<impl ReadAt>, buttons: u16) -> Result<()> {
        self.finished |= self.quit || (self.previous == 0 && matches!(buttons, 8 | 0x2000));
        self.previous = buttons;
        self.ticks += 1;
        if !self.finished && self.ticks.is_multiple_of(2) {
            let frame = self
                .stream
                .as_mut()
                .unwrap()
                .next(disc)?
                .ok_or("Japanese Tenchu 2 movie ended before its final frame")?;
            self.finished = frame.number >= self.last_frame;
            self.frame = Some(Arc::new(frame));
        }
        Ok(())
    }
}

pub struct Prologue {
    pub resources: frontend::Prologue,
    pub state: tenchu_core::tenchu2::introduction::Prologue,
    pub audio: Arc<Clip>,
    vblanks: u8,
    skip: bool,
}

impl Prologue {
    pub fn load(disc: &Disc<impl ReadAt>) -> Result<Self> {
        let resources = frontend::Prologue::load(disc)?;
        let audio = Arc::new(Clip::load(disc, &resources.audio)?);
        Ok(Self {
            resources,
            state: Default::default(),
            audio,
            vblanks: 0,
            skip: false,
        })
    }

    pub fn step(&mut self, buttons: u16, skip: bool) {
        self.skip |= skip;
        self.vblanks += 2;
        if self.vblanks >= 4 {
            self.vblanks -= 4;
            let finished = u64::from(self.state.tick) * u64::from(self.audio.rate)
                >= self.audio.frames.len() as u64 * 15;
            let buttons = if std::mem::take(&mut self.skip) {
                8
            } else {
                buttons
            };
            self.state.step(buttons, finished);
        }
    }
}
