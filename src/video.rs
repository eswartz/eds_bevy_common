use bevy::core_pipeline::oit::OrderIndependentTransparencySettings;
use bevy::pbr::ScreenSpaceTransmission;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use strum_macros::Display;
use strum_macros::EnumIter;
use strum_macros::EnumString;
use strum_macros::FromRepr;
use strum_macros::VariantArray;

use bevy::anti_alias::taa::TemporalAntiAliasing;

use bevy::pbr::ScreenSpaceAmbientOcclusion;
use bevy::pbr::ScreenSpaceAmbientOcclusionQualityLevel;
use bevy::pbr::ScreenSpaceTransmissionQuality;

use crate::GameplayState;
use crate::LevelState;
use crate::WorldCamera;

pub struct VideoPlugin;

impl Plugin for VideoPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(GameplayState::Playing),
                (
                    apply_effect_settings,
                    apply_camera_settings,
                )
            )
            .add_systems(OnEnter(LevelState::Configuring),
                (
                    apply_effect_settings,
                    apply_camera_settings,
                )
            )
            .add_systems(PreUpdate,
                (
                    apply_effect_settings.run_if(resource_changed::<VideoSettings>),
                    apply_camera_settings.run_if(resource_changed::<VideoSettings>.or_else(resource_changed::<FovDelta>)),
                )
            )
        ;
    }
}

#[derive(Resource, Debug, Clone, Reflect)]
#[reflect(Default, Clone, Resource)]
#[type_path = "game"]
pub struct VideoSettings {
    pub fov_degrees: f32,
    pub antialiasing: Antialiasing,
    pub mesh_quality: MeshQuality,
    pub texture_quality: TextureQuality,
    pub shadow_quality: ShadowQuality,
    pub glass_quality: GlassQuality,
    pub oit_settings: OrderIndependentTransparencyQuality,
}

