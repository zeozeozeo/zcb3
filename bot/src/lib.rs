mod audio;
mod bot;
mod parser;

pub use audio::*;
pub use bot::*;
pub use fasteval2;
pub use parser::*;

use std::ops::RangeInclusive;

#[inline]
pub fn f32_range(range: RangeInclusive<f32>) -> f32 {
    fastrand::f32() * (range.end() - range.start()) + range.start()
}
