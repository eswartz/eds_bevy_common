use crate::*;
use bevy::input::ButtonState;
use bevy::input::gamepad::GamepadButtonChangedEvent;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;

pub const GAMEPAD_BUTTON_MENU: GamepadButton = GamepadButton::Start;

/// Handle Escape, which is handled differently outside and inside menus.
///
pub(crate) fn handle_escape(
    mut commands: Commands,
    overlay_state: Res<State<OverlayState>>,
    going_back: Option<Res<GoBackInMenuRequest>>,
    mut previous_menu: ResMut<PreviousMenuStack>,
    mut keyboard_reader: MessageReader<KeyboardInput>,
    mut gamepad_reader: MessageReader<GamepadButtonChangedEvent>,
    mut pause: ResMut<PauseState>,
) {
    // // Menu logic handles this itself.
    // if overlay_state.is_menu() {
    //     return;
    // }
    if going_back.is_some() {
        return;
    }

    let mut toggle_menu = false;

    for key_event in keyboard_reader.read() {
        if key_event.state == ButtonState::Pressed && key_event.key_code == KeyCode::Escape {
            toggle_menu = true;
        }
    }

    for button_event in gamepad_reader.read() {
        if button_event.state == ButtonState::Pressed && button_event.button == GAMEPAD_BUTTON_MENU {
            toggle_menu = true;
        }
    }

    if toggle_menu {
        // If we reach the root, handle it here.
        match overlay_state.get() {
            OverlayState::Hidden => {
                // The one case where Escape *opens* the menu the first time.
                previous_menu.0.clear();
                commands.set_state(OverlayState::EscapeMenu);
            }
            OverlayState::EscapeMenu => {
                // commands.write_message(MenuActionMessage::ResumeGame);   // nope

                // Go back to gameplay, like the ResumeGame command.
                pause.set_menu_paused(false);

                commands.set_state(OverlayState::Hidden);
            }
            OverlayState::MainMenu => {
                // Ignore, since we don't leave exit via Quit (TODO: can this quit?)
            }
            OverlayState::GameOverScreen => {
                commands.set_state(OverlayState::MainMenu);
            }
            OverlayState::DebugGuiVisible => commands.set_state(OverlayState::Hidden),
            _ => (),
        }
    }
}
