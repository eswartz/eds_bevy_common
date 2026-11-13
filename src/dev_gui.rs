use crate::prelude::*;
use avian3d::debug_render::PhysicsGizmos;
use bevy::{input::keyboard::Key, prelude::*};

use super::states_sets::ProgramState;

// Included by DebugPlugin, but you can override to change the debug KeyCode.
pub struct DevGuiPlugin {
    pub debug_key_code: KeyCode,
}

impl Default for DevGuiPlugin {
    fn default() -> Self {
        Self {
            debug_key_code: KeyCode::Backquote,
        }
    }
}

impl Plugin for DevGuiPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DevGuiState::default())
            .add_systems(
                OnEnter(ProgramState::InGame),
                update_gui_state, // initialize
            )
            .add_systems(
                Update,
                (
                    handle_debug_ui_toggle,
                    update_gui_state.run_if(resource_changed::<DevGuiState>),
                ),
            );
    }
}

/// State controlling developer overlays.
#[derive(Resource, Debug, Clone, PartialEq, Reflect)]
#[reflect(Resource)]
#[type_path = "game"]
pub struct DevGuiState {
    pub enabled: bool,
    pub show_stats: bool,
    pub show_inspector: bool,
    /// Show inspector even if !enabled.
    pub show_inspector_always: bool,
    pub show_physics_gizmos: bool,
    pub show_player_status: bool,
}

impl Default for DevGuiState {
    fn default() -> Self {
        Self {
            enabled: false,
            show_stats: false,
            show_inspector: true,
            show_inspector_always: false,
            show_physics_gizmos: false,
            show_player_status: false,
        }
    }
}

impl DevGuiState {
    pub fn is_debug_ui_inspector_visible(&self) -> bool {
        self.show_inspector_always || (self.enabled && self.show_inspector)
    }
    pub fn show_cursor(&self) -> bool {
        self.enabled || self.show_inspector_always
    }
    pub fn center_cursor(&self) -> bool {
        !self.enabled
    }
}

pub fn is_debug_ui_enabled(gui_state: Option<Res<DevGuiState>>) -> bool {
    gui_state.is_some_and(|g| g.enabled)
}

pub fn is_debug_ui_inspector_visible(gui_state: Option<Res<DevGuiState>>) -> bool {
    gui_state.is_some_and(|g| g.is_debug_ui_inspector_visible())
}

/// Handle the debug UI key, which is handled globally and
/// not passed to the egui debug UI (else it enters
/// that character in searches/etc which is unlikely
/// to be what anyone wants) and avoids needing to
/// use Enter or Tab to complete a text entry or
/// release focus.
///
/// This system should be used without filters.
pub(crate) fn handle_debug_ui_toggle(
    mut commands: Commands,
    mut gui_state: If<ResMut<DevGuiState>>,
    key_codes: Res<ButtonInput<KeyCode>>,
    keys: Res<ButtonInput<Key>>,
) {
    dbg!(keys.get_just_pressed().collect::<Vec<_>>());
    if key_codes.just_pressed(DEBUG_UI_KEYCODE)
        && !key_codes.any_pressed([
            // KeyCode::ShiftLeft,      // i.e. for tilde
            KeyCode::ShiftRight, // allow for other key layouts
            KeyCode::ControlLeft,
            KeyCode::ControlRight,
            KeyCode::AltLeft,
            // KeyCode::AltRight,       // exclude AltGr for dead-key usage
            KeyCode::SuperLeft,
            KeyCode::SuperRight,
        ])
    {
        if dev_tools_enabled() {
            gui_state.enabled ^= true;
            commands.write_message(GrabCursor(!gui_state.show_cursor()));
        }
    }
}

pub(crate) fn update_gui_state(
    state: If<Res<DevGuiState>>,
    fps_visible: Option<ResMut<StatsOverlayVisible>>,
    status_visible: Option<ResMut<StatusVisible>>,
    mut gizmo_config: ResMut<GizmoConfigStore>,
) {
    if let Some(mut fps_visible) = fps_visible {
        fps_visible.0 = state.show_stats || state.enabled;
    }
    if let Some(mut status_visible) = status_visible {
        status_visible.0 = state.show_player_status;
    }

    if let Some(physics_gizmos) = gizmo_config.get_config_mut::<PhysicsGizmos>() {
        let was_enabled = physics_gizmos.0.enabled;
        if was_enabled != state.show_physics_gizmos {
            // Only trigger on actual change,
            // to avoid avian3d::debug_render::change_mesh_visibility
            // showing everything without recourse.
            physics_gizmos.0.enabled = state.show_physics_gizmos;
        }
    }
}
