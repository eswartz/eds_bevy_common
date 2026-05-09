use std::collections::BTreeMap;

use bevy::{ecs::{query::QueryFilter, system::SystemParam}, prelude::*};
use bevy_egui::{EguiContext, EguiContexts, EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass, PrimaryEguiContext, input::{EguiWantsInput, egui_wants_any_keyboard_input, egui_wants_any_pointer_input}};
use bevy_inspector_egui::{DefaultInspectorConfigPlugin};
use fuzzy_matcher::skim::SkimMatcherV2;

use crate::*;

use super::gui::GuiState;

/// This uses bevy-inspector-egui. If you don't add
/// `EguiPlugin` and/or `DefaultInspectorConfigPlugin` yourself,
/// this plugin will do so with default settings.
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
                update_egui_inspector_ui
                .run_if(is_debug_ui_inspector_active),
            )
            .add_systems(
                EguiPrimaryContextPass,
                update_egui_settings_ui
                .run_if(is_debug_ui_enabled),
            )
        ;
    }
}


/// Which 3D camera hosts egui UI?
#[derive(Resource, Reflect, Default, PartialEq, Debug)]
#[reflect(Resource)]
#[type_path = "game"]
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

/// egui filter
pub(crate) const ENTITY_FILTER_ID: &str = "my_inspector_entity_filter";
pub(crate) const SELECTED_ENTITY_FILTER_ID: &str = "selected_inspector_entity_filter";

pub fn update_egui_inspector_ui(
    world: &mut World,
    mut show_tree: Local<bool>,
    mut show_all: Local<bool>,
) {
    use bevy_inspector_egui::bevy_inspector::*;
    use egui::*;

    let now_selected_opt = world.query_filtered::<
        Entity, (With<CrosshairTargetable>, Added<Highlighted>)
    >().iter(world).next();

    // Find the current context using the world's querying.
    // We'll need to clone this to avoid double-borrow of `world` below.
    // (Don't use SystemState or World::with_scope here to avoid
    // stealing them from the bevy-egui-inspector plugin!)
    let Ok(egui_context) = world
        .query_filtered::<&mut EguiContext, (With<Camera3d>, With<ViewerCamera>)>()
        .single_mut(world) else { return };

    Window::new("Inspector")
        .default_pos(Pos2::new(5.0, 150.0))
        .default_size(Vec2::new(250.0, 300.0))
        .show(egui_context.clone().get_mut(), |ui| {

            if let Some(selected) = now_selected_opt {
                // Set up selection filter if new.

                // Copied from BIE.
                let id = egui::Id::new(ENTITY_FILTER_ID).with("word");

                let (filter, last_filter) = ui.memory_mut(|mem| {
                    let filter = mem.data.get_persisted_mut_or_default::<String>(
                        id).clone();
                    let last_filter = mem.data.get_persisted_mut_or_default::<String>(
                        egui::Id::new(SELECTED_ENTITY_FILTER_ID)).clone();
                    (filter, last_filter)
                });

                let new_filter = format!("{selected}");
                if filter.is_empty() || last_filter != new_filter {
                    ui.memory_mut(|mem| {
                        let filter: &mut String = mem.data.get_persisted_mut_or_default(id);
                        *filter = new_filter.clone();

                        *mem.data.get_persisted_mut_or_default::<String>(
                            egui::Id::new(SELECTED_ENTITY_FILTER_ID)) = new_filter;
                    });
                }
            }

            ui.scope(|ui| {
                ui.style_mut().override_text_style = Some(TextStyle::Small);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Entities:").strong());
                    ui.checkbox(&mut show_tree, RichText::new("As Tree"));
                    ui.checkbox(&mut show_all, RichText::new("All"))
                        .on_hover_text("When unset, hides entities without Names, which are usually behind-the-scenes entities.");
                });
            });

            ScrollArea::both().show(ui, |ui| {

            let mut entities_with_filter = |ui: &mut Ui| {
                #[derive(QueryFilter)]
                struct NotAllFilter {
                    is_named: With<Name>,
                }

                type Roots = Without<ChildOf>;

                // We share the FILTER_ID for each combination of button states,
                // so the text entry is retained when switching modes.
                // (Do not try to reimplement ui_for_entities_filtered again.)
                let id = Id::new(ENTITY_FILTER_ID);

                let show_noisy = ! *show_all;
                let show_tree = *show_tree;
                match (show_tree, show_noisy) {
                    (false, false) => {
                        let filter: Filter<()> = Filter::from_ui_fuzzy(ui, id);
                        ui_for_entities_filtered(world, ui, true, &filter);
                    }
                    (false, true) => {
                        let filter: Filter<NotAllFilter> = Filter::from_ui_fuzzy(ui, id);
                        ui_for_entities_filtered(world, ui, true, &filter);
                    }
                    (true, false) => {
                        // As parent-child tree and all entities, each a root.
                        let filter: Filter<Roots> = Filter::from_ui_fuzzy(ui, id);
                        ui_for_entities_filtered(world, ui, true, &filter);
                    }
                    (true, true) => {
                        // As parent-child tree and , each a root.
                        let filter: Filter<(Roots, NotAllFilter)> = Filter::from_ui_fuzzy(ui, id);
                        ui_for_entities_filtered(world, ui, true, &filter);
                    }
                }
            };

            CollapsingHeader::new("Entities")
                .default_open(false)
                .show(ui, |ui| {
                    entities_with_filter(ui);
                });

            CollapsingHeader::new("Resources").show(ui, |ui| {
                const FILTER_ID: &str = "my_inspector_resource_filter";
                let filter: Filter<()> = Filter::from_ui_fuzzy(ui, Id::new(FILTER_ID));
                ui_for_filtered_resources(world, ui, filter);
            });

            CollapsingHeader::new("Assets").show(ui, |ui| {
                ui_for_all_assets(world, ui);
            });

            // CollapsingHeader::new("Audio Listeners").show(ui, |ui| {
            //     ui_for_entities_filtered::<Filter<With<AudioCameraListener>>>(world, ui, false, &Filter::all());
            // });
            // CollapsingHeader::new("Audio Cues").show(ui, |ui| {
            //     ui_for_entities_filtered::<Filter<With<AudioCue>>>(world, ui, false, &Filter::all());
            // });
            // CollapsingHeader::new("Audio Players").show(ui, |ui| {
            //     ui_for_entities_filtered::<Filter<With<AudioPlayState>>>(world, ui, false, &Filter::all());
            // });

        });
    });
}

