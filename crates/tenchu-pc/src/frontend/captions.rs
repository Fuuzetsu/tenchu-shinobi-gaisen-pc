use super::raster::{Canvas, Sprite};
use tenchu_assets::{
    ReadAt, Result,
    font::Font,
    frontend::{Image, Movie},
    game::GameAssets,
    script::Language,
    texture::Vram,
};
use tenchu_core::frontend::Captions;

pub struct Playback {
    original: Captions,
    alternate: Option<Captions>,
    language: Language,
}

impl Playback {
    pub fn load(
        assets: &GameAssets<impl ReadAt>,
        movie: Movie,
        language: Language,
    ) -> Result<Self> {
        Ok(Self {
            original: Self::track(assets, movie, language)?,
            alternate: None,
            language,
        })
    }

    fn track(
        assets: &GameAssets<impl ReadAt>,
        movie: Movie,
        language: Language,
    ) -> Result<Captions> {
        Ok(Captions::new(
            movie
                .subtitles(language)
                .map(|path| {
                    assets
                        .asset(&path)
                        .and_then(|bytes| tenchu_assets::frontend::movie_commands(&bytes))
                })
                .transpose()?
                .unwrap_or_default(),
        ))
    }

    pub fn select(
        &mut self,
        assets: &GameAssets<impl ReadAt>,
        movie: Movie,
        audio: Language,
        subtitles: Option<Language>,
        frame: u32,
    ) -> Result<bool> {
        let language = subtitles.unwrap_or(audio);
        if language == self.language {
            return Ok(false);
        }
        let alternate = if language == audio {
            None
        } else {
            let mut captions = Self::track(assets, movie, language)?;
            for frame in 1..=frame {
                captions.frame(frame + 1);
            }
            Some(captions)
        };
        self.alternate = alternate;
        self.language = language;
        Ok(true)
    }

    pub fn frame(&mut self, frame: u32) -> Vec<i16> {
        if let Some(alternate) = &mut self.alternate {
            alternate.frame(frame);
        }
        self.original.frame(frame)
    }

    pub fn text(&self) -> &[u8] {
        &self.alternate.as_ref().unwrap_or(&self.original).text
    }
}

pub fn paint(canvas: &mut Canvas, text: &[u8], font: &mut Font, vram: &mut Vram) {
    if text.is_empty() {
        return;
    }
    if let Some(image) = font.prepare_caption(text, vram) {
        let image = Image::from_vram(vram, image);
        canvas.sprite(Sprite::new(&image, 0.0, 99.5));
    } else {
        let width: i32 = text.iter().map(|&c| font.width(c)).sum();
        let mut x = -width as f32 / 2.0;
        let mut y = 92.0;
        for &code in text {
            if code == b'\n' {
                x = -width as f32 / 2.0;
                y += 16.0;
                continue;
            }
            let (image, nudge) = font.glyph(code);
            let image = Image::from_vram(vram, image);
            canvas.sprite(Sprite::new(
                &image,
                x + image.size[0] as f32 / 2.0,
                y + f32::from(nudge) + 8.0,
            ));
            x += font.width(code) as f32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires TENCHU_DISC; all localized movie subtitle combinations preserve source narration timing"]
    fn independent_movie_subtitles_preserve_every_voice_cue() {
        let path = std::env::var_os("TENCHU_DISC").unwrap();
        let assets = GameAssets::open(crate::source::open_disc(path.as_ref()).unwrap()).unwrap();
        let languages = [
            Language::English,
            Language::French,
            Language::Italian,
            Language::Japanese,
        ];
        let mut combinations = 0;
        for movie in [
            Movie::Opening,
            Movie::Training,
            Movie::Rikimaru,
            Movie::Ayame,
            Movie::Credits,
        ] {
            for audio in languages {
                for subtitle in languages {
                    let mut original = Playback::load(&assets, movie, audio).unwrap();
                    let mut translated = Playback::load(&assets, movie, audio).unwrap();
                    let mut captions = Playback::load(&assets, movie, subtitle).unwrap();
                    translated
                        .select(&assets, movie, audio, Some(subtitle), 0)
                        .unwrap();
                    for frame in 1..movie.last_frame() {
                        assert_eq!(translated.frame(frame + 1), original.frame(frame + 1));
                        captions.frame(frame + 1);
                        assert_eq!(translated.text(), captions.text());
                        if frame == 200 {
                            // Switching at a held frame neither replays earlier voices nor loses the current caption.
                            translated
                                .select(&assets, movie, audio, None, frame)
                                .unwrap();
                            assert_eq!(translated.text(), original.text());
                            translated
                                .select(&assets, movie, audio, Some(subtitle), frame)
                                .unwrap();
                            assert_eq!(translated.text(), captions.text());
                        }
                    }
                    combinations += 1;
                }
            }
        }
        assert_eq!(combinations, 80);
    }
}
