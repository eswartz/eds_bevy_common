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

/// Make sure lights cast shadows.
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
        commands.entity(ent).try_remove::<(ShadowCaster, ConfigureBeforePlaying)>();
    }
    for (ent, mut light, enabled) in light_q.p1().iter_mut() {
        light.shadows_enabled = enabled;
        commands.entity(ent).try_remove::<(ShadowCaster, ConfigureBeforePlaying)>();
    }
    for (ent, mut light, enabled) in light_q.p2().iter_mut() {
        light.shadows_enabled = enabled;
        commands.entity(ent).try_remove::<(ShadowCaster, ConfigureBeforePlaying)>();
    }
}
