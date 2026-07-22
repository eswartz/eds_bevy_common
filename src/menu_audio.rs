use bevy::prelude::*;
use bevy_seedling::prelude::SamplePlayer;

use crate::*;

pub struct MenuAudioPlugin;

impl Plugin for MenuAudioPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(FixedUpdate,
                (
                    spawn_menu_fx,
                    handle_menu_actions,
                )
            )
        ;
    }
}

fn spawn_menu_fx(mut commands: Commands,
    fx: If<Res<CommonFxAssets>>,
    mut reader: MessageReader<MenuActionMessage>,
) {
    if reader.is_empty() {
        return
    }

    let any = reader.read().any(is_menu_action_click_bait);

    if any {
        commands.spawn((
            UiSfx,
            SamplePlayer::new(fx.action.clone()),
        ));
    }
}

fn handle_menu_actions(mut commands: Commands,
    fx: If<Res<CommonFxAssets>>,
    mut reader: MessageReader<MenuActionMessage>,
) {
    if reader.is_empty() {
        return
    }

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
        MenuActionMessage::Next(_) |
        MenuActionMessage::Reset(_) | MenuActionMessage::Previous(_) => true,
        MenuActionMessage::Slide(..) => false,
    }
}
