use bevy::light::NotShadowCaster;
use bevy::prelude::*;

use crate::ConfigureBeforePlaying;
use crate::LevelState;
use crate::ShadowCaster;

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

/// Make sure lights cast shadows if marked to do so.
pub(crate) fn fixup_light_shadows(
    mut commands: Commands,
    mut light_q: ParamSet<(
        Query<(Entity, &mut PointLight, Has<ShadowCaster>)>,
        Query<(Entity, &mut SpotLight, Has<ShadowCaster>)>,
        Query<(Entity, &mut DirectionalLight, Has<ShadowCaster>)>,
    )>,
) {
    for (ent, mut light, enabled) in light_q.p0().iter_mut() {
        light.shadows_enabled = enabled;

        let mut ent_commands = commands.entity(ent);
        // ent_commands.try_remove::<(ShadowCaster, ConfigureBeforePlaying)>();
        if !enabled {
            ent_commands.insert(NotShadowCaster);
        }
        ent_commands.try_remove::<ConfigureBeforePlaying>();
    }
    for (ent, mut light, enabled) in light_q.p1().iter_mut() {
        light.shadows_enabled = enabled;

        let mut ent_commands = commands.entity(ent);
        if !enabled {
            ent_commands.insert(NotShadowCaster);
        }
        ent_commands.try_remove::<ConfigureBeforePlaying>();
    }
    for (ent, mut light, enabled) in light_q.p2().iter_mut() {
        light.shadows_enabled = enabled;

        let mut ent_commands = commands.entity(ent);
        if !enabled {
            ent_commands.insert(NotShadowCaster);
        }
        ent_commands.try_remove::<ConfigureBeforePlaying>();
    }
}
