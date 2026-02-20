use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use strum_macros::Display;
use strum_macros::EnumIter;
use strum_macros::EnumString;
use strum_macros::FromRepr;
use strum_macros::VariantArray;

#[derive(Resource, Clone, Copy, PartialEq, Reflect)]
#[reflect(Default, Clone, Resource)]
#[type_path = "game"]
pub struct VideoSettings {
    pub fov_degrees: f32,
    pub antialiasing: Antialiasing,
    pub mesh_quality: MeshQuality,
    pub texture_quality: TextureQuality,
    pub glass_quality: GlassQuality,
}

impl Default for VideoSettings {
    fn default() -> Self {
        Self {
            fov_degrees: 75.0,
            antialiasing: Default::default(),
            mesh_quality: Default::default(),
            texture_quality: Default::default(),
            glass_quality: GlassQuality::Off,
        }
    }
}

/// When present, apply camera settings.
#[derive(Resource, Default)]
pub struct VideoCameraSettingsChanged;

/// When present, apply effects settings.
#[derive(Resource, Default)]
pub struct VideoEffectSettingsChanged;

#[derive(
    Component,
    Reflect,
    EnumIter,
    EnumString,
    VariantArray,
    Display,
    FromRepr,
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
)]
#[reflect(Default, Clone, Component)]
#[type_path = "game"]
pub enum Antialiasing {
    #[cfg_attr(target_arch = "wasm32", default)]
    Off,
    #[cfg_attr(not(target_arch = "wasm32"), default)]
    TSAA,
    // MSAA,    // can't use with OrderIndependentTransparency, so don't even offer it
}

#[derive(
    Component,
    Reflect,
    EnumIter,
    EnumString,
    VariantArray,
    Display,
    FromRepr,
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
)]
#[reflect(Default, Clone, Component)]
#[type_path = "game"]
pub enum MeshQuality {
    Low,
    #[default]
    Medium,
    High,
    Ultra,
}

#[derive(
    Component,
    Reflect,
    EnumIter,
    EnumString,
    VariantArray,
    Display,
    FromRepr,
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
)]
#[reflect(Default, Clone, Component)]
#[type_path = "game"]
pub enum TextureQuality {
    Low,
    #[default]
    Medium,
    High,
    Ultra,
}

#[derive(
    Component,
    Reflect,
    EnumIter,
    EnumString,
    VariantArray,
    Display,
    FromRepr,
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
)]
#[reflect(Default, Clone, Component)]
#[type_path = "game"]
pub enum GlassQuality {
    Off,
    Low,
    #[default]
    Medium,
    High,
    Ultra,
}
