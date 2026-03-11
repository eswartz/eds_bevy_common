use crate::*;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::window::WindowMode;
use bevy_enhanced_input::prelude::*;
use bevy_seedling::prelude::MainBus;

pub const CTRL_COMMAND: ModKeys = if cfg!(target_os = "macos") {
    ModKeys::SUPER
} else {
    ModKeys::CONTROL
};

pub struct ActionPlugin;
impl Plugin for ActionPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EnhancedInputPlugin)
            .add_input_context::<PlayerContext>()
            .add_input_context::<MenuContext>()
            .add_systems(Update, handle_escape)
            .add_systems(Update, toggle_context.run_if(resource_changed::<State<OverlayState>>))
            .add_observer(handle_pause)
            .add_observer(handle_debug_ui)
            .add_observer(handle_full_screen)
            .add_observer(handle_mute);
    }
}

/// Context for gameplay.
/// Note, this is a parent. Use PlayerAction to detect.
#[derive(Component, Reflect)]
pub struct PlayerContext;

/// Context for menu.
/// Note, this is a parent. Use MenuAction to detect.
#[derive(Component, Reflect)]
pub struct MenuContext;

/// Marker for Actions on a Player.
#[derive(Component, Reflect, Clone)]
#[reflect(Component)]
pub struct PlayerAction;

/// Marker for Actions on a menu.
#[derive(Component, Reflect, Clone)]
#[reflect(Component)]
pub struct MenuAction;

pub mod actions {
    use super::*;

    #[derive(InputAction)]
    #[action_output(bool)]
    pub struct Pause;

    #[derive(InputAction)]
    #[action_output(bool)]
    pub struct Mute;

    /// Enter the menu from anywhere.
    #[derive(InputAction)]
    #[action_output(bool)]
    pub struct Menu;

    #[derive(InputAction)]
    #[action_output(bool)]
    pub struct DebugUi;

    #[derive(InputAction)]
    #[action_output(bool)]
    pub struct FullScreen;

    /// Go back (i.e. out of menu or dialog)
    #[derive(InputAction)]
    #[action_output(bool)]
    pub struct Back;

    #[derive(InputAction)]
    #[action_output(bool)]
    pub struct Firing;

    #[derive(InputAction)]
    #[action_output(bool)]
    pub struct Interact;

    /// Reset something (look to identity, menu item to default)
    #[derive(InputAction)]
    #[action_output(bool)]
    pub struct Reset;

    /// Turn around 180 degrees around Y axis.#[derive(InputAction)]
    #[derive(InputAction)]
    #[action_output(bool)]
    pub struct TurnAround;

    /// When held, move faster (i.e. Shift).
    #[derive(InputAction)]
    #[action_output(bool)]
    pub struct Accelerate;

    #[derive(InputAction)]
    #[action_output(bool)]
    pub struct Crouch;

    /// Switch perspective.
    #[derive(InputAction)]
    #[action_output(bool)]
    pub struct ChangeCamera;

    #[derive(InputAction)]
    #[action_output(Vec2)]
    pub struct MoveFlycam;

    #[derive(InputAction)]
    #[action_output(f32)]
    pub struct MoveDownUp;

    #[derive(InputAction)]
    #[action_output(f32)]
    pub struct MoveLeftRight;

    /// Get closer/further from active object.
    #[derive(InputAction)]
    #[action_output(Vec2)]
    pub struct Zoom;

    /// Turn the camera up/down on X and left/right on Y axes.
    #[derive(InputAction)]
    #[action_output(Vec2)]
    pub struct Look;

    #[derive(InputAction)]
    #[action_output(bool)]
    pub struct ForceWin;

    #[derive(InputAction)]
    #[action_output(bool)]
    pub struct ForceLose;
}

fn toggle_context(
    mut commands: Commands,
    player_active: Single<Entity, With<ContextActivity::<PlayerContext>>>,
    menu_active: Single<Entity, With<ContextActivity::<MenuContext>>>,
    overlay: Res<State<OverlayState>>,
) {
    if overlay.is_menu() {
        commands.entity(*player_active).insert(ContextActivity::<PlayerContext>::INACTIVE);
        commands.entity(*menu_active).insert(ContextActivity::<MenuContext>::ACTIVE);
    } else {
        commands.entity(*player_active).insert(ContextActivity::<PlayerContext>::ACTIVE);
        commands.entity(*menu_active).insert(ContextActivity::<MenuContext>::INACTIVE);
    }

}

