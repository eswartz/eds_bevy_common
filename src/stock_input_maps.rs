
use crate::*;
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

/// These actions are useful for GUIs.
pub fn default_gui_input_map() -> InputMap<UserAction> {
    use actions_common::UserAction::*;

    let mut input_map = InputMap::default()
        .with_axis(
            MoveLeftRight2d,
            VirtualAxis::new(KeyCode::ArrowLeft, KeyCode::ArrowRight),
        )
        .with_axis(
            MoveDownUp2d,
            VirtualAxis::new(KeyCode::ArrowDown, KeyCode::ArrowUp),
        );

    // Note: this usage as an action is only processed in gameplay
    // (which, being pauseable, means there'd be no way to escape),
    // but KeyCode::Escape is elsewhere handled manually in an unpauseable way.
    input_map.insert(ToggleMenu, KeyCode::Escape);
    input_map.insert(TogglePause, KeyCode::Pause);
    input_map.insert(
        TogglePause,
        ButtonlikeChord::new([CTRL_COMMAND, KeyCode::KeyP]),
    ); // "P"ause
    input_map.insert(ToggleMute, KeyCode::F12);
    input_map.insert(
        ToggleMute,
        ButtonlikeChord::new([CTRL_COMMAND, KeyCode::KeyM]),
    ); // "M"ute
    input_map.insert(ToggleFullScreen, KeyCode::F11);

    input_map.insert(ToggleDebugUi, KeyCode::Backquote);

    input_map
}

/// This provides mappings for WASD + Space/C + mouse bindings for FPS or space controllers.
pub fn default_wasd_input_map() -> InputMap<UserAction> {
    use actions_common::UserAction::*;

    let mut input_map = InputMap::default()
        .with_dual_axis(MoveFlycam, VirtualDPad::wasd().inverted_y())
        .with_axis(MoveDownUp, VirtualAxis::new(KeyCode::KeyC, KeyCode::Space))
        .with_dual_axis(Look, MouseMove::default())
        .with_axis(
            Tilt,
            VirtualAxis::new(KeyCode::BracketRight, KeyCode::BracketLeft),
        );

    // Lazy finger movement falsely triggers these, which is very annoying.
    if cfg!(target_os = "macos") {
        const MOD: ModifierKey = ModifierKey::Alt;
        input_map.insert_axis(
            Zoom,
            VirtualAxis::new(
                ButtonlikeChord::modified(MOD, MouseScrollDirection::UP),
                ButtonlikeChord::modified(MOD, MouseScrollDirection::DOWN),
            ),
        );
        input_map.insert_axis(
            Tilt,
            VirtualAxis::new(
                ButtonlikeChord::modified(MOD, MouseScrollDirection::LEFT),
                ButtonlikeChord::modified(MOD, MouseScrollDirection::RIGHT),
            ),
        );
    } else {
        input_map.insert_axis(Zoom, MouseScrollAxis::Y);
        input_map.insert_axis(Tilt, MouseScrollAxis::X);
    }

    input_map.insert(Accelerate, ModifierKey::Shift);
    input_map.insert(ToggleCrouch, ModifierKey::Control);
    input_map.insert(Crouch, KeyCode::KeyC);
    input_map.insert(TurnAround, KeyCode::Backspace);

    // Home, i.e. to reset the camera to home position.
    input_map.insert(Home, KeyCode::Backslash);

    input_map.insert(Fire, MouseButton::Left);
    input_map.insert(
        ShiftFire,
        ButtonlikeChord::modified(ModifierKey::Shift, MouseButton::Left),
    );

    input_map.insert(AlternateFire, MouseButton::Right);
    input_map.insert(
        ShiftAlternateFire,
        ButtonlikeChord::modified(ModifierKey::Shift, MouseButton::Right),
    );

    input_map.insert(Interact, KeyCode::KeyE);

    input_map
}
