use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;

use super::states_sets::GameplayState;
use super::states_sets::ProgramState;
use crate::physics::*;

pub struct WorldStatePlugin;

impl Plugin for WorldStatePlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(Gravity(9.8 * Vector::NEG_Y))

            // Spawn the [WorldMarker] once per game.
            .add_systems(OnEnter(GameplayState::Setup),
                (
                    setup_world_marker,
                )
            )

            // Despawn the whole world [WorldMarker] on down,
            // on exiting a game. This is the last-ditch cleanup which
            // a game should handle instead in [GameplayState::]
            .add_systems(OnTransition{ exited: ProgramState::InGame, entered: ProgramState::LaunchMenu },
                (
                    despawn_world,
                )
            )
        ;
    }
}

/// Mark entities that are specific to the gameplay world.
/// This only needs to be placed on toplevel parent entities.
///
/// The AABB reflects the full extent of the "valid content" of the world.
#[derive(Component, Reflect)]
#[reflect(Component)]
#[type_path = "game"]
pub struct WorldMarker(pub Aabb3d);

impl Default for WorldMarker {
    fn default() -> Self {
        Self(Aabb3d::new(Vec3::ZERO, Vec3::ONE))
    }
}

/// The singleton entity for [WorldMarker], while defined.
#[derive(Resource, Reflect)]
#[reflect(Resource)]
#[type_path = "game"]
pub struct WorldMarkerEntity(pub Entity);

/// This marker is created once and marks where game level content is swapped out.
pub fn setup_world_marker(
    mut commands: Commands,
    world_q: Query<&WorldMarker>,
    world_marker: Option<Res<WorldMarkerEntity>>,
    child_q: Query<&Children>,
) {
    if let Some(ent) = &world_marker
    && let Ok(children) = child_q.get(ent.0) {
        for kid in children {
            commands.entity(*kid).despawn();
        }
    }
    if world_q.is_empty() {
        let ent = commands.spawn((
            Name::new("World"),
            //DespawnOnExit(ProgramState::InGame),
            WorldMarker::default(),
            Transform::IDENTITY,
            Visibility::Inherited,
        )).id();
        commands.insert_resource(WorldMarkerEntity(ent));
    }
}

pub fn despawn_world(
    world_q: Query<Entity, With<WorldMarker>>,
    child_q: Query<&Children>,
    mut commands: Commands,
) {
    for world in world_q.iter() {
        for kid in child_q.iter_descendants(world) {
            commands.entity(kid).try_despawn();
        }
    }
}
