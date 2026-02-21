/// Generic reusable area markers.
use bevy::prelude::*;

#[derive(
    Default,
    Reflect,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    strum::Display,
    strum::FromRepr,
    strum::EnumIter,
    strum::EnumString,
    strum::IntoStaticStr,
    strum::VariantArray,
)]
#[reflect(Default)]
#[type_path = "game"]

#[cfg_attr(feature = "trenchbroom", derive(FgdType))]
#[cfg_attr(feature = "trenchbroom", number_key)]
pub enum AreaContent {
    /// Air (empty).
    #[default]
    Air = 0,
    /// Water.
    Water = 1,
    /// Ice.
    Ice = 2,
    /// Slime.
    Slime = 3,
    /// Lava.
    Lava = 4,
}

impl AreaContent {
    pub fn in_liquid(&self) -> bool {
        match self {
            AreaContent::Air => false,
            AreaContent::Water => true,
            AreaContent::Ice => false,
            AreaContent::Slime => true,
            AreaContent::Lava => true,
        }
    }
}
