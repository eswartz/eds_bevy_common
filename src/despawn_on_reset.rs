//! This defines generic markers for use in most contexts.

use std::marker::PhantomData;
use bevy::ecs::entity_disabling::Disabled;
use bevy::prelude::*;
use bevy::state::state::StateTransitionSystems;

/// Add this plugin along with any user States with which you use [DespawnOnReset].
#[derive(Default)]
pub struct DespawnOnResetPlugin<S: States> {
    _marker: PhantomData<S>,
}

impl<S: States> Plugin for DespawnOnResetPlugin<S> {
    fn build(&self, app: &mut App) {
        app.add_systems(StateTransition,
            despawn_entities_on_state_set::<S>.in_set(StateTransitionSystems::EnterSchedules),
        );
    }
}

/// Despawns entities marked with [`DespawnOnReset<S>`] when their state no
/// longer matches the world state.
///
/// If the entity has already been despawned no warning will be emitted.
fn despawn_entities_on_state_set<S: States>(
    mut commands: Commands,
    mut transitions: MessageReader<StateTransitionEvent<S>>,
    query: Query<(Entity, &DespawnOnReset<S>), Allow<Disabled>>,
) {
    for transition in transitions.read() {
        if let Some(entered) = &transition.entered {
            // We're (again) at the entry state, so we have reset.
            for (entity, binding) in &query {
                if binding.0 == *entered {
                    commands.entity(entity).try_despawn();
                }
            }
        } else {
            // State is gone.
            for (entity, _binding) in &query {
                commands.entity(entity).try_despawn();
            }
        }
    }
}

/// Mark an entity to be culled when the given state is
/// either exited or set to the same value.
///
/// This works around a deficit in the combination
/// [DespawnOnExit] (and [DespawnOnEnter]), which
/// define "exit" and "enter" as transitions
/// from *changing*, so both see nothing to do.
#[derive(Component, Clone, Reflect, Debug)]
#[component(storage = "SparseSet")]
#[reflect(Component, Clone)]
#[type_path = "game"]
pub struct DespawnOnReset<S: States>(pub S);

impl<S> Default for DespawnOnReset<S>
where
    S: States + Default,
{
    fn default() -> Self {
        Self(S::default())
    }
}
