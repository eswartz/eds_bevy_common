use std::collections::BTreeMap;

use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::{EguiContext, EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass, PrimaryEguiContext, input::{EguiWantsInput, egui_wants_any_keyboard_input, egui_wants_any_pointer_input}};
use bevy_inspector_egui::DefaultInspectorConfigPlugin;

use crate::*;

use super::{gui::GuiState, states_sets::OverlayState};

/// You need to manually add EguiPlugin and DefaultInspectorConfigPlugin.
pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<EguiPlugin>() {
            app.add_plugins(EguiPlugin::default());
            app.insert_resource(EguiGlobalSettings {
                auto_create_primary_context: false,
                ..default()
            });
        }
        if !app.is_plugin_added::<DefaultInspectorConfigPlugin>() {
            app.add_plugins(DefaultInspectorConfigPlugin);
        }

        app
            .init_resource::<DebugEguiCamera>()

            .add_systems(
                PreUpdate,
                (setup_egui_style, ensure_egui_context)
                    .chain()
                    .run_if(not(egui_initialized))
                    .run_if(in_state(GameplayState::Playing))
                    ,
            )

            .add_systems(
                EguiPrimaryContextPass,
                (
                    update_egui_inspector_ui
                    .run_if(
                        |gui_state: Res<GuiState>, ovl_state: Res<State<OverlayState>>|
                            gui_state.show_inspector_always ||
                            (gui_state.show_inspector && ovl_state.get().is_debug())
                        )
                    ,
                ),
            )
        ;
    }
}

/// Which 3D camera hosts egui UI?
#[derive(Resource, Reflect, Default, PartialEq, Debug)]
#[reflect(Resource)]
pub enum DebugEguiCamera {
    WorldCamera,
    #[default]
    ViewerCamera,
}

// Define a custom `SystemParam` for our collision hooks.
// It can have read-only access to queries, resources, and other system parameters.
#[derive(SystemParam)]
pub struct DebugEguiCameraQuery<'w, 's> {
    debug_camera: Res<'w, DebugEguiCamera>,
    camera_q: Query<'w, 's, (Has<WorldCamera>, Has<ViewerCamera>), With<Camera3d>>,
}

impl<'w, 's> DebugEguiCameraQuery<'w, 's> {
    /// Is this camera the one matching [DebugEguiCamera]?
    pub fn is_debug_camera(&self, camera: Entity) -> bool {
        if let Ok((is_world, is_view)) = self.camera_q.get(camera) {
            match &*self.debug_camera {
                DebugEguiCamera::WorldCamera => is_world,
                DebugEguiCamera::ViewerCamera => is_view,
            }
        } else {
            false
        }
    }
}

pub fn egui_initialized(
    camera_q: Query<Entity, (With<Camera3d>, With<PrimaryEguiContext>)>,
    debug: DebugEguiCameraQuery,
) -> bool
{
    for ent in camera_q.iter() {
        if debug.is_debug_camera(ent) {
            return true;
        }
    }

    false
}

pub fn ensure_egui_context(
    mut commands: Commands,
    camera_q: Query<Entity, (With<Camera3d>, Without<PrimaryEguiContext>)>,
    debug: DebugEguiCameraQuery,
) {
    for camera_ent in camera_q.iter() {
        if debug.is_debug_camera(camera_ent) {
            dbg!(camera_ent);
            commands.entity(camera_ent).insert(
                PrimaryEguiContext,
            );
        }
    }
}

pub fn setup_egui_style(
    mut q: Query<(&mut EguiContext, Option<&PrimaryEguiContext>)>,
) {
    let Ok((mut ctx, Some(_))) = q.single_mut() else { return };
    dbg!("got");
    let ctx = ctx.get_mut();
    {
        use egui::FontFamily::Proportional;
        use egui::FontId;
        use egui::TextStyle::*;

        // Redefine text_styles
        let text_styles: BTreeMap<_, _> = [
            // Defaults...
            (Heading, FontId::new(14.0, Proportional)),
            (Body, FontId::new(12.5, Proportional)),
            (Button, FontId::new(12.5, Proportional)),
            (Small, FontId::new(9.0, Proportional)),
            // Edits: make monospace a bit larger (normally 12.0)
            (Monospace, FontId::new(13.0, egui::FontFamily::Monospace)),
        ]
        .into();

        // Mutate global styles with new text styles
        ctx.all_styles_mut(move |style| style.text_styles = text_styles.clone());
    }

    ctx.style_mut(|style| {
        style.visuals.window_shadow = egui::Shadow::NONE;
    });
}

pub fn update_egui_inspector_ui(
    world: &mut World,
    mut show_flat: Local<bool>,
) {
    use bevy_inspector_egui::bevy_inspector::*;

    // Find the current context using the world's querying.
    // We'll need to clone this to avoid double-borrow of `world` below.
    let Ok(egui_context) = world
        .query_filtered::<&mut EguiContext, (With<Camera3d>, With<ViewerCamera>)>()
        .single_mut(world) else { return };

    egui::Window::new("Inspector")
        .default_pos(egui::Pos2::new(50.0, 100.0))
        .show(egui_context.clone().get_mut(), |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                ui.checkbox(&mut show_flat, "Show Flat");
                const FILTER_ID: &str = "my_world_entities_filter";
                if *show_flat == false {
                    let filter: Filter<Without<Observer>> =
                        Filter::from_ui_fuzzy(ui, egui::Id::new(FILTER_ID));
                    egui::CollapsingHeader::new("Entities")
                        .default_open(false)
                        .show(ui, |ui| {
                            ui_for_entities_filtered(world, ui, false, &filter);
                        });
                } else {
                    let filter: Filter<(Without<Observer>, Without<ChildOf>)> =
                        Filter::from_ui_fuzzy(ui, egui::Id::new(FILTER_ID));
                    egui::CollapsingHeader::new("Entities")
                        .default_open(false)
                        .show(ui, |ui| {
                            ui_for_entities_filtered(world, ui, true, &filter);
                        });
                }

                egui::CollapsingHeader::new("Resources").show(ui, |ui| {
                    ui_for_resources(world, ui);
                });

                egui::CollapsingHeader::new("Assets").show(ui, |ui| {
                    ui_for_all_assets(world, ui);
                });

                // egui::CollapsingHeader::new("Audio Listeners").show(ui, |ui| {
                //     ui_for_entities_filtered::<Filter<With<AudioCameraListener>>>(world, ui, false, &Filter::all());
                // });
                // egui::CollapsingHeader::new("Audio Cues").show(ui, |ui| {
                //     ui_for_entities_filtered::<Filter<With<AudioCue>>>(world, ui, false, &Filter::all());
                // });
                // egui::CollapsingHeader::new("Audio Players").show(ui, |ui| {
                //     ui_for_entities_filtered::<Filter<With<AudioPlayState>>>(world, ui, false, &Filter::all());
                // });

            });
        });
}

// Re-exports.

pub fn debug_gui_wants_pointer_input(r: Option<Res<EguiWantsInput>>) -> bool {
    if let Some(r) = r {
        egui_wants_any_pointer_input(r)
    } else {
        false
    }
}
pub fn debug_gui_wants_keyboard_input(r: Option<Res<EguiWantsInput>>) -> bool {
    if let Some(r) = r {
        egui_wants_any_keyboard_input(r)
    } else {
        false
    }
}
pub fn debug_gui_wants_direct_input(r: Option<Res<EguiWantsInput>>) -> bool {
    if let Some(r) = r {
        r.is_pointer_over_area() || r.is_popup_open()
    } else {
        false
    }
}
