use bevy::light::CascadeShadowConfig;
use bevy::light::CascadeShadowConfigBuilder;
use bevy::light::ShadowFilteringMethod;
use bevy::prelude::*;

use crate::Antialiasing;
use crate::ConfigureBeforePlaying;
use crate::GameplayState;
use crate::LevelState;
use crate::ShadowQuality;
use crate::VideoSettings;

pub struct LightsPlugin;

impl Plugin for LightsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(
                OnEnter(LevelState::Configuring),
                fixup_light_shadows
            )
            .add_systems(OnEnter(GameplayState::Playing),
                (
                    apply_light_effect_settings,
                )
            )
            .add_systems(PreUpdate,
                (
                    apply_light_effect_settings.run_if(resource_changed::<VideoSettings>),
                )
            )
        ;
    }
}

/// Mark the light as casting shadows.
///
/// (It's needed apparently since Blender glTF doesn't seem to export this
/// interesting property of lights.)
#[derive(Default, Component, Reflect, Debug)]
#[require(ConfigureBeforePlaying)]
#[reflect(Component)]
#[type_path = "game"]
pub struct ShadowCaster;

/// Make sure lights cast shadows if marked to do so.
pub(crate) fn fixup_light_shadows(
    mut commands: Commands,
    mut light_q: ParamSet<(
        Query<(Entity, &mut PointLight, Has<ShadowCaster>)>,
        Query<(Entity, &mut SpotLight, Has<ShadowCaster>)>,
        Query<(Entity, &mut DirectionalLight, Has<ShadowCaster>)>,
    )>,
) {
    let common_handling = |mut commands: Commands, ent, _enabled: bool| {
        let mut ent_commands = commands.entity(ent);
        ent_commands.try_remove::<ConfigureBeforePlaying>();
    };
    for (ent, mut light, enabled) in light_q.p0().iter_mut() {
        light.shadows_enabled = enabled;

        common_handling(commands.reborrow(), ent, enabled);
    }
    for (ent, mut light, enabled) in light_q.p1().iter_mut() {
        light.shadows_enabled = enabled;

        common_handling(commands.reborrow(), ent, enabled);
    }
    for (ent, mut light, enabled) in light_q.p2().iter_mut() {
        light.shadows_enabled = enabled;

        common_handling(commands.reborrow(), ent, enabled);
    }
}

fn apply_light_effect_settings(
    mut commands: Commands,
    camera_q: Query<Entity, With<Camera3d>>, // all cameras
    video_settings: Res<VideoSettings>,
    shadow_q: Query<&CascadeShadowConfig>,
    mut point_light_q: Query<(Entity, &mut PointLight)>,
    mut spot_light_q: Query<(Entity, &mut SpotLight)>,
    mut dir_light_q: Query<(Entity, &mut DirectionalLight)>,
) {
    for camera_ent in camera_q.iter() {
        let mut ent_commands = commands.entity(camera_ent);

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
    }

    // Update lights.
    let default_method = if video_settings.antialiasing == Antialiasing::TSAA {
        ShadowFilteringMethod::Temporal
    } else {
        ShadowFilteringMethod::Gaussian
    };

    for (ent, mut light) in point_light_q.iter_mut() {
        match video_settings.shadow_quality {
            ShadowQuality::Off => {
                light.shadows_enabled = false;
                light.soft_shadows_enabled = false;
                commands.entity(ent).remove::<ShadowFilteringMethod>();
            }
            ShadowQuality::Low => {
                light.shadows_enabled = true;
                light.soft_shadows_enabled = false;
                commands.entity(ent).insert(ShadowFilteringMethod::Hardware2x2);
            }
            ShadowQuality::Medium => {
                // default
                light.shadows_enabled = true;
                light.soft_shadows_enabled = false;
                commands.entity(ent).insert(default_method);
            }
            ShadowQuality::High => {
                light.shadows_enabled = true;
                light.soft_shadows_enabled = true;
                commands.entity(ent).insert(default_method);
            }
            ShadowQuality::Ultra => {
                light.shadows_enabled = true;
                light.soft_shadows_enabled = true;
                commands.entity(ent).insert(default_method);
            }

        };
    }

    for (ent, mut light) in spot_light_q.iter_mut() {
        match video_settings.shadow_quality {
            ShadowQuality::Off => {
                light.soft_shadows_enabled = false;
                commands.entity(ent).remove::<ShadowFilteringMethod>();
            }
            ShadowQuality::Low => {
                light.soft_shadows_enabled = false;
                commands.entity(ent).insert(ShadowFilteringMethod::Hardware2x2);
            }
            ShadowQuality::Medium => {
                // default
                light.soft_shadows_enabled = false;
                commands.entity(ent).insert(default_method);
            }
            ShadowQuality::High => {
                light.soft_shadows_enabled = true;
                commands.entity(ent).insert(default_method);
            }
            ShadowQuality::Ultra => {
                light.soft_shadows_enabled = true;
                commands.entity(ent).insert(default_method);
            }

        };
    }

    for (ent, mut light) in dir_light_q.iter_mut() {
        match video_settings.shadow_quality {
            ShadowQuality::Off => {
                light.shadows_enabled = false;
                light.soft_shadow_size = None;
                commands.entity(ent).remove::<ShadowFilteringMethod>();
            }
            ShadowQuality::Low => {
                light.shadows_enabled = true;
                light.soft_shadow_size = None;
                commands.entity(ent).insert(ShadowFilteringMethod::Hardware2x2);
            }
            ShadowQuality::Medium => {
                // default
                light.shadows_enabled = true;
                light.soft_shadow_size = None;
                commands.entity(ent).insert(default_method);
            }
            ShadowQuality::High => {
                light.shadows_enabled = true;
                // light.soft_shadow_size = Some(0.02);
                commands.entity(ent).insert(default_method);
            }
            ShadowQuality::Ultra => {
                light.shadows_enabled = true;
                // light.soft_shadow_size = Some(0.02);
                commands.entity(ent).insert(default_method);
            }

        };
    }
}
