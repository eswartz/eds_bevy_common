/// Generic reusable area markers.
use bevy::prelude::*;

/// The content volume of a given area.
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
    /// Slime.
    Slime = 2,
    /// Lava.
    Lava = 3,
}

impl AreaContent {
    pub fn in_liquid(&self) -> bool {
        match self {
            AreaContent::Air => false,
            AreaContent::Water => true,
            AreaContent::Slime => true,
            AreaContent::Lava => true,
        }
    }
}

/// Marks water.
#[derive(Component, Reflect, Default, Clone)]
#[reflect(Component, Clone, Default)]
#[type_path = "game"]
pub struct Water;

/// Report change in area.
#[derive(Message, Reflect, Clone)]
#[reflect(Message, Clone)]
#[type_path = "game"]
pub struct AreaChanged {
    pub entity: Entity,
    pub from_area: AreaContent,
    pub to_area: AreaContent,
}
