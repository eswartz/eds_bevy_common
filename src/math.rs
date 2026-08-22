use std::{cmp::Ordering, hash::{Hash, Hasher}};

use bevy::prelude::*;

/// A float that is useful in data models.
/// Copied and extended from Bevy's FloatOrd, which just falls short (why no Default?!).
#[derive(Debug, Copy, Clone, Reflect, Default, Deref)]
#[reflect(Debug, PartialEq, Hash, Clone)]
pub struct Float32(pub f32);

impl PartialOrd for Float32 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }

    fn lt(&self, other: &Self) -> bool {
        !other.le(self)
    }
    // If `self` is NaN, it is equal to another NaN and less than all other floats, so return true.
    // If `self` isn't NaN and `other` is, the float comparison returns false, which match the `FloatOrd` ordering.
    // Otherwise, a standard float comparison happens.
    fn le(&self, other: &Self) -> bool {
        self.0.is_nan() || self.0 <= other.0
    }
    fn gt(&self, other: &Self) -> bool {
        !self.le(other)
    }
    fn ge(&self, other: &Self) -> bool {
        other.le(self)
    }
}

impl Ord for Float32 {
    #[expect(
        clippy::comparison_chain,
        reason = "This can't be rewritten with `match` and `cmp`, as this is `cmp` itself."
    )]
    fn cmp(&self, other: &Self) -> Ordering {
        if self > other {
            Ordering::Greater
        } else if self < other {
            Ordering::Less
        } else {
            Ordering::Equal
        }
    }
}

impl PartialEq for Float32 {
    fn eq(&self, other: &Self) -> bool {
        if self.0.is_nan() {
            other.0.is_nan()
        } else {
            self.0 == other.0
        }
    }
}

impl Eq for Float32 {}

impl Hash for Float32 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if self.0.is_nan() {
            // Ensure all NaN representations hash to the same value
            state.write(&f32::to_ne_bytes(f32::NAN));
        } else if self.0 == 0.0 {
            // Ensure both zeroes hash to the same value
            state.write(&f32::to_ne_bytes(0.0f32));
        } else {
            state.write(&f32::to_ne_bytes(self.0));
        }
    }
}

impl From<f32> for Float32 {
    fn from(value: f32) -> Self {
        Self(value)
    }
}
impl From<Float32> for f32 {
    fn from(value: Float32) -> f32 {
        value.0
    }
}
