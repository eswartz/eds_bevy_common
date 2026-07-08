use bevy::prelude::*;

/// Mark the material an object's surface, for use in sound effects.
///
/// NOTE: DON'T REORDER OR REMOVE ITEMS else it breaks Skein
/// (i.e. reloading a data model inside Blender *silently* changes enum discriminants!)
#[derive(Component, Reflect, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[reflect(Component)]
#[non_exhaustive]
#[type_path = "game"]
pub enum SurfaceMaterial {
    #[default]
    /// No defined material.
    Unknown,

    Water,
    Synthetic,  // e.g. plastic or epoxy or whatever
    Wood,
    Metal,
    Glass,
    Earth,
    Stone,
    Sand,
    Gravel,
    Fabric, // e.g. rug
}
