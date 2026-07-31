use bevy::prelude::*;
#[cfg(feature = "firewheel")]
use bevy_seedling::sample::SamplePlayer;
#[cfg(feature = "firewheel")]
use firewheel::nodes::volume::VolumeNode;
#[cfg(feature = "firewheel")]
use rand::RngExt as _;

#[cfg(feature = "firewheel")]
use crate::{assets::CommonFxAssets, audio::UiSfx};

pub struct FlashlightPlugin;

impl Plugin for FlashlightPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<FlashlightOffset>()
            .init_resource::<FlashlightRotation>()
            .add_systems(
                Update,
                update_flashlight,
            )
        ;
    }
}

#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
pub struct Flashlight {
    pub enabled: bool,

    pub spot: SpotLight,
}

impl Default for Flashlight {
    fn default() -> Self {
        Self {
            enabled: false,
            spot: SpotLight {
                intensity: 100_000.0,
                range: 50.0,
                color: Color::LinearRgba(Color::WHITE.to_linear() * 10.0f32),
                outer_angle: 0.75,
                inner_angle: 0.5,
                shadow_maps_enabled: true,
                #[cfg(feature = "experimental_pbr_pcss")]
                soft_shadows_enabled: true,
                .. default()
            }
        }
    }
}

#[derive(Resource, Reflect, Debug, Clone)]
#[reflect(Resource, Default)]
#[type_path = "game"]
pub struct FlashlightOffset(pub Vec3);

#[derive(Resource, Reflect, Debug, Clone)]
#[reflect(Resource, Default)]
#[type_path = "game"]
pub struct FlashlightRotation(pub Vec3);

impl Default for FlashlightOffset {
    fn default() -> Self {
        Self(Vec3::new(0.5, 0.5, 0.0))
    }
}

impl Default for FlashlightRotation {
    fn default() -> Self {
        Self(Vec3::new(-30.0, 15.0, 0.0))
    }
}

fn update_flashlight(
    mut commands: Commands,
    offs: Res<FlashlightOffset>,
    rot: Res<FlashlightRotation>,
    lights_q: Query<(Entity, &Flashlight)>,
    changed_lights_q: Query<(Entity, &Flashlight), Changed<Flashlight>>,
    spot_q: Query<&SpotLight>,
    fx: Option<Res<CommonFxAssets>>,
) {
    let offs_rot_changed = offs.is_changed() || rot.is_changed();
    for (ent, light) in lights_q.iter() {
        let changed = changed_lights_q.contains(ent);
        if !changed && !offs_rot_changed { continue };

        let mut ent_commands = commands.entity(ent);
        if light.enabled {
            ent_commands.try_insert(light.spot.clone());
            ent_commands.insert(Transform::from_translation(offs.0)
                .with_rotation(Quat::from_euler(
                    EulerRot::XYZ,
                    rot.0.x.to_radians(),
                    rot.0.y.to_radians(),
                    rot.0.z.to_radians()
                )
            ));

            if !spot_q.contains(ent) && cfg!(feature = "firewheel") && let Some(fx) = &fx {
                let mut rng = rand::rng();
                commands.spawn((
                    UiSfx,
                    SamplePlayer::new(
                        fx.click_on.clone(),
                    ),
                    PlaybackSettings {
                        speed: rng.random_range(0.9..1.1),
                        ..default()
                    },
                    VolumeNode::from_linear(rng.random_range(0.25..0.5)),
                ));
            }
        } else if spot_q.contains(ent) {
            ent_commands.try_remove::<SpotLight>();

            if cfg!(feature = "firewheel") && let Some(fx) = &fx {
                let mut rng = rand::rng();
                commands.spawn((
                    UiSfx,
                    SamplePlayer::new(
                        fx.click_off.clone(),
                    ),
                    PlaybackSettings {
                        speed: rng.random_range(0.9..1.1),
                        ..default()
                    },
                    VolumeNode::from_linear(rng.random_range(0.25..0.5)),
                ));
            }
        }
    }
}
