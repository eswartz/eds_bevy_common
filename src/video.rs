use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use strum_macros::Display;
use strum_macros::EnumIter;
use strum_macros::EnumString;
use strum_macros::FromRepr;
use strum_macros::VariantArray;

use bevy::camera::ScreenSpaceTransmissionQuality;

use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::pbr::ScreenSpaceAmbientOcclusion;
use bevy::pbr::ScreenSpaceAmbientOcclusionQualityLevel;

use crate::GameplayState;
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
            .add_systems(PreUpdate,
                (
                    apply_effect_settings.run_if(resource_changed::<VideoSettings>),
                    apply_camera_settings,
                )
            )
        ;
    }
}

#[derive(Resource, Clone, Copy, PartialEq, Reflect)]
#[reflect(Default, Clone, Resource)]
#[type_path = "game"]
pub struct VideoSettings {
    pub fov_degrees: f32,
    pub antialiasing: Antialiasing,
    pub mesh_quality: MeshQuality,
    pub texture_quality: TextureQuality,
    pub shadow_quality: ShadowQuality,
    pub glass_quality: GlassQuality,
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
        }
    }
}

#[derive(Resource, Default, Clone, Copy, PartialEq, Reflect, Deref, DerefMut)]
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
    if !video_settings.is_changed() && !fov_delta.is_changed() {
        return;
    }

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
    mut camera_q: Query<(Entity, &mut Camera3d)>, // all cameras
    video_settings: Res<VideoSettings>,
) {
    debug!("Setting up effects");
    for (camera_ent, mut cam3d) in camera_q.iter_mut() {
        let mut ent_commands = commands.entity(camera_ent);

        ent_commands.remove::<Msaa>();
        ent_commands.remove::<ScreenSpaceAmbientOcclusion>();
        ent_commands.remove::<TemporalAntiAliasing>();

        match video_settings.antialiasing {
            Antialiasing::Off => {
                ent_commands.remove::<(
                    ScreenSpaceAmbientOcclusion,
                    TemporalAntiAliasing,
                )>();

                ent_commands.insert((
                    Msaa::Off,
                ));
            },
            Antialiasing::TSAA => {
                ent_commands.insert((
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
            }
            // Antialiasing::MSAA => {
            //     ent_commands.remove::<(Msaa, ScreenSpaceAmbientOcclusion, TemporalAntiAliasing)>();
            //     // ent_commands.insert(Msaa::Sample4);
            // }
        }

        match video_settings.glass_quality {
            GlassQuality::Off => {
                cam3d.screen_space_specular_transmission_steps = 0;
                cam3d.screen_space_specular_transmission_quality = ScreenSpaceTransmissionQuality::Low;
            }
            GlassQuality::Low => {
                cam3d.screen_space_specular_transmission_steps = 1;
                cam3d.screen_space_specular_transmission_quality = ScreenSpaceTransmissionQuality::Low;
            }
            GlassQuality::Medium => {
                cam3d.screen_space_specular_transmission_steps = 1;
                cam3d.screen_space_specular_transmission_quality = ScreenSpaceTransmissionQuality::Medium;
            }
            GlassQuality::High => {
                cam3d.screen_space_specular_transmission_steps = 2;
                cam3d.screen_space_specular_transmission_quality = ScreenSpaceTransmissionQuality::High;
            }
            GlassQuality::Ultra => {
                cam3d.screen_space_specular_transmission_steps = 3;
                cam3d.screen_space_specular_transmission_quality = ScreenSpaceTransmissionQuality::Ultra;
            }
        }
    }

    // Lights and shadows handled in [lights::apply_light_effect_settings].
}
