use bevy::input::ButtonState;
use bevy::input::gamepad::GamepadButtonChangedEvent;
use bevy::prelude::*;

use crate::prelude::*;

pub const GAMEPAD_BUTTON_MENU: GamepadButton = GamepadButton::Start;

/// Handle Escape, which is handled differently outside and inside
/// a modal GUI context (in-game, independent of e.g. the egui BIE).
///
/// While in a menu, we intercept `KeyCode::Escape` and `GAMEPAD_BUTTON_MENU`
/// to leave a menu.
///
/// This system should be used without filters.
pub(crate) fn handle_escape(
    mut commands: Commands,
    gui_prereq_opt: Option<Res<CommonGuiAssets>>,
    overlay_state: Res<State<OverlayState>>,
    program_state: Res<State<ProgramState>>,
    going_back: Option<Res<GoBackInMenuRequest>>,
    mut previous_menu: ResMut<PreviousMenuStack>,
    key_codes: Res<ButtonInput<KeyCode>>,
    mut gamepad_reader: MessageReader<GamepadButtonChangedEvent>,
) {
    if going_back.is_some() {
        // Escape/etc is being handled elsewhere.
        return;
    }

    let mut menu_detected = false;

    if key_codes.just_pressed(KeyCode::Escape) {
        menu_detected = true;
    }

    for button_event in gamepad_reader.read() {
        if button_event.state == ButtonState::Pressed && button_event.button == GAMEPAD_BUTTON_MENU
        {
            menu_detected = true;
            break;
        }
    }

    // If we reach the root, handle it here.

    // Quit program from Error menu.
    if menu_detected {
        if matches!(**overlay_state, OverlayState::ErrorScreen) {
            info!("... Bye!");
            commands.insert_resource(ExitRequest);
            return;
        } else if matches!(**overlay_state, OverlayState::GameOverScreen) {
            commands.set_state(ProgramState::LaunchMenu);
            commands.set_state(GameplayState::New);
            return;
        }
    }

    // If we could not load assets, don't try to show menu UI.
    if gui_prereq_opt.is_none() {
        return;
    }

    // When Escape *opens* the menu the first time
    if menu_detected
        && matches!(**overlay_state,
            OverlayState::Hidden | OverlayState::GameOverScreen | OverlayState::ErrorScreen)
        && **program_state == ProgramState::InGame
    {
        debug!("... Escape");
        previous_menu.0.clear();
        commands.set_state(OverlayState::EscapeMenu);
    }
}
