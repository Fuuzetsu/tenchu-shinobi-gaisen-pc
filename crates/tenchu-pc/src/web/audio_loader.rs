use crate::{web, web_assets::track_file};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    path::Path,
    rc::Rc,
    sync::Arc,
};
use tenchu_assets::{
    Result,
    audio::stream::{Clip, Track},
};
use wasm_bindgen::JsValue;

type Loaded = Result<(Arc<Clip>, bool)>;

type Clips = Rc<RefCell<VecDeque<(String, Arc<Clip>)>>>;

pub struct Loader {
    generation: Rc<Cell<u64>>,
    result: Rc<RefCell<Option<Loaded>>>,
    cache: Clips,
}

impl Loader {
    pub fn new(_: &Path) -> Result<Self> {
        Ok(Self {
            generation: Rc::new(Cell::new(0)),
            result: Rc::default(),
            cache: Rc::default(),
        })
    }

    pub fn request(&mut self, track: Option<Track>) {
        let serial = self.generation.get().wrapping_add(1);
        self.generation.set(serial);
        self.result.borrow_mut().take();
        let Some(track) = track else {
            return;
        };
        let path = track_file(&track);
        let cached = self.cache.borrow().iter().position(|(key, _)| key == &path);
        if let Some(index) = cached {
            let entry = self.cache.borrow_mut().remove(index).unwrap();
            *self.result.borrow_mut() = Some(Ok((entry.1.clone(), track.repeat)));
            self.cache.borrow_mut().push_back(entry);
            return;
        }
        let generation = self.generation.clone();
        let result = self.result.clone();
        let cache = self.cache.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let loaded = async {
                let audio = web::audio_clip(&path).await.map_err(web::js_error)?;
                let field = |name| {
                    js_sys::Reflect::get(&audio, &JsValue::from_str(name)).map_err(web::js_error)
                };
                let rate = field("rate")?
                    .as_f64()
                    .ok_or("Missing decoded sample rate")? as u32;
                let left = js_sys::Float32Array::new(&field("left")?).to_vec();
                let right = js_sys::Float32Array::new(&field("right")?).to_vec();
                if rate == 0 || left.len() != right.len() {
                    return Err("Invalid decoded audio".into());
                }
                let sample = |n: f32| (n * 32768.0).round().clamp(-32768.0, 32767.0) as i16;
                Ok(Arc::new(Clip {
                    rate,
                    frames: left
                        .into_iter()
                        .zip(right)
                        .map(|(l, r)| [sample(l), sample(r)])
                        .collect(),
                }))
            }
            .await;
            if generation.get() != serial {
                return;
            }
            if let Ok(clip) = &loaded {
                let mut cache = cache.borrow_mut();
                while !cache.is_empty()
                    && cache.iter().map(|(_, c)| c.frames.len() * 4).sum::<usize>()
                        + clip.frames.len() * 4
                        > 64 * 1024 * 1024
                {
                    cache.pop_front();
                }
                cache.push_back((path, clip.clone()));
            }
            *result.borrow_mut() = Some(loaded.map(|clip| (clip, track.repeat)));
        });
    }

    pub fn poll(&self) -> Option<Loaded> {
        self.result.borrow_mut().take()
    }
}

impl Drop for Loader {
    fn drop(&mut self) {
        self.generation.set(self.generation.get().wrapping_add(1));
    }
}
