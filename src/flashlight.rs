use bevy::prelude::*;

pub struct FlashlightPlugin;

impl Plugin for FlashlightPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(
                Update,
                enable_disable_flashlight,
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
                intensity: 500_000.0,
                range: 50.0,
                color: Color::LinearRgba(Color::WHITE.to_linear() * 10.0f32),
                outer_angle: 0.75,
                inner_angle: 0.5,
                shadows_enabled: true,
                .. default()
            }
        }
    }
}

fn enable_disable_flashlight(
    mut commands: Commands,
    lights_q: Query<(Entity, &Flashlight), Changed<Flashlight>>,
) {
    for (ent, light) in lights_q.iter() {
        let mut ent_commands = commands.entity(ent);
        if light.enabled {
            ent_commands.insert(light.spot.clone());
        } else {
            ent_commands.remove::<SpotLight>();
        }
    }
}
