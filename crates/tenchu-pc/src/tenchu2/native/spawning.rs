use super::{
    Session,
    campaign::{Content, Playback},
};
use crate::visuals::appearance::library::{Library, actor_behavior};
use tenchu_assets::{
    ReadAt, Result,
    disc::Disc,
    tenchu2::{audio::voices_for, layout::Layout},
};
use tenchu_core::math::Position;

mod body;

pub enum Source {
    Native(i16),
    Model(String),
}

impl Source {
    pub fn selected(id: &str) -> Result<Self> {
        if let Some(kind) = id.strip_prefix("native:") {
            return kind
                .parse()
                .map(Self::Native)
                .map_err(|_| "Unknown native actor".into());
        }
        if id.is_empty() {
            return Err("Choose an actor first".into());
        }
        Ok(Self::Model(id.to_owned()))
    }
}

pub struct Request {
    pub source: Source,
    pub position: Position,
    pub yaw: i16,
    pub snap: bool,
}

impl<S: ReadAt> Playback<S> {
    pub fn spawn_actor(&mut self, request: Request, library: Option<&Library>) -> Result<usize> {
        let Content::Gameplay(session) = &mut self.content else {
            return Err("Start a mission before placing actors".into());
        };
        session.spawn_actor(&self.disc, request, library)
    }
}

impl Session {
    fn spawn_actor(
        &mut self,
        disc: &Disc<impl ReadAt>,
        request: Request,
        library: Option<&Library>,
    ) -> Result<usize> {
        if self.cinematic() {
            return Err("Wait for the cinematic before placing actors".into());
        }
        let (kind, body) = match &request.source {
            Source::Native(kind) => (*kind, None),
            Source::Model(id) => {
                let library = library.ok_or("Wait for the cross-game model gallery to load")?;
                let body = library.get(id).ok_or("This actor model is unavailable")?;
                (body::kind(library, body)?, Some(body))
            }
        };
        if kind < 0 {
            return Err("Choose a native T2 actor definition".into());
        }
        self.assets.load_actor(disc, kind)?;
        let definition =
            body.map_or(Ok(kind), |body| body::install(&mut self.assets, body, kind))?;
        let voice = if let Some(body) = body {
            let behavior = if body.native.is_some() {
                body
            } else {
                actor_behavior(library.unwrap(), body)?
            };
            Some(
                body.voice_set
                    .as_ref()
                    .or(behavior.voice_set.as_ref())
                    .cloned(),
            )
        } else if self.assets.cast.contains(&kind) {
            None
        } else {
            Some(voices_for(disc, &self.assets.resources.registry, &[kind])?.remove(&kind))
        };
        let behavior = Layout::available(disc)?
            .iter()
            .flat_map(|layout| &layout.actors)
            .find(|a| a.kind == kind && a.behavior >= 3)
            .map_or_else(
                || match kind & 0xf0 {
                    0x10 | 0x60 => 0x1513,
                    0x70 | 0x80 | 0x90 => 5,
                    _ => 0x2123,
                },
                |a| a.behavior,
            );
        let behavior = if behavior & 15 == 8 {
            (behavior & !15) | 3
        } else {
            behavior
        };
        let id = self.world.place_actor(
            tenchu_core::tenchu2::world::Placement {
                kind,
                definition,
                behavior,
                position: request.position,
                yaw: request.yaw,
                snap: request.snap,
            },
            &self.assets,
        )?;
        if let Some(voice) = voice {
            self.world.instances[id].voice = 4;
            self.actor_voices.insert(id, voice);
        }
        self.reset_interpolation()?;
        Ok(id)
    }
}

#[cfg(test)]
mod tests;
