use bevy::prelude::*;

use crate::LevelState;
use crate::OverlayState;
use crate::PauseState;
use crate::ProgramState;

pub fn show_dev_tools() -> bool {
    if let Ok(val) = std::env::var("DEBUG") {
        return val == "1" || val == "on";
    }

    #[cfg(debug_assertions)]
    {
        true
    }
    #[cfg(not(debug_assertions))]
    {
        false
    }
}


/// Use as a condition to test whether any field in PauseState is set.
pub fn is_paused(paused: Res<PauseState>) -> bool {
    paused.is_paused()
}
/// Use as a condition to test whether the user pause state is set.
/// (In the outer game, this refers specifically to user input of [Action::TogglePause].)
pub fn is_user_paused(paused: Res<PauseState>) -> bool {
    paused.is_user_paused()
}
/// Use as a condition to test whether the menu pause state is set.
/// This refers specifically to internal menu-driven changes
/// (using in-game menu), not user inputs.
pub fn is_menu_paused(paused: Res<PauseState>) -> bool {
    paused.is_menu_paused()
}

pub fn is_game_active(program_state: Res<State<ProgramState>>) -> bool {
    *program_state.get() == ProgramState::InGame
}

pub fn is_in_menu(overlay: Res<State<OverlayState>>) -> bool {
    overlay.is_menu()
}

/// Set if the level is active (i.e. player can move around).
pub fn is_level_active(level_state: Res<State<LevelState>>) -> bool {
    matches!(*level_state.get(), LevelState::Playing | LevelState::Won | LevelState::Lost)
}
