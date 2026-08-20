use std::{collections::BTreeMap, io::Write, sync::LazyLock};

use bevy::{ecs::{query::QueryFilter, system::SystemParam}, prelude::*};
use bevy_egui::{EguiContext, EguiContexts, EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass, PrimaryEguiContext, helpers::*, input::{EguiWantsInput, egui_wants_any_keyboard_input, egui_wants_any_pointer_input}};
use bevy_egui::egui;
use bevy_inspector_egui::{DefaultInspectorConfigPlugin};
use fuzzy_matcher::skim::SkimMatcherV2;

use crate::{prelude::*, world_load_save::{fetch_saveable_entities, save_world_state}};

use super::gui::GuiState;

/// This uses bevy-inspector-egui. If you don't add
/// `EguiPlugin` and/or `DefaultInspectorConfigPlugin` yourself,
/// this plugin will do so with default settings.
///
pub struct DebugPlugin;

#[derive(Resource, Reflect)]
#[reflect(Resource, Default)]
#[type_path = "game"]
pub struct DebugLayout {
    pub dev_settings_rect: Rect,
    pub inspector_rect: Rect,
}

impl Default for DebugLayout {
    fn default() -> Self {
        Self {
            inspector_rect: Rect::from_center_size(
                Vec2::new(160.0, 400.0),
                Vec2::new(300.0, 400.0),
            ),
            dev_settings_rect: Rect::from_center_size(
                Vec2::new(1000.0, 160.0),
                Vec2::new(300.0, 300.0),
            ),
        }
    }
}

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
            .init_resource::<DebugLayout>()

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
                .run_if(is_debug_ui_inspector_visible),
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
            commands.entity(camera_ent).try_insert(
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

    ctx.global_style_mut(|style| {
        style.visuals.window_shadow = egui::Shadow::NONE;
    });
}

pub fn update_egui_settings_ui(
    mut contexts: EguiContexts,
    mut in_state: ResMut<GuiState>,
    debug_layout: Res<DebugLayout>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

    // Work on clones to avoid firing mutable change listeners
    // (as they will simply by virtue of passing their `mut`s into egui).
    let mut state = in_state.clone();

    let rect = rect_into_egui_rect(debug_layout.dev_settings_rect);

    let mut window = egui::Window::new("Dev Settings")
        .default_open(true)
        .default_rect(rect)
        .resizable(true);

    if debug_layout.is_changed() {
        window = window.current_pos(rect.min);
    }

    window
        .show(ctx, |ui| {
            egui::CollapsingHeader::new("UI")
                .default_open(true)

                .show(ui, |ui| {
                ui.checkbox(&mut state.show_stats, "Always Show Stats")
                    .on_hover_text("Show stats overlay even during play.");
                ui.checkbox(&mut state.show_inspector, "Show World Inspector")
                    .on_hover_text("Show World Inspector.");
                ui.add_enabled_ui(state.show_inspector, |ui|
                    ui.indent("inspector", |ui| {
                        ui.checkbox(&mut state.show_inspector_always, "Always Show")
                        .on_hover_text("Show even in gameplay.");
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
}

/// egui filter
pub(crate) const ENTITY_FILTER_ID: &str = "my_inspector_entity_filter";
pub(crate) const SELECTED_ENTITY_FILTER_ID: &str = "selected_inspector_entity_filter";

// pattern for an Entity filter
static ENT_RX: LazyLock<regex::Regex> = LazyLock::new(||
    regex::Regex::new("^([0-9]+)v([0-9]+)$").expect("valid regex")
);

pub fn update_egui_inspector_ui(
    world: &mut World,
    mut pin_selection: Local<bool>,
    mut show_tree: Local<bool>,
    mut show_all: Local<bool>,
    mut last_selected: Local<Vec<Entity>>,
) {
    use bevy_inspector_egui::bevy_inspector::*;
    use egui::*;

    // Select a new entry if it was not selected before.
    let new_selected_opt = {
        let selected = world
            .query_filtered::<Entity, With<Highlighted>>()
            .iter(world)
            .collect::<Vec<_>>();
        let mut new = None::<Entity>;
        if selected != *last_selected {
            for ent in selected.iter() {
                if !last_selected.contains(ent) {
                    new = Some(*ent);
                    *last_selected = selected;
                    break;
                }
            }
        }
        new
    };

    let debug_layout = world.resource_ref::<DebugLayout>();
    let rect = rect_into_egui_rect(debug_layout.inspector_rect);

    let mut window = egui::Window::new("World Inspector")
        .default_open(true)
        .default_rect(rect)
        .resizable(true);

    if debug_layout.is_changed() {
        window = window.current_pos(rect.min);
    }

    let mut save_state = false;

    // Find the current context using the world's querying.
    // We'll need to clone this to avoid double-borrow of `world` below.
    // (Don't use SystemState or World::with_scope here to avoid
    // stealing them from the bevy-egui-inspector plugin!)
    let Ok(egui_context) = world
        .query_filtered::<&mut EguiContext, (With<Camera3d>, With<ViewerCamera>)>()
        .single_mut(world) else { return };

    window.show(
        egui_context.clone().get_mut(),
        |ui| {
            if let Some(selected) = new_selected_opt {
                // Set up selection filter if new.

                // Copied from BIE.
                let id = egui::Id::new(ENTITY_FILTER_ID).with("word");

                let (filter, last_auto_filter) = ui.memory_mut(|mem| {
                    let filter = mem.data.get_persisted_mut_or_default::<String>(
                        id).clone();
                    let last_filter = mem.data.get_persisted_mut_or_default::<String>(
                        egui::Id::new(SELECTED_ENTITY_FILTER_ID)).clone();
                    (filter, last_filter)
                });

                let new_auto_filter = format!("{selected}");
                let is_new_selection = last_auto_filter != new_auto_filter;
                let is_important = *pin_selection && !filter.is_empty();
                let last_was_auto_or_empty = filter.is_empty() || ENT_RX.is_match(&filter);
                if !is_important && is_new_selection && last_was_auto_or_empty {
                    ui.memory_mut(|mem| {
                        let filter: &mut String = mem.data.get_persisted_mut_or_default(id);
                        *filter = new_auto_filter.clone();

                        *mem.data.get_persisted_mut_or_default::<String>(
                            egui::Id::new(SELECTED_ENTITY_FILTER_ID)) = new_auto_filter;
                    });
                }
            }

            ui.scope(|ui| {
                ui.style_mut().override_text_style = Some(TextStyle::Small);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Entities:").strong());
                    ui.checkbox(&mut pin_selection, RichText::new("Pin"));
                    ui.checkbox(&mut show_tree, RichText::new("As Tree"));
                    ui.checkbox(&mut show_all, RichText::new("All"))
                        .on_hover_text("When unset, hides entities without Names, which are usually behind-the-scenes entities.");
                    ui.separator();
                    if ui.button("Save").clicked() {
                        save_state = true;
                    }
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

    if save_state {
        let ents = fetch_saveable_entities(world, true);
        let _ = match save_world_state(world, ents) {
            Ok(text) => {
                let _ = match std::fs::File::create("save_world.scn.ron") {
                    Ok(mut file) => file.write_all(text.as_bytes()),
                    Err(e) => Err(e),
                }
                .map_err(|e| error!("failed to save: {e}"));
            }
            Err(e) => error!("failed to save: {e}")
        };
    }
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
            matcher.fuzzy_match(name, filter).is_some()
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
    resources.sort_by_key(|name| *name);
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
