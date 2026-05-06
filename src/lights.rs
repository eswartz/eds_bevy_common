use bevy::prelude::*;

use crate::ConfigureBeforePlaying;
use crate::LevelState;

pub struct LightsPlugin;

impl Plugin for LightsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(
                OnEnter(LevelState::Configuring),
                fixup_light_shadows
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
    let common_handling = |mut commands: Commands, ent, enabled: bool| {
        let mut ent_commands = commands.entity(ent);
        // ent_commands.try_remove::<(ShadowCaster, ConfigureBeforePlaying)>();
        if !enabled {
            ent_commands.insert(bevy::light::NotShadowCaster);
        }
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