fn name_satisfies_filter(
    name: &str,
    filter: &str,
    is_fuzzy: bool,
) -> bool {
    use fuzzy_matcher::FuzzyMatcher;
    if filter.is_empty() {
        true
    } else {
        if is_fuzzy {
            let matcher = SkimMatcherV2::default();
            matcher.fuzzy_match(&name, filter).is_some()
        } else {
            name.to_lowercase().contains(filter)
        }
    }
}

pub fn ui_for_filtered_resources(
    world: &mut World,
    ui: &mut egui::Ui,
    filter: bevy_inspector_egui::bevy_inspector::Filter<()>,
) {
    use bevy_inspector_egui::bevy_inspector::*;

    let type_registry = world.resource::<AppTypeRegistry>().0.clone();
    let type_registry = type_registry.read();

    let mut resources: Vec<_> = type_registry
        .iter()
        .filter(|registration| {
            registration.data::<ReflectResource>().is_some() &&
            name_satisfies_filter(
                registration.type_info().type_path(),
                &filter.word,
                filter.is_fuzzy)
        })
        .map(|registration| {
            (
                registration.type_info().type_path_table().short_path(),
                registration.type_id(),
            )
        })
        .collect();
    resources.sort_by(|(name_a, ..), (name_b, ..)| name_a.cmp(name_b));
    for (name, type_id) in resources {
        ui.collapsing(name, |ui| {
            by_type_id::ui_for_resource(world, type_id, ui, name, &type_registry);
        });
    }
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
pub fn debug_gui_wants_input(r: Option<Res<EguiWantsInput>>) -> bool {
    if let Some(r) = r {
        r.is_popup_open() || r.wants_any_keyboard_input() || r.wants_any_pointer_input()
    } else {
        false
    }
}

pub fn update_egui_settings_ui(
    mut contexts: EguiContexts,
    mut in_state: ResMut<GuiState>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };
    // Work on clones to avoid firing mutable change listeners
    let mut state = in_state.clone();

    egui::Window::new("Settings")
        .default_open(true)
        .default_rect(egui::Rect::from_min_size(
            ctx.available_rect().right_bottom() - egui::Vec2::new(400., 700.),
            egui::Vec2::new(400., 600.))
        )
        .resizable(true)
        .show(ctx, |ui| {
            egui::CollapsingHeader::new("UI")
                .default_open(true)
                .show(ui, |ui| {
                ui.checkbox(&mut state.show_status, "Always Show Player Status")
                    .on_hover_text("Show player status (position/movement) during gameplay");
                ui.checkbox(&mut state.show_fps, "Show FPS Always")
                    .on_hover_text("Show FPS overlay, even outside the control UI.");
                // ui.checkbox(&mut state.show_skybox, "Show Skybox")
                //     .on_hover_text("Show skybox.");
                ui.checkbox(&mut state.show_inspector, "Show Inspector")
                    .on_hover_text("Show Bevy inspector.");
                ui.add_enabled_ui(state.show_inspector, |ui|
                    ui.indent("inspector", |ui| {
                        ui.checkbox(&mut state.show_inspector_always, "Always")
                        .on_hover_text("Always show Bevy inspector.");
                    })
                );
                ui.checkbox(&mut state.show_physics_gizmos, "Show Physics Gizmos")
                    .on_hover_text("Show Avian physics gizmo overlays.");

            });

            // if let Ok((player, cheats)) = player_cheat_q.single_mut() {
            //     let mut enabled = cheats.has(Cheats::Noclip);
            //     if ui.checkbox(&mut enabled, "Enable Noclip")
            //         .on_hover_text("Toggle collision bounds for player.")
            //         .changed() {

            //         commands.write_message(PlayerRequestMessage{
            //             request: PlayerRequest::SetCheat(Cheats::Noclip, enabled),
            //             player,
            //         });
            //     }
            // }

        }
    );

    in_state.set_if_neq(state);
    // audio.audio_ctrl.set_if_neq(audio_ctrl);
    // synth.set_if_neq(synth_ctrl);
}
