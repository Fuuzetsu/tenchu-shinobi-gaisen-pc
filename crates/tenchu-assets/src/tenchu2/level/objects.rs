use crate::{Result, i16_le, short3, slice, u16_le};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Variant {
    #[default]
    Base,
    Second,
    Event,
    SecondEvent,
}

impl Variant {
    pub fn japanese(map: usize, sequence: usize, hidden: bool) -> Self {
        if hidden {
            return match map {
                0 if matches!(sequence, 17 | 117) => Self::SecondEvent,
                0 | 2 | 4 | 8 | 11 | 13 | 15 | 17 | 18 => Self::Event,
                5 if sequence == 120 => Self::Event,
                _ => Self::Base,
            };
        }
        match map {
            23 => match sequence {
                92 | 94 | 96 => Self::Second,
                243 | 245 => Self::Event,
                _ => Self::Base,
            },
            8 => match sequence {
                28 | 30 => Self::Second,
                128 => Self::SecondEvent,
                _ => Self::Base,
            },
            _ if matches!(sequence, 47 | 58 | 64 | 72 | 74 | 79 | 144) => Self::Second,
            _ => Self::Base,
        }
    }

    pub(super) fn files(self) -> (&'static str, &'static str) {
        match self {
            Self::Base => ("map.osd", "mapOBJ.arc"),
            Self::Second => ("map2.osd", "mapOBJ2.arc"),
            Self::Event => ("map_e.osd", "mapOBJe.arc"),
            Self::SecondEvent => ("map_e2.osd", "mapOBJe2.arc"),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Bounds {
    pub offset: [i16; 3],
    pub top: i16,
    pub half_width: i16,
    pub half_depth: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum StaticKind {
    Model(usize),
    Effect(u16),
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Static {
    pub position: [i32; 3],
    pub rotation: [i16; 3],
    pub kind: StaticKind,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Dynamic {
    pub position: [i32; 3],
    pub facing: i16,
    pub kind: u16,
    pub model: Option<usize>,
    pub trigger: Option<usize>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Trigger {
    pub position: [i32; 3],
    pub angle: i16,
    pub kind: i16,
    pub target: u16,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Objects {
    pub variant: Variant,
    pub bounds: Vec<Bounds>,
    pub statics: Vec<Static>,
    pub dynamic: Vec<Dynamic>,
    pub triggers: Vec<Trigger>,
}

impl Objects {
    pub(super) fn parse(data: &[u8], variant: Variant, models: usize) -> Result<Self> {
        let counts = [0, 2, 4, 6].map(|at| u16_le(data, at).map(usize::from));
        let [statics, dynamic, triggers, bounds] = counts;
        let (statics, dynamic, triggers, bounds) = (statics?, dynamic?, triggers?, bounds?);
        if bounds != models {
            return Err("Tenchu 2 object table mismatch".into());
        }
        let mut rows =
            slice(data, 12, (statics + dynamic + triggers + bounds) * 12)?.chunks_exact(12);
        let mut out = Self {
            variant,
            bounds: Vec::with_capacity(bounds),
            statics: Vec::with_capacity(statics),
            dynamic: Vec::with_capacity(dynamic),
            triggers: Vec::with_capacity(triggers),
        };
        for row in rows.by_ref().take(bounds) {
            out.bounds.push(Bounds {
                offset: short3(row, 0)?,
                top: i16_le(row, 6)?,
                half_width: i16_le(row, 8)?,
                half_depth: i16_le(row, 10)?,
            });
        }
        let position = |row| short3(row, 0).map(|p| p.map(|n| i32::from(n) * 10));
        for row in rows.by_ref().take(statics) {
            let id = u16_le(row, 10)?;
            let kind = if u16_le(row, 8)? == 0 {
                StaticKind::Effect(id)
            } else {
                if usize::from(id) >= models {
                    return Err("Tenchu 2 placement references a missing object".into());
                }
                StaticKind::Model(usize::from(id))
            };
            out.statics.push(Static {
                position: position(row)?,
                rotation: [i16::from(row[7]) * 16, i16::from(row[6]) * 16, 0],
                kind,
            });
        }
        for row in rows.by_ref().take(dynamic) {
            let kind = u16_le(row, 8)?;
            let value = u16_le(row, 10)?;
            let linked = kind < 4 || kind == 5;
            let model = usize::from(if linked { value & 255 } else { value });
            let trigger = linked.then_some(usize::from(value >> 8));
            if model >= models || trigger.is_some_and(|index| index >= triggers) {
                return Err("Invalid Tenchu 2 interactive object reference".into());
            }
            out.dynamic.push(Dynamic {
                position: position(row)?,
                facing: i16::from(row[6]) * 16,
                kind,
                model: Some(model),
                trigger,
            });
        }
        for row in rows {
            out.triggers.push(Trigger {
                position: position(row)?,
                angle: i16_le(row, 6)?,
                kind: i16_le(row, 8)?,
                target: u16_le(row, 10)?,
            });
        }
        Ok(out)
    }

    pub fn static_model_count(&self) -> usize {
        self.statics
            .iter()
            .filter(|p| matches!(p.kind, StaticKind::Model(_)))
            .count()
    }

    pub fn unrendered_count(&self) -> usize {
        self.statics.len() - self.static_model_count() + self.dynamic.len() + self.triggers.len()
    }
}
