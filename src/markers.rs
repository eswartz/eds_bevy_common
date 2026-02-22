/// This defines generic markers for use in most contexts.

use std::time::Duration;
use bevy::prelude::*;
use crate::PlayerMode;

/// Mark the object for persistence.
#[derive(Default, Component, Reflect, Debug)]
#[reflect(Component)]
#[type_path = "game"]
pub struct Saveable;

/// Mark an entity as temporary.
#[derive(Component, Clone, Reflect, Debug)]
#[reflect(Component, Clone)]
#[type_path = "game"]
pub struct DespawnAfter(pub Duration);

/// Mark the entity as being out of play.
#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
#[type_path = "game"]
pub struct Ignored;

/// Mark the entity as being spawned during gameplay.
#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
#[type_path = "game"]
pub struct Spawned;

/// Mark the entity as a projectile (bullet / etc).
#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
#[type_path = "game"]
pub struct Projectile;

/// Mark where a player start point should go.
/// This should either be loaded in a .glb or manually.
#[derive(Component, Reflect, Default, Debug)]
#[reflect(Component, Default)]
#[type_path = "game"]
pub struct PlayerStart;

/// This marks an entity playing background music / sound.
#[derive(Component, Clone, Reflect, Debug)]
// #[require(Saveable)]
#[reflect(Component, Clone)]
#[type_path = "game"]
pub struct BackgroundAudio;

///

// Map markers (in .glb when using Bevy Skein).

/// Marker for the top level entity of a level (for searching metadata).
#[derive(Component, Reflect, Default, Debug)]
#[reflect(Component, Default)]
#[type_path = "game"]
pub struct LevelRoot;

/// Place on LevelRoot for the camera mode of the level.
#[derive(Component, Clone, Reflect, Debug)]
#[reflect(Component, Clone)]
#[type_path = "game"]
pub struct PlayerCameraMode(pub PlayerMode);

/////

/// Marker for deathbox (to catch falling player / items)
#[derive(Component, Reflect, Default, Debug)]
#[reflect(Component, Default)]
#[type_path = "game"]
pub struct DeathboxCollider;

/////
///
/// Mark the world camera (for 3D).
#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
#[type_path = "game"]
pub struct WorldCamera;

/// Mark the viewer camera (e.g player weapon).
#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
#[type_path = "game"]
pub struct ViewerCamera;
