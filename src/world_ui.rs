use bevy::camera::ScreenSpaceTransmissionQuality;
use bevy::light::CascadeShadowConfig;
use bevy::light::CascadeShadowConfigBuilder;
use bevy::light::ShadowFilteringMethod;
use bevy::prelude::*;

use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::pbr::ScreenSpaceAmbientOcclusion;
use bevy::pbr::ScreenSpaceAmbientOcclusionQualityLevel;

use crate::WorldCamera;

use super::video::Antialiasing;
use super::video::ShadowQuality;
use super::video::GlassQuality;
use super::video::VideoCameraSettingsChanged;
use super::video::VideoEffectSettingsChanged;
use super::video::VideoSettings;

pub struct WorldUiPlugin;

impl Plugin for WorldUiPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(PreUpdate,
                (
                    apply_effect_settings,
                    apply_camera_settings,
                )
                // .in_set(SimulationSystems)
            )
        ;
    }
}

pub fn apply_camera_settings(
    trigger: Option<Res<VideoCameraSettingsChanged>>,
    mut commands: Commands,
    mut camera_q: Query<&mut Projection, (With<Camera3d>, With<WorldCamera>)>,
    video_settings: Res<VideoSettings>,
) {
    if trigger.is_none() {
        return;
    }

    let Ok(mut proj) = camera_q.single_mut() else {
        return
    };

    if let Projection::Perspective(proj) = &mut *proj {
        proj.fov = video_settings.fov_degrees.to_radians();
    }

    commands.remove_resource::<VideoCameraSettingsChanged>();
}

pub fn apply_effect_settings(
    trigger: Option<Res<VideoEffectSettingsChanged>>,
    mut commands: Commands,
    mut camera_q: Query<(Entity, &mut Camera3d)>, // all cameras
    shadow_q: Query<&CascadeShadowConfig>,
    mut point_light_q: Query<(Entity, &mut PointLight)>,
    mut spot_light_q: Query<(Entity, &mut SpotLight)>,
    mut dir_light_q: Query<(Entity, &mut DirectionalLight)>,
    video_settings: Res<VideoSettings>,
) {
    if trigger.is_none() {
        return;
    }

    info!("Setting up effects");
    for (camera_ent, mut cam3d) in camera_q.iter_mut() {
        let mut ent_commands = commands.entity(camera_ent);

        ent_commands.remove::<Msaa>();
        ent_commands.remove::<ScreenSpaceAmbientOcclusion>();
        ent_commands.remove::<TemporalAntiAliasing>();

        match video_settings.antialiasing {
            Antialiasing::Off => {
                ent_commands.remove::<(ScreenSpaceAmbientOcclusion, TemporalAntiAliasing)>();

                ent_commands.insert((
                    Msaa::Off,
                ));
            },
            Antialiasing::TSAA => {
                ent_commands.insert((
                    Msaa::Off,
                    ScreenSpaceAmbientOcclusion {
                        quality_level: ScreenSpaceAmbientOcclusionQualityLevel::Medium,
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

        // Configure the camera's shadow settings. See below for lighting.
        {
            let mut new_config = CascadeShadowConfigBuilder::default();
            if let Ok(current_config) = shadow_q.get(camera_ent) {
                new_config.num_cascades = current_config.bounds.len();
            }
            // new_config.first_cascade_far_bound = 10.0;
            // new_config.maximum_distance = 150.0;

            match video_settings.shadow_quality {
                ShadowQuality::Off => {
                    ent_commands.remove::<CascadeShadowConfig>();
                }
                ShadowQuality::Low => {
                    new_config.num_cascades = 1;

                }
                ShadowQuality::Medium => {
                    // default
                    new_config.num_cascades = 3;
                    // new_config.maximum_distance = 25.0;
                }
                ShadowQuality::High => {
                    new_config.num_cascades = 6;
                    // new_config.maximum_distance = 35.0;
                }
                ShadowQuality::Ultra => {
                    new_config.num_cascades = 8;
                    // new_config.maximum_distance = 50.0;
                }

            }

            if new_config.num_cascades > 0 {
                // Copied from default()
                if cfg!(target_arch = "wasm32") && !cfg!(feature = "webgpu") {
                    new_config.num_cascades = 1;
                }

                ent_commands.insert(new_config.build());
            }
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

    // Update lights.
    for (ent, mut light) in point_light_q.iter_mut() {
        match video_settings.shadow_quality {
            ShadowQuality::Off => {
                light.soft_shadows_enabled = false;
            }
            ShadowQuality::Low => {
                light.soft_shadows_enabled = false;
                commands.entity(ent).insert(ShadowFilteringMethod::Hardware2x2);
            }
            ShadowQuality::Medium => {
                // default
                light.soft_shadows_enabled = true;
                commands.entity(ent).insert(ShadowFilteringMethod::Hardware2x2);
            }
            ShadowQuality::High => {
                light.soft_shadows_enabled = true;
                commands.entity(ent).insert(ShadowFilteringMethod::Gaussian);
            }
            ShadowQuality::Ultra => {
                light.soft_shadows_enabled = true;
                commands.entity(ent).insert(ShadowFilteringMethod::Temporal);
            }

        };
    }

    for (ent, mut light) in dir_light_q.iter_mut() {
        match video_settings.shadow_quality {
            ShadowQuality::Off => {
                light.shadows_enabled = false;
                light.soft_shadow_size = None;
            }
            ShadowQuality::Low => {
                light.shadows_enabled = true;
                light.soft_shadow_size = None;
                commands.entity(ent).insert(ShadowFilteringMethod::Hardware2x2);
            }
            ShadowQuality::Medium => {
                // default
                light.shadows_enabled = true;
                light.soft_shadow_size = Some(1.0);
                commands.entity(ent).insert(ShadowFilteringMethod::Gaussian);
            }
            ShadowQuality::High => {
                light.shadows_enabled = true;
                light.soft_shadow_size = Some(2.0);
                commands.entity(ent).insert(ShadowFilteringMethod::Gaussian);
            }
            ShadowQuality::Ultra => {
                light.shadows_enabled = true;
                light.soft_shadow_size = Some(3.0);
                commands.entity(ent).insert(ShadowFilteringMethod::Temporal);
            }

        };
    }

    // Done.

    commands.remove_resource::<VideoEffectSettingsChanged>();
}
