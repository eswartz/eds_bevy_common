use bevy::camera::ScreenSpaceTransmissionQuality;
use bevy::prelude::*;

use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::pbr::ScreenSpaceAmbientOcclusion;
use bevy::pbr::ScreenSpaceAmbientOcclusionQualityLevel;

use crate::WorldCamera;

use super::video::Antialiasing;
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

    commands.remove_resource::<VideoEffectSettingsChanged>();
}
