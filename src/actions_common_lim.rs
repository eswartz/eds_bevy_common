use crate::*;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::window::WindowMode;
use bevy_seedling::prelude::MainBus;
use leafwing_input_manager::action_diff::ActionDiffMessage;
use leafwing_input_manager::prelude::*;
use strum_macros::EnumIter;

pub const CTRL_COMMAND: KeyCode = if cfg!(target_os = "macos") {
    KeyCode::SuperLeft
} else {
    KeyCode::ControlLeft
};

pub const MOD_CTRL_COMMAND: ModifierKey = if cfg!(target_os = "macos") {
    ModifierKey::Super
} else {
    ModifierKey::Control
};

pub struct ActionPlugin;
impl Plugin for ActionPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InputManagerPlugin::<UserAction>::default())
            .register_type::<UserAction>()
            .add_message::<ActionDiffMessage<UserAction>>()
            .init_resource::<ActionState<UserAction>>()
            // Note: this only helps with Buttonlike actions. [Dual]Axis actions are not considered.
            .insert_resource(ClashStrategy::PrioritizeLongest)
            .add_systems(
                Update,
                (
                    process_global_actions,
                    handle_escape,
                ),
            )
            .add_systems(
                Update,
                toggle_pointer_actions
                    .run_if(resource_changed::<State<OverlayState>>),
            )
            ;
    }
}

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect, EnumIter)]
#[type_path = "game"]
pub enum UserAction {
    TogglePause,
    ToggleMute,

    /// If multiple music tracks supported, switch it.
    SwitchNextAudioTrack,
    /// If multiple music tracks supported, switch it.
    SwitchPrevAudioTrack,

    /// Enter/exit menu.
    ToggleMenu,
    /// If there is debug UI (i.e. inspector/cheats), toggle it.
    ToggleDebugUi,
    ToggleHelp,

    ToggleFps,
    ToggleSkybox,
    ToggleFullScreen,

    SaveState,
    LoadState,
    DumpState,

    /// Move relative to the camera rotation (flycam).
    #[actionlike(DualAxis)]
    MoveFlycam,
    /// Move up/down from camera rotation.
    #[actionlike(Axis)]
    MoveDownUp,
    /// Move left/right from camera rotation.
    #[actionlike(Axis)]
    MoveLeftRight,

    /// UI editing.
    #[actionlike(Axis)]
    MoveLeftRight2d,
    #[actionlike(Axis)]
    MoveUpDown2d,
    #[actionlike(DualAxis)]
    Move2d,

    /// Back in a menu.
    Back,

    /// All-purpose "fire" (e.g. left-click)
    Fire,
    /// Shift+Fire.
    ShiftFire,
    /// Alt-Fire (e.g. right-click)
    AlternateFire,
    /// Shift+Alt-Fire (e.g. right-click)
    ShiftAlternateFire,

    /// All-purpose "action".
    Interact,

    /// Reset (for menus).
    Reset,

    /// Tilt/roll the camera on Z axis.
    #[actionlike(Axis)]
    Tilt,
    /// Turn the camera up/down on X and left/right on Y axes.
    #[actionlike(DualAxis)]
    Look,
    /// Reset orientation to identity.
    Home,

    /// Get closer/further from active object.
    #[actionlike(Axis)]
    Zoom,

    /// Turn around 180 degrees around Y axis.
    TurnAround,

    /// When held, move faster (i.e. Shift).
    Accelerate,

    /// When held, lower camera and move slower (i.e. Ctrl).
    ToggleCrouch,
    /// When held, lower camera and move slower (i.e. Ctrl).
    Crouch,

    /// Switch perspective.
    ChangeCamera,

    /// Force winning the level.
    ForceWin,
    /// Force losing the level.
    ForceLose,
}

/// Process actions, sampling actions globally.
///
/// Clients handle sub-UserActions on their own
/// in similar systems. Multiple clients independently
/// see the UserActions and can respond appropriately.
pub(crate) fn process_global_actions(
    mut commands: Commands,
    action_state: Res<ActionState<UserAction>>,
    overlay_state: Res<State<OverlayState>>,
    mut primary_window: Query<&mut Window, With<PrimaryWindow>>,
    mut pause_state: ResMut<PauseState>,
    mut vol_q: Single<&mut UserVolume, With<MainBus>>,
) {
    if action_state.just_pressed(&UserAction::TogglePause) {
        // Toggle from whatever means we are paused, as an
        // escape hatch.
        let paused = !pause_state.is_paused();
        pause_state.set_user_paused(paused);
    }
    if action_state.just_pressed(&UserAction::ToggleDebugUi) {
        if show_dev_tools() {
            if !overlay_state.is_menu() {
                commands.set_state(match overlay_state.get() {
                    OverlayState::Hidden => OverlayState::DebugGuiVisible,
                    OverlayState::DebugGuiVisible => OverlayState::Hidden,
                    current => *current,
                });
            }
        }
    }
    if action_state.just_pressed(&UserAction::ToggleFullScreen)
        && let Ok(mut window) = primary_window.single_mut()
    {
        let cur_mode = window.mode;
        window.mode = match cur_mode {
            WindowMode::Windowed => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
            WindowMode::BorderlessFullscreen(_monitor_selection) => WindowMode::Windowed,

            // WindowMode::BorderlessFullscreen(monitor_selection) => WindowMode::Fullscreen(
            //     monitor_selection, VideoModeSelection::Current),
            WindowMode::Fullscreen(_monitor_selection, _video_mode_selection) => {
                WindowMode::Windowed
            }
        };
    }
    if action_state.just_pressed(&UserAction::ToggleMute) {
        vol_q.muted = !vol_q.muted;
    }

    // other [UserAction]::s handled separately.
}

pub(crate) fn toggle_pointer_actions(
    overlay: Res<State<OverlayState>>,
    mut state: ResMut<ActionState<UserAction>>,
) {
    // Just to be safe, turn off all look actions while debug UI is open.
    let Some(look_axis) = state.action_data_mut(&UserAction::Look) else { return };
    if overlay.is_debug() {
        look_axis.disabled = true;
    } else {
        look_axis.disabled = false;
    }
}


/// These actions are useful for GUIs.
pub fn default_gui_input_map() -> InputMap<UserAction> {
    use actions_common_lim::UserAction::*;

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
    use actions_common_lim::UserAction::*;

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