pub(crate) fn handle_pause(_event: On<Start<actions::Pause>>, mut pause_state: ResMut<PauseState>) {
    // Toggle from whatever means we are paused, as an
    // escape hatch.
    let paused = !pause_state.is_paused();
    pause_state.set_user_paused(paused);
}

pub(crate) fn handle_debug_ui(
    _event: On<Start<actions::DebugUi>>,
    mut gui_state: ResMut<GuiState>,
) {
    gui_state.enabled = dev_tools_enabled() && !gui_state.enabled;
}

pub(crate) fn handle_full_screen(
    _event: On<Start<actions::FullScreen>>,
    mut primary_window: Query<&mut Window, With<PrimaryWindow>>,
) {
    let Ok(mut window) = primary_window.single_mut() else {
        return;
    };

    let cur_mode = window.mode;
    window.mode = match cur_mode {
        WindowMode::Windowed => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
        WindowMode::BorderlessFullscreen(_monitor_selection) => WindowMode::Windowed,

        // WindowMode::BorderlessFullscreen(monitor_selection) => WindowMode::Fullscreen(
        //     monitor_selection, VideoModeSelection::Current),
        WindowMode::Fullscreen(_monitor_selection, _video_mode_selection) => WindowMode::Windowed,
    }
}

pub(crate) fn handle_mute(
    _event: On<Start<actions::Mute>>,
    mut vol_q: Single<&mut UserVolume, With<MainBus>>,
) {
    vol_q.muted = !vol_q.muted;
}

// pub(crate) fn toggle_pointer_actions(
//     overlay: Res<State<OverlayState>>,
//     mut state: ResMut<ActionState<UserAction>>,
// ) {
//     // Just to be safe, turn off all look actions while debug UI is open.
//     let Some(look_axis) = state.action_data_mut(&UserAction::Look) else { return };
//     if overlay.is_debug() {
//         look_axis.disabled = true;
//     } else {
//         look_axis.disabled = false;
//     }
// }

#[macro_export]
macro_rules! add_actions {
    ($context:ty [$($action:expr),*$(,)?]) => {
        ::bevy::prelude::related!($crate::prelude::Actions<$context>[$($action),*])
    };
}

const UI_SENSITIVITY_X: f32 = 1.0 / 5.;     // relatively quick for sliders
const UI_SENSITIVITY_Y: f32 = 1.0 / 15.;    // move through menus slower

/// Assign actions to your own context/etc.
/// include: should be at least e.g. `ActionOf::<YourContext>::new(context_entity)`
pub fn assign_stock_common_actions(
    mut commands: Commands,
    include: impl Bundle + Clone,
) {
    commands.spawn((
        include.clone(),
        Action::<actions::Pause>::new(),
        bindings![
            KeyCode::Pause,
            KeyCode::KeyP.with_mod_keys(ModKeys::CONTROL),
            GamepadButton::Mode,
        ],
    ));
    commands.spawn((
        include.clone(),
        Action::<actions::Mute>::new(),
        bindings![
            KeyCode::F12,
            KeyCode::KeyM.with_mod_keys(CTRL_COMMAND),
        ],
    ));
    commands.spawn((
        include.clone(),
        Action::<actions::FullScreen>::new(),
        bindings![
            KeyCode::F11,
            KeyCode::Enter.with_mod_keys(ModKeys::ALT),
        ],
    ));
    commands.spawn((
        include.clone(),
        Action::<actions::DebugUi>::new(),
        bindings![
            KeyCode::Backquote,
        ],
    ));

    commands.spawn((
        include.clone(),
        Action::<actions::Interact>::new(),
        bindings![
            KeyCode::KeyE,
            KeyCode::Enter,
            GamepadButton::South,
        ],
    ));
    commands.spawn((
        include.clone(),
        Action::<actions::Reset>::new(),
        bindings![
            KeyCode::Backspace,
            GamepadButton::LeftTrigger,
        ],
    ));

}

