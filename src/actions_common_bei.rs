use crate::*;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::window::WindowMode;
use bevy_enhanced_input::prelude::*;
use bevy_seedling::prelude::MainBus;

pub const MOD_CTRL_COMMAND: ModKeys = if cfg!(target_os = "macos") {
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
            .add_observer(handle_mute)

            ;
    }
}

/// Context for gameplay.
/// Note, this is a parent. Use PlayerAction to detect.
#[derive(Component, Reflect)]
#[reflect(Component)]
#[type_path = "game"]
pub struct PlayerContext;

/// Context for menu.
/// Note, this is a parent. Use MenuAction to detect.
#[derive(Component, Reflect)]
#[reflect(Component)]
#[type_path = "game"]
pub struct MenuContext;

/// Marker for Actions on a Player.
#[derive(Component, Reflect, Clone)]
#[reflect(Component)]
#[type_path = "game"]

pub struct PlayerAction;

/// Marker for Actions on a menu.
#[derive(Component, Reflect, Clone)]
#[reflect(Component)]
#[type_path = "game"]
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

    /// Button version of crouching.
    #[derive(InputAction)]
    #[action_output(bool)]
    pub struct Crouch;

    /// Button version of jump.
    #[derive(InputAction)]
    #[action_output(bool)]
    pub struct Jump;

    /// Switch camera perspective.
    #[derive(InputAction)]
    #[action_output(bool)]
    pub struct ChangeCamera;

    /// Move in the XZ plane (forward/back/strafe).
    #[derive(InputAction)]
    #[action_output(Vec2)]
    pub struct MoveFlycam;

    /// Move in the Y axis (fly/dive).
    #[derive(InputAction)]
    #[action_output(f32)]
    pub struct MoveDownUp;

    /// Move in the X axis (i.e. strafe).
    #[derive(InputAction)]
    #[action_output(f32)]
    pub struct MoveLeftRight;

    /// Change camera to be closer/further from some object.
    #[derive(InputAction)]
    #[action_output(Vec2)]
    pub struct Zoom;

    /// Turn the camera up/down on X and left/right on Y axes.
    #[derive(InputAction)]
    #[action_output(Vec2)]
    pub struct Look;

    /// Select items in the scene.
    #[derive(InputAction)]
    #[action_output(bool)]
    pub struct ToggleSelect(pub Entity);

    /// (Try to) grab selected item(s).
    #[derive(InputAction)]
    #[action_output(bool)]
    pub struct StartGrab;

    /// (Try to) stop grabbing items.
    /// This has a lead-up time so that quick taps release/drop the item,
    /// but longer presses fire the item.
    #[derive(InputAction)]
    #[action_output(bool)]
    pub struct ReleaseGrab;

    /// Move further/closer away, either hovered item or distance of grabbed item.
    #[derive(InputAction)]
    #[action_output(f32)]
    pub struct CycleHighlightedItem;
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
    mut commands: Commands,
    mut gui_state: ResMut<GuiState>,
) {
    gui_state.enabled = dev_tools_enabled() && !gui_state.enabled;
    commands.write_message(GrabCursor(!gui_state.enabled));
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

const UI_SENSITIVITY_X: f32 = 8.0;    // relatively quick for sliders
const UI_SENSITIVITY_Y: f32 = 7.0;    // move through menus slower

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
            KeyCode::KeyM.with_mod_keys(MOD_CTRL_COMMAND),
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
        ActionSettings {
            require_reset: true,
            ..default()
        },
        bindings![
            KeyCode::KeyE,
            KeyCode::Enter,
            GamepadButton::South,
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
            GamepadButton::East,    // not GAMEPAD_BUTTON_MENU here
        ],
    ));

    commands.spawn((
        include.clone(),

        Action::<actions::MoveDownUp>::new(),
        DeadZone::default(),
        DeltaScale::default(),
        Scale::splat(UI_SENSITIVITY_Y),
        Bindings::spawn((
            Bidirectional::new(KeyCode::ArrowDown, KeyCode::ArrowUp),
            Bidirectional::new(GamepadButton::DPadDown, GamepadButton::DPadUp),
        )),
    ));
    commands.spawn((
        include.clone(),

        Action::<actions::MoveLeftRight>::new(),
        DeadZone::default(),
        DeltaScale::default(),
        Scale::splat(UI_SENSITIVITY_X),
        Bindings::spawn((
            Bidirectional::new(KeyCode::ArrowRight, KeyCode::ArrowLeft),
            Bidirectional::new(GamepadButton::DPadRight, GamepadButton::DPadLeft),
        )),
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
        Bindings::spawn((
            Bidirectional::new(GamepadButton::DPadRight, GamepadButton::DPadLeft),
        )),
    ));
    commands.spawn((
        include.clone(),
        Action::<actions::Look>::new(),
        Bindings::spawn((
            Spawn((Binding::mouse_motion(), Scale::new(Vec3::splat(1.0)))),
            Axial::right_stick()
                .with((
                    DeadZone::default(),
                    Scale::new(Vec3::splat(100.0)),
                    Negate::y(),
                    SmoothNudge::default(),
                )),
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
            GamepadButton::West,
        ],
    ));
    commands.spawn((
        include.clone(),
        Action::<actions::Jump>::new(),
        bindings![
            KeyCode::Space,
            GamepadButton::East,
        ],
    ));
    commands.spawn((
        include.clone(),
        Action::<actions::TurnAround>::new(),
        bindings![
            KeyCode::Backspace,
            GamepadButton::RightThumb,
        ],
    ));
    commands.spawn((
        include.clone(),
        Action::<actions::Firing>::new(),
        ActionSettings {
            require_reset: true,
            ..default()
        },
        bindings![
            MouseButton::Left,
            KeyCode::Enter,
            GamepadButton::RightTrigger2,
        ],
    ));
    commands.spawn((
        include.clone(),
        Action::<actions::Reset>::new(),
        bindings![
            KeyCode::Backslash,
        ],
    ));

    commands.spawn((
        include.clone(),
        Action::<actions::StartGrab>::new(),
        bindings![
            MouseButton::Right,
            GamepadButton::LeftTrigger2,

            KeyCode::KeyF,

            // These are dangerous since they must be used in isolation
            // and not with keyboard combinations.
            KeyCode::AltLeft,
            KeyCode::AltRight,
        ],
    ));

    commands.spawn((
        include.clone(),
        Action::<actions::ReleaseGrab>::new(),
        Hold::new(0.5),
        // Cooldown::new(0.125),
        bindings![
            MouseButton::Left,
            GamepadButton::RightTrigger2,
        ],
    ));
    commands.spawn((
        include.clone(),
        Action::<actions::CycleHighlightedItem>::new(),
        Scale::splat(0.25),
        Bindings::spawn((
            Spawn((Binding::mouse_wheel(), SwizzleAxis::YYY)),
            Bidirectional::new(KeyCode::ArrowUp, KeyCode::ArrowDown),
            Bidirectional::new(GamepadButton::RightTrigger, GamepadButton::LeftTrigger),
        )),
    ));

}
