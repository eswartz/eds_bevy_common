use crate::*;
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

/// These actions are useful for GUIs.
pub fn default_gui_input_map() -> InputMap<UserAction> {
    use actions_common::UserAction::*;

    const UI_SENSITIVITY_X: f32 = 0.125;
    const UI_SENSITIVITY_Y: f32 = 0.125;

    let mut input_map = InputMap::default()
        .with_axis(
            MoveLeftRight2d,
            VirtualAxis::new(KeyCode::ArrowLeft, KeyCode::ArrowRight) //
                .sensitivity(UI_SENSITIVITY_X),
        )
        .with_axis(
            MoveLeftRight2d,
            VirtualAxis::new(GamepadButton::DPadLeft, GamepadButton::DPadRight) //
                .sensitivity(UI_SENSITIVITY_X),
        )
        .with_axis(
            MoveUpDown2d,
            VirtualAxis::new(KeyCode::ArrowUp, KeyCode::ArrowDown) //
                .sensitivity(UI_SENSITIVITY_Y),
        )
        .with_axis(
            MoveUpDown2d,
            VirtualAxis::new(GamepadButton::DPadUp, GamepadButton::DPadDown) //
                .sensitivity(UI_SENSITIVITY_Y),
        )
        .with_axis(
            MoveLeftRight2d,
            GamepadControlAxis::LEFT_X
                .with_deadzone_symmetric(0.25)
                .sensitivity(UI_SENSITIVITY_X),
        )
        .with_axis(
            MoveUpDown2d,
            GamepadControlAxis::LEFT_Y
                .inverted()
                .with_deadzone_symmetric(0.5)
                .sensitivity(UI_SENSITIVITY_Y),
        );

    // Note: this usage as an action is only processed in gameplay
    // (which, being pauseable, means there'd be no way to escape),
    // but KeyCode::Escape is elsewhere handled manually in an unpauseable way.
    input_map.insert(ToggleMenu, KeyCode::Escape);
    input_map.insert(ToggleMenu, GAMEPAD_BUTTON_MENU);

    input_map.insert(Back, KeyCode::Escape);
    input_map.insert(Back, GamepadButton::East);

    input_map.insert(TogglePause, KeyCode::Pause);
    input_map.insert(
        TogglePause,
        ButtonlikeChord::new([CTRL_COMMAND, KeyCode::KeyP]),
    ); // "P"ause
    input_map.insert(TogglePause, GamepadButton::Mode);

    input_map.insert(ToggleMute, KeyCode::F12);
    input_map.insert(
        ToggleMute,
        ButtonlikeChord::new([CTRL_COMMAND, KeyCode::KeyM]),
    ); // "M"ute

    input_map.insert(ToggleFullScreen, KeyCode::F11);

    input_map.insert(ToggleDebugUi, KeyCode::Backquote);

    input_map
}

/// This provides mappings for WASD + Space/C + mouse bindings + gamepad control for FPS or space controllers.
pub fn default_fps_input_map() -> InputMap<UserAction> {
    use actions_common::UserAction::*;

    let mut input_map = InputMap::default()
        .with_dual_axis(MoveFlycam, VirtualDPad::wasd().inverted_y())
        .with_dual_axis(
            MoveFlycam,
            GamepadStick::LEFT
                .inverted_y()
                .with_deadzone_symmetric_unscaled(0.25)
                .with_processor(DualAxisSensitivity::all(1.0)),
        )
        .with_axis(MoveDownUp, VirtualAxis::new(KeyCode::KeyC, KeyCode::Space))
        // .with_axis(MoveDownUp, GamepadAxis::LeftZ)
        .with_axis(
            MoveDownUp,
            VirtualAxis::new(GamepadButton::DPadDown, GamepadButton::DPadUp),
        )
        .with_axis(
            MoveLeftRight,
            VirtualAxis::new(GamepadButton::DPadLeft, GamepadButton::DPadRight),
        )
        .with_dual_axis(Look, MouseMove::default())
        .with_dual_axis(
            Look,
            GamepadStick::RIGHT
                .inverted_y()
                .with_deadzone_symmetric_unscaled(0.25)
                .with_processor(DualAxisSensitivity::all(100.0)),
        )
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

    // input_map.insert_axis(Tilt, MouseScrollAxis::X);

    input_map.insert(Accelerate, ModifierKey::Shift);
    input_map.insert(Accelerate, GamepadButton::West);

    input_map.insert(ToggleCrouch, ModifierKey::Control);
    // input_map.insert(ToggleCrouch, GamepadButton::DPadDown);
    // input_map.insert(ToggleCrouch, GamepadButton::LeftThumb);

    input_map.insert(Crouch, KeyCode::KeyC);
    input_map.insert(Crouch, GamepadButton::LeftThumb);

    input_map.insert(TurnAround, KeyCode::Backspace);
    input_map.insert(TurnAround, GamepadButton::LeftTrigger);

    // Home, i.e. to reset the camera to home position.
    input_map.insert(Home, KeyCode::Backslash);

    input_map.insert(Fire, MouseButton::Left);
    input_map.insert(
        ShiftFire,
        ButtonlikeChord::modified(ModifierKey::Shift, MouseButton::Left),
    );
    input_map.insert(Fire, GamepadButton::RightTrigger);

    input_map.insert(AlternateFire, MouseButton::Right);
    input_map.insert(
        ShiftAlternateFire,
        ButtonlikeChord::modified(ModifierKey::Shift, MouseButton::Right),
    );

    input_map.insert(Interact, KeyCode::KeyE);
    input_map.insert(Interact, KeyCode::Enter);
    input_map.insert(Interact, GamepadButton::South);

    input_map.insert(Reset, KeyCode::Backspace);
    input_map.insert(Reset, GamepadButton::LeftTrigger);

    input_map
}
