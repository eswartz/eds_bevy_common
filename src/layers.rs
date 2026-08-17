/// Common render and physics layers.
use bevy::prelude::*;
use crate::physics::*;

#[derive(PhysicsLayer, Clone, Copy, Debug, Default)]
#[derive(Reflect)]
#[reflect(Default)]
#[type_path = "game"]
pub enum GameLayer {
    #[default]
    /// Layer 0 (1) - the default layer that objects are assigned to
    Default,
    /// Layer 1 (2) = player/camera
    /// Assumes [crate::Player] component.
    Player,
    /// Layer 2 (4) - static geometry
    World,
    /// Layer 3 (8) - components with gameplay-specific behavior
    /// independent of the player body.
    /// (This may be set on a child collider of [crate::Player]
    /// when interacting with e.g. tiles / buttons / etc.)
    Gameplay,
    /// Layer 4 (16) - temporary bullets/projectiles/etc.
    /// Assumes [crate::Projectile] component.
    Projectiles,
    /// Layer 5 (32) - enemy entities.
    Enemy,
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
