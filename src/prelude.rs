//! The most common functionality re-exported.

pub use crate::{
    canvas::{Canvas, CanvasSpace},
    path::{LineCommand, Path, ToPaths},
    process::Process,
    units::{Inches, Millis, Paper},
    user_const, Config,
};
pub use euclid::{point2, vec2};
pub use fart_aabb::{Aabb, ToAabb};
pub use fart_utils::{clamp, map_range};
pub use lazy_static::lazy_static;
pub use noise::NoiseFn;
pub use rand::{
    distributions::{Distribution, Uniform},
    Rng,
};

pub use partial_min_max::*;
