use std::time::Duration;

use avian3d::prelude::PhysicsLayer;
use bevy::prelude::*;

/// Mark the object for persistence.
#[derive(Default, Component, Reflect)]
#[reflect(Component)]
#[type_path = "game"]
pub struct Saveable;

/// Mark an entity as temporary.
#[derive(Component, Clone, Reflect)]
#[reflect(Component, Clone)]
#[type_path = "game"]
pub struct DespawnAfter(pub Duration);

/// Mark the entity as being out of play.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Ignored;


/// Mark where a player start point should go.
#[derive(Component, Reflect, Default)]
#[reflect(Component, Default)]
#[type_path = "game"]
pub struct PlayerStart;

/// This marks an entity playing background music / sound.
#[derive(Component, Clone, Reflect)]
// #[require(Saveable)]
#[reflect(Component, Clone)]
#[type_path = "game"]
pub struct BackgroundAudio;


#[derive(PhysicsLayer, Clone, Copy, Debug, Default)]
#[allow(unused)]
pub enum GameLayer {
    #[default]
    /// Layer 0 - the default layer that objects are assigned to
    Default,
    /// Layer 1 = player/camera
    Player,
    /// Layer 2 - static geometry
    World,
    /// Layer 3 - components with gameplay-specific physics
    Gameplay,
    /// Layer 4 - temporary bullets/projectiles/etc.
    Projectiles,
}


/// Used implicitly by all entities without a `RenderLayers` component.
/// Our world model camera and all objects other than the player are on this layer.
/// The light source belongs to both layers.
pub const RENDER_LAYER_DEFAULT: usize = 0;

/// Used by the view model camera and the player's arm.
/// The light source belongs to both layers.
pub const RENDER_LAYER_VIEW: usize = 1;

/// Shows UI overlays.
pub const RENDER_LAYER_UI: usize = 10;

/// Mark the world camera
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct WorldCamera;

/// Mark the viewer camera (e.g player)
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct ViewerCamera;
