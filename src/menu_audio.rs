use bevy::prelude::*;
use bevy_asset_loader::loading_state::LoadingStateAppExt as _;
use bevy_asset_loader::loading_state::config::ConfigureLoadingState as _;
use bevy_asset_loader::loading_state::config::LoadingStateConfig;
use bevy_seedling::prelude::SamplePlayer;

use crate::*;

pub struct MenuAudioPlugin;

impl Plugin for MenuAudioPlugin {
    fn build(&self, app: &mut App) {
        app
            // In case not added.
            .configure_loading_state(
                LoadingStateConfig::new(ProgramState::Initializing)
                    .load_collection::<CommonFxAssets>()
            )
            .configure_loading_state(
                LoadingStateConfig::new(ProgramState::LoadingSave)
                    .load_collection::<CommonFxAssets>()
            )
            .add_systems(Update,
                (
                    spawn_menu_fx,
                    handle_menu_actions,
                )
            )
        ;
    }
}


fn spawn_menu_fx(mut commands: Commands,
    fx: Option<Res<CommonFxAssets>>,
    mut reader: MessageReader<MenuActionMessage>,
) {
    if reader.is_empty() {
        return
    }
    let Some(fx) = fx else { return };

    let any = reader.read().any(is_menu_action_click_bait);

    if any {
        commands.spawn((
            UiSfx,
            SamplePlayer::new(fx.action.clone()),
        ));
    }
}

fn handle_menu_actions(mut commands: Commands,
    fx: Option<Res<CommonFxAssets>>,
    mut reader: MessageReader<MenuActionMessage>,
) {
    if reader.is_empty() {
        return
    }
    let Some(fx) = fx else { return };

    // See if a menu action happened and play a click
    let any = reader.read().any(is_menu_action_click_bait);

    if any {
        commands.spawn((
            UiSfx,
            SamplePlayer::new(fx.action.clone()),
        ));
    }
}

/// Play a click sound on menu action?
fn is_menu_action_click_bait(event: &MenuActionMessage) -> bool {
    match event {
        MenuActionMessage::Activate(_) => false,
        MenuActionMessage::Navigate(_) |
        // MenuActionMessage::Activate(_) |
        MenuActionMessage::Next(_) |
        MenuActionMessage::Reset(_) | MenuActionMessage::Previous(_) => true,
        MenuActionMessage::Slide(..) => false,
    }
}
