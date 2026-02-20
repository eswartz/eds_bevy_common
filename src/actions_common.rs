use crate::*;
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
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
            );
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

    /// UI editing.
    #[actionlike(Axis)]
    MoveLeftRight2d,
    #[actionlike(Axis)]
    MoveDownUp2d,

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

/// Handle Escape, which is handled differently outside and inside menus.
///
fn handle_escape(
    mut commands: Commands,
    overlay_state: Res<State<OverlayState>>,
    going_back: Option<Res<GoBackInMenuRequest>>,
    mut previous_menu: ResMut<PreviousMenuStack>,
    // actions: Res<ActionState<UserAction>>,
    mut reader: MessageReader<KeyboardInput>,
    mut pause: ResMut<PauseState>,
) {
    // // Menu logic handles this itself.
    // if overlay_state.is_menu() {
    //     return;
    // }
    if going_back.is_some() {
        return;
    }

    for key_event in reader.read() {
        if key_event.state == ButtonState::Pressed && key_event.key_code == KeyCode::Escape {
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
}

/// Process actions, sampling actions globally.
///
/// Clients handle sub-UserActions on their own
/// in similar systems. Multiple clients independently
/// see the UserActions and can respond appropriately.
fn process_global_actions(
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
