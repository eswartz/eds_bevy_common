use crate::*;
use bevy::input::ButtonState;
use bevy::input::gamepad::GamepadButtonChangedEvent;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;

pub const GAMEPAD_BUTTON_MENU: GamepadButton = GamepadButton::Start;

/// Handle Escape, which is handled differently outside and inside
/// a modal GUI context (in-game, independent of e.g. the egui BIE).
///
/// While in a menu, we intercept `KeyCode::Escape` and `GAMEPAD_BUTTON_MENU`
/// to
/// It should be used without filters.
pub(crate) fn handle_escape(
    mut commands: Commands,
    overlay_state: Res<State<OverlayState>>,
    program_state: Res<State<ProgramState>>,
    going_back: Option<Res<GoBackInMenuRequest>>,
    mut previous_menu: ResMut<PreviousMenuStack>,
    mut keyboard_reader: MessageReader<KeyboardInput>,
    mut gamepad_reader: MessageReader<GamepadButtonChangedEvent>,
    mut pause: ResMut<PauseState>,
) {
    if going_back.is_some() {
        return;
    }

    let mut menu_detected = false;

    for key_event in keyboard_reader.read() {
        if key_event.state == ButtonState::Pressed && key_event.key_code == KeyCode::Escape {
            menu_detected = true;
            break;
        }
    }

    for button_event in gamepad_reader.read() {
        if button_event.state == ButtonState::Pressed && button_event.button == GAMEPAD_BUTTON_MENU {
            menu_detected = true;
            break;
        }
    }

    if menu_detected {
        // If we reach the root, handle it here.
        match overlay_state.get() {
            OverlayState::Hidden => {
                if **program_state == ProgramState::InGame {
                    // The one case where Escape *opens* the menu the first time.
                    debug!("... Escape");
                    previous_menu.0.clear();
                    commands.set_state(OverlayState::EscapeMenu);
                } else {
                    // This is a hack to avoid freezing a game forever at Loading... (allows exiting at least)
                    commands.set_state(OverlayState::MainMenu);
                }
            }
            OverlayState::EscapeMenu => {
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
            _ => (),
        }
    }
}
