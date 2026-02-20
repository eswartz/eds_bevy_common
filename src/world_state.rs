use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use avian3d::prelude::*;

use super::states_sets::GameplayState;
use super::states_sets::ProgramState;

pub struct WorldStatePlugin;

impl Plugin for WorldStatePlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(Gravity((9.8 * Vec3::NEG_Y).into()))
            .add_systems(OnEnter(GameplayState::AssetsLoaded),
                (
                    transition_from_loading,
                    setup_world_marker,
                )
                // .in_set(SimulationSystems)
                .run_if(in_state(ProgramState::InGame))
            )
            .add_systems(OnTransition{ exited: ProgramState::InGame, entered: ProgramState::LaunchMenu },
                (
                    despawn_world,
                )
                .chain()
            )
        ;
    }
}

#[derive(Component, Default, Reflect, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[reflect(Default)]
#[type_path = "game"]
// #[number_key]
pub enum AreaContent {
    /// Air.
    #[default]
    Air = 0,
    /// Water.
    Water = 1,
}

impl AreaContent {
    pub fn in_liquid(&self) -> bool {
        match self {
            AreaContent::Air => false,
            AreaContent::Water => true,
        }
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

/// The AABB reflects the full extent of the "valid content" of the world.
#[derive(Resource, Reflect)]
#[reflect(Resource)]
#[type_path = "game"]
pub struct WorldMarkerEntity(pub Entity);

fn transition_from_loading(
    mut commands: Commands,
) {
    commands.set_state(GameplayState::Setup);
}

/// This marker is created once and marks where game level content is swapped out.
pub fn setup_world_marker(
    mut commands: Commands,
    world_q: Query<&WorldMarker>,
) {
    if world_q.is_empty() {
        let ent = commands.spawn((
            Name::new("World"),
            DespawnOnExit(ProgramState::InGame),
            WorldMarker::default(),
            Transform::IDENTITY,
            Visibility::Inherited,
        )).id();
        commands.insert_resource(WorldMarkerEntity(ent));
    }
}

pub fn despawn_world(
    world: Single<Entity, With<WorldMarker>>,
    child_q: Query<&Children>,
    mut commands: Commands,
) {
    for kid in child_q.iter_descendants(*world) {
        commands.entity(kid).try_despawn();
    }
}