/// include: should be at least e.g. `(ActionOf::<YourContext>::new(context_entity), {Menu,Player}Action)`
pub fn assign_stock_menu_actions(
    mut commands: Commands,
    include: impl Bundle + Clone,
) {
    commands.spawn((
        include.clone(),
        Action::<actions::Back>::new(),
        ActionSettings {
            require_reset: true,
            ..default()
        },
        bindings![
            KeyCode::Escape,
            GamepadButton::East,
        ],
    ));

    commands.spawn((
        include.clone(),

        Action::<actions::MoveDownUp>::new(),
        DeadZone::default(),
        // SmoothNudge::default(),
        // DeltaScale::default(),
        Scale::new(Vec3::new(UI_SENSITIVITY_X, UI_SENSITIVITY_Y, 1.0)),
        Bindings::spawn((
            Bidirectional::new(KeyCode::ArrowDown, KeyCode::ArrowUp),
            Bidirectional::new(GamepadButton::DPadDown, GamepadButton::DPadUp),
        )),
    ));
    commands.spawn((
        include.clone(),

        Action::<actions::MoveLeftRight>::new(),
        DeadZone::default(),
        // SmoothNudge::default(),
        // DeltaScale::default(),
        Scale::new(Vec3::new(UI_SENSITIVITY_X, UI_SENSITIVITY_Y, 1.0)),
        Bindings::spawn((
            Bidirectional::new(KeyCode::ArrowLeft, KeyCode::ArrowRight),
            Bidirectional::new(GamepadButton::DPadLeft, GamepadButton::DPadRight),
        )),
    ));
    // commands.spawn((
    //     // Note: this usage as an action is only processed in gameplay
    //     // (which, being pauseable, means there'd be no way to escape),
    //     // but KeyCode::Escape is elsewhere handled manually in an unpauseable way.
    //     include.clone(),

    //     Action::<actions::Menu>::new(),
    //     bindings![
    //         KeyCode::Escape,
    //         GAMEPAD_BUTTON_MENU,
    //     ],
    // ));
    // commands.spawn((
    //     include.clone(),

    //     Action::<actions::Back>::new(),
    //     bindings![
    //         KeyCode::Escape,
    //         GamepadButton::East,
    //     ],
    // ));
}

/// include: should be at least e.g. `(ActionOf::<YourContext>::new(context_entity), {Menu,Player}Action)`
pub fn assign_stock_player_actions(
    mut commands: Commands,
    include: impl Bundle + Clone,
) {
    commands.spawn((
        // Note: this usage as an action is only processed in gameplay
        // (which, being pauseable, means there'd be no way to escape),
        // but KeyCode::Escape is elsewhere handled manually in an unpauseable way.
        include.clone(),
        Action::<actions::Menu>::new(),
        ActionSettings {
            require_reset: true,
            ..default()
        },
        bindings![
            KeyCode::Escape,
            GAMEPAD_BUTTON_MENU,
            ],
        ));

    commands.spawn((
        include.clone(),
        Action::<actions::MoveFlycam>::new(),
        // DeadZone::default(),
        // SmoothNudge::default(),
        // DeltaScale::default(),
        Negate::y(),
        Bindings::spawn((
            Cardinal::wasd_keys(),
            Cardinal::dpad(),
            Axial::left_stick(),
        )),
    ));
    commands.spawn((
        include.clone(),
        Action::<actions::MoveDownUp>::new(),
        // DeadZone::default(),
        // SmoothNudge::default(),
        // DeltaScale::default(),
        Bindings::spawn((
            Bidirectional::new(KeyCode::Space, KeyCode::KeyC),
            Bidirectional::new(GamepadButton::DPadUp, GamepadButton::DPadDown),
        )),
    ));
    commands.spawn((
        include.clone(),
        Action::<actions::MoveLeftRight>::new(),
        // DeadZone::default(),
        // SmoothNudge::default(),
        // DeltaScale::default(),
        Bindings::spawn((
            Bidirectional::new(GamepadButton::DPadLeft, GamepadButton::DPadRight),
        )),
    ));
    commands.spawn((
        include.clone(),
        Action::<actions::Look>::new(),
        Negate::y(),
        // DeadZone::default(),
        // SmoothNudge::default(),
        // DeltaScale::default(),
        // Scale::new(Vec3::splat(1.0)),
        Bindings::spawn((
            Spawn((Binding::mouse_motion(), Negate::y())),
            Axial::right_stick(),
        )),
    ));
    commands.spawn((
        include.clone(),
        Action::<actions::Accelerate>::new(),
        bindings![KeyCode::ShiftLeft, KeyCode::ShiftRight],
    ));
    commands.spawn((
        include.clone(),
        Action::<actions::Crouch>::new(),
        bindings![
            KeyCode::KeyC,
            KeyCode::ControlRight,
            GamepadButton::LeftThumb,
        ],
    ));
    commands.spawn((
        include.clone(),
        Action::<actions::TurnAround>::new(),
        bindings![
            KeyCode::Backspace,
            GamepadButton::LeftTrigger,
        ],
    ));
    commands.spawn((
        include.clone(),
        Action::<actions::Firing>::new(),
        bindings![
            MouseButton::Left,
            KeyCode::Enter,
            GamepadButton::RightTrigger,
        ],
    ));

}
