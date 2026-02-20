/// Common assets.
///
use bevy::prelude::*;
use bevy_asset_loader::asset_collection::AssetCollection;

#[derive(Resource, AssetCollection)]
pub struct CommonAssets {
    /// This font provides common icons (pause/mute).
    #[asset(path = "fonts/emoji-icon-font.ttf")]
    pub emoji_icon_font: Handle<Font>,
    #[asset(path = "textures/crosshair.png")]
    pub crosshair: Handle<Image>,
}
