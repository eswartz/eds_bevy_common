use std::collections::BTreeMap;

use bevy::prelude::*;
use bevy_egui::{EguiContext, EguiContexts, EguiPrimaryContextPass, PrimaryEguiContext};

use crate::*;

use super::{gui::GuiState, states_sets::OverlayState};

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(
                PreUpdate,
                (setup_egui_style, ensure_egui_context)
                    .chain()
                    .run_if(egui_not_initialized)
                    .run_if(in_state(GameplayState::Playing)),
            )

            .add_systems(
                EguiPrimaryContextPass,
                (
                    update_egui_inspector_ui.run_if(|gui_state: Res<GuiState>, ovl_state: Res<State<OverlayState>>|
                        gui_state.show_inspector_always ||
                        (gui_state.show_inspector && ovl_state.get().is_debug())),
                ),
            )
        ;
    }
}

pub fn egui_not_initialized(camera_q: Query<Entity, (With<Camera3d>, With<ViewerCamera>, With<PrimaryEguiContext>)>) -> bool
{
    camera_q.single().is_err()
}

pub fn ensure_egui_context(mut commands: Commands, camera_q: Query<Entity, (With<Camera3d>, With<ViewerCamera>, Without<PrimaryEguiContext>)>)      {
    for camera_ent in camera_q.iter() {
        commands.entity(camera_ent).insert(
            PrimaryEguiContext,
        );
    }
}

pub fn setup_egui_style(mut contexts: EguiContexts) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

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
                            // ui_for_entities(world, ui);
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
