// Original Actor.flags layout; collision slots and terrain materials use different bits.
pub const POISONED: u32 = 0x100;
pub const BURNING: u32 = 0x200;
pub const CONFUSED: u32 = 0x400;
pub const SLEEPING: u32 = 0x800;
pub const FALLING: u32 = 0x1000;
pub const IN_WATER: u32 = 0x2000;
pub const CEILING: u32 = 0x4000;
pub const BLOCKED: u32 = 0x8000;
pub const LANDED: u32 = 0x10000;
pub const UNEVEN: u32 = 0x20000;
pub const TOUCHING: u32 = 0x40000;
pub const HIT: u32 = 0x80000;
pub const TOUCHING_PLAYER: u32 = 0x400000;