impl Default for VideoSettings {
    fn default() -> Self {
        Self {
            fov_degrees: 75.0,
            antialiasing: Default::default(),
            mesh_quality: Default::default(),
            texture_quality: Default::default(),
            shadow_quality: Default::default(),
            glass_quality: GlassQuality::Off,
            oit_settings: OrderIndependentTransparencyQuality::default(),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Reflect)]
#[reflect(Default, Clone)]
#[type_path = "game"]
pub enum OrderIndependentTransparencyQuality {
    Off,
    Low,
    #[default]
    Medium,
    High,
    Ultra,
}

impl OrderIndependentTransparencyQuality {
    pub fn to_settings(&self) -> Option<OrderIndependentTransparencySettings> {
        if cfg!(any(target_arch = "wasm32", feature = "solari")) {
            return None
        }

        Some(match self {
            OrderIndependentTransparencyQuality::Off => return None,
            OrderIndependentTransparencyQuality::Low =>
                OrderIndependentTransparencySettings {
                    sorted_fragment_max_count: 4,
                    fragments_per_pixel_average: 2.0,
                    alpha_threshold: 0.0,
                    ..default()
                },
            OrderIndependentTransparencyQuality::Medium =>
                OrderIndependentTransparencySettings {
                    sorted_fragment_max_count: 8,
                    fragments_per_pixel_average: 4.0,
                    alpha_threshold: 0.0,
                    ..default()
                },
            OrderIndependentTransparencyQuality::High =>
                OrderIndependentTransparencySettings {
                    sorted_fragment_max_count: 16,
                    fragments_per_pixel_average: 6.0,
                    alpha_threshold: 0.0,
                    ..default()
                },
            OrderIndependentTransparencyQuality::Ultra =>
                OrderIndependentTransparencySettings {
                    sorted_fragment_max_count: 24,
                    fragments_per_pixel_average: 8.0,
                    alpha_threshold: 0.0,
                    ..default()
                },
        })
    }
}

#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Reflect, Deref, DerefMut)]
#[reflect(Default, Clone, Resource)]
#[type_path = "game"]
pub struct FovDelta(pub f32);

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
    #[cfg_attr(any(target_arch = "wasm32", feature = "solari"), default)]
    Off,
    #[cfg_attr(all(not(target_arch = "wasm32"), not(feature = "solari")), default)]
    TSAA,
    MSAA,
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
pub enum ShadowQuality {
    #[cfg_attr(feature = "solari", default)]
    Off,
    Low,
    #[cfg_attr(not(feature = "solari"), default)]
    Medium,
    High,
    Ultra,
}

fn apply_camera_settings(
    mut camera_q: Query<&mut Projection, (With<Camera3d>, With<WorldCamera>)>,
    video_settings: Res<VideoSettings>,
    fov_delta: Res<FovDelta>,
) {
    let Ok(mut proj) = camera_q.single_mut() else {
        return
    };

    if let Projection::Perspective(proj) = &mut *proj {
        let fov_degrees = video_settings.fov_degrees + **fov_delta;
        proj.fov = fov_degrees.clamp(2.0, 150.0).to_radians();
    }
}

fn apply_effect_settings(
    mut commands: Commands,
    camera_q: Query<Entity, With<Camera3d>>, // all cameras
    video_settings: Res<VideoSettings>,
) {
    info!("Setting up effects {video_settings:?}");
    for camera_ent in camera_q.iter() {
        let mut ent_commands = commands.entity(camera_ent);

        ent_commands.try_remove::<Msaa>();
        ent_commands.try_remove::<ScreenSpaceAmbientOcclusion>();
        ent_commands.try_remove::<TemporalAntiAliasing>();

        match video_settings.antialiasing {
            Antialiasing::Off => {
                ent_commands.try_remove::<(
                    ScreenSpaceAmbientOcclusion,
                    TemporalAntiAliasing,
                )>();

                ent_commands.try_insert((
                    Msaa::Off,
                ));

                if let Some(settings) = video_settings.oit_settings.to_settings() {
                    ent_commands.try_insert(settings);
                }
            },
            Antialiasing::TSAA => {
                ent_commands.try_insert((
                    Msaa::Off,
                    ScreenSpaceAmbientOcclusion {
                        quality_level:
                            match video_settings.texture_quality {
                                TextureQuality::Low => ScreenSpaceAmbientOcclusionQualityLevel::Low,
                                TextureQuality::Medium => ScreenSpaceAmbientOcclusionQualityLevel::Medium,
                                TextureQuality::High => ScreenSpaceAmbientOcclusionQualityLevel::High,
                                TextureQuality::Ultra => ScreenSpaceAmbientOcclusionQualityLevel::Ultra,
                            },
                        ..default()
                    },
                    TemporalAntiAliasing::default(),
                ));
                if let Some(settings) = video_settings.oit_settings.to_settings() {
                    ent_commands.try_insert(settings);
                }
            }
            Antialiasing::MSAA => {
                ent_commands.try_remove::<(Msaa, ScreenSpaceAmbientOcclusion, TemporalAntiAliasing, OrderIndependentTransparencySettings)>();
                ent_commands.try_insert(Msaa::Sample4);
            }
        }

        match video_settings.glass_quality {
            GlassQuality::Off => {
                ent_commands.try_remove::<ScreenSpaceTransmission>();
            }
            GlassQuality::Low => {
                ent_commands.try_insert(ScreenSpaceTransmission {
                    steps: 1,
                    quality: ScreenSpaceTransmissionQuality::Low,
                });
            }
            GlassQuality::Medium => {
                ent_commands.try_insert(ScreenSpaceTransmission {
                    steps: 1,
                    quality: ScreenSpaceTransmissionQuality::Medium,
                });
            }
            GlassQuality::High => {
                ent_commands.try_insert(ScreenSpaceTransmission {
                    steps: 2,
                    quality: ScreenSpaceTransmissionQuality::High,
                });
            }
            GlassQuality::Ultra => {
                ent_commands.try_insert(ScreenSpaceTransmission {
                    steps: 3,
                    quality: ScreenSpaceTransmissionQuality::Ultra,
                });
            }
        }
    }

    // Lights and shadows handled in [lights::apply_light_effect_settings].
}
