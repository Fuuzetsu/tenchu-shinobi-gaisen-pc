use super::raster::Canvas;
use crate::{source::DiscSource, web, web_assets::movie_file};
use std::{path::Path, sync::Arc, time::Duration};
use tenchu_assets::{
    Result,
    audio::stream::Clip,
    font::Font,
    frontend::{Movie, movie_commands},
    game::GameAssets,
    script::Language,
    texture::Vram,
};
use tenchu_core::frontend::Captions;

pub struct Player {
    video: web_sys::HtmlVideoElement,
    movie: Movie,
    captions: Captions,
    frame: u32,
    pixels: Vec<u8>,
    font: Font,
    vram: Vram,
}

#[derive(Default)]
pub struct Update {
    pub changed: bool,
    pub finished: bool,
    pub audio: Option<Arc<Clip>>,
    pub voices: Vec<i16>,
}

impl Player {
    pub fn new(
        _: &Path,
        assets: &GameAssets<DiscSource>,
        movie: Movie,
        language: Language,
    ) -> Result<Self> {
        let commands = movie
            .subtitles(language)
            .map(|p| assets.asset(&p).and_then(|b| movie_commands(&b)))
            .transpose()?
            .unwrap_or_default();
        let mut vram = Vram::default();
        let font = Font::load(assets, &mut vram)?;
        Ok(Self {
            video: web::movie_create(&movie_file(movie)).map_err(web::js_error)?,
            movie,
            captions: Captions::new(commands),
            frame: 0,
            pixels: vec![0; 320 * 240 * 4],
            font,
            vram,
        })
    }

    pub fn advance(&mut self, _: Duration) -> Result<Update> {
        let error = web::movie_error(&self.video);
        if !error.is_empty() {
            return Err(error);
        }
        let mut update = Update::default();
        if self.video.ready_state() < 2 {
            return Ok(update);
        }
        let frame = ((self.video.current_time() * 15.0).floor() as u32 + 1)
            .min(self.movie.last_frame() - 1);
        while self.frame < frame {
            self.frame += 1;
            update.voices.extend(self.captions.frame(self.frame + 1));
            update.changed = true;
        }
        if update.changed {
            self.pixels = js_sys::Uint8ClampedArray::new(
                &web::movie_pixels(&self.video).map_err(web::js_error)?,
            )
            .to_vec();
            if self.pixels.len() != 320 * 240 * 4 {
                return Err("Invalid decoded movie frame".into());
            }
        }
        update.finished = self.video.ended()
            || self.video.current_time() >= f64::from(self.movie.last_frame() - 1) / 15.0;
        Ok(update)
    }

    pub fn paint(&mut self, canvas: &mut Canvas) {
        canvas.pixels.copy_from_slice(&self.pixels);
        super::captions::paint(canvas, &self.captions.text, &mut self.font, &mut self.vram);
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        web::movie_close(&self.video);
    }
}
