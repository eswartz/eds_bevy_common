
use bevy::prelude::*;

/// Name of the product, as seen in the main menu and window title.
#[derive(Resource, Reflect)]
#[reflect(Resource)]
#[type_path = "game"]
pub struct ProductName(pub String);
