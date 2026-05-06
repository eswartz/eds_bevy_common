use std::{any::TypeId, collections::BTreeMap, path::Path};

use bevy::{ecs::{component::ComponentId, system::SystemParam, world::CommandQueue}, prelude::*, reflect::TypeRegistry};
use bevy_egui::{EguiContext, EguiContexts, EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass, PrimaryEguiContext, input::{EguiWantsInput, egui_wants_any_keyboard_input, egui_wants_any_pointer_input}};
use bevy_inspector_egui::{DefaultInspectorConfigPlugin, bevy_inspector, reflect_inspector::InspectorUi, restricted_world_view::{ReflectBorrow, RestrictedWorldView}};
use fuzzy_matcher::skim::SkimMatcherV2;

use crate::*;

use super::gui::GuiState;

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

pub fn update_egui_inspector_ui(
    world: &mut World,
    mut show_tree: Local<bool>,
    mut show_observers: Local<bool>,
) {
    use bevy_inspector_egui::bevy_inspector::*;

    // Find the current context using the world's querying.
    // We'll need to clone this to avoid double-borrow of `world` below.
    // (Don't use SystemState or World::with_scope here to avoid
    // stealing them from the bevy-egui-inspector plugin!)
    let Ok(egui_context) = world
        .query_filtered::<&mut EguiContext, (With<Camera3d>, With<ViewerCamera>)>()
        .single_mut(world) else { return };

    const FILTER_ID: &str = "my_inspector_entity_filter";

    egui::Window::new("Inspector")
        .default_pos(egui::Pos2::new(5.0, 150.0))
        .default_size(egui::Vec2::new(250.0, 300.0))
        // .hscroll(true)
        // .vscroll(true)
        .show(egui_context.clone().get_mut(), |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                ui.checkbox(&mut show_tree, "Show As Tree");
                ui.checkbox(&mut show_observers, "Show Observers");

                let mut entities_with_filter = |ui: &mut egui::Ui| {
                    match (*show_tree, *show_observers) {
                        (false, false) => {
                            let filter: Filter<Without<Observer>> =
                                Filter::from_ui_fuzzy(ui, egui::Id::new(FILTER_ID));
                            ui_for_entities_filtered(world, ui, true, &filter);
                        }
                        (false, true) => {
                            let filter: Filter<()> =
                                Filter::from_ui_fuzzy(ui, egui::Id::new(FILTER_ID));
                            ui_for_entities_filtered(world, ui, true, &filter);
                        }
                        (true, false) => {
                            let filter: Filter<(Without<ChildOf>, Without<Observer>)> =
                                Filter::from_ui_fuzzy(ui, egui::Id::new(FILTER_ID));
                            ui_for_entities_filtered(world, ui, true, &filter);
                        }
                        (true, true) => {
                            let filter: Filter<Without<ChildOf>> =
                                Filter::from_ui_fuzzy(ui, egui::Id::new(FILTER_ID));
                            ui_for_entities_filtered(world, ui, true, &filter);
                        }
                    }
                };

                egui::CollapsingHeader::new("Entities")
                    .default_open(false)
                    .show(ui, |ui| {
                        entities_with_filter(ui);
                    });

                egui::CollapsingHeader::new("Resources").show(ui, |ui| {
                    const FILTER_ID: &str = "my_inspector_resource_filter";
                    let filter: Filter<()> = Filter::from_ui_fuzzy(ui, egui::Id::new(FILTER_ID));
                    ui_for_filtered_resources(world, ui, filter);
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

// // Our versions of bevy-egui-inspector features, differently opinionated.

// use bevy_inspector::*;
// use bevy_inspector_egui::bevy_inspector::guess_entity_name;

// /// Display all entities matching the given [`EntityFilter`].
// ///
// /// You can use the [`Filter`] type to specify both a static filter as a generic parameter (default is `Without<Parent>`),
// /// and a word to match. [`Filter::from_ui`] will display a search box and fuzzy filter checkbox.
// pub fn ui_for_entities_filtered<F>(
//     world: &mut World,
//     ui: &mut egui::Ui,
//     with_children: bool,
//     filter: &F,
// ) where
//     F: bevy_inspector::EntityFilter,
// {

//     let type_registry = world.resource::<AppTypeRegistry>().0.clone();
//     let type_registry = type_registry.read();

//     let mut root_entities = world.query_filtered::<Entity, F::StaticFilter>();
//     let mut entities = root_entities.iter(world).collect::<Vec<_>>();

//     filter.filter_entities(world, &mut entities);

//     entities.sort();

//     let id = egui::Id::new("world ui");
//     for entity in entities {
//         let id = id.with(entity);

//         let entity_name = bevy_inspector::guess_entity_name(world, entity);
//         egui::CollapsingHeader::new(&entity_name)
//             .id_salt(id)
//             .show(ui, |ui| {
//                 if with_children {
//                     ui_for_entity_with_children_inner(
//                         world,
//                         entity,
//                         ui,
//                         id,
//                         &type_registry,
//                         filter,
//                     );
//                 } else {
//                     let mut queue = CommandQueue::default();
//                     ui_for_entity_components(
//                         &mut world.into(),
//                         Some(&mut queue),
//                         entity,
//                         ui,
//                         id,
//                         &type_registry,
//                     );
//                     queue.apply(world);
//                 }
//             });
//     }
// }

// fn ui_for_entity_with_children_inner<F>(
//     world: &mut World,
//     entity: Entity,
//     ui: &mut egui::Ui,
//     id: egui::Id,
//     type_registry: &TypeRegistry,
//     filter: &F,
// ) where
//     F: EntityFilter,
// {
//     let mut queue = CommandQueue::default();
//     ui_for_entity_components(
//         &mut world.into(),
//         Some(&mut queue),
//         entity,
//         ui,
//         id,
//         type_registry,
//     );

//     let children = world
//         .get::<Children>(entity)
//         .map(|children| children.iter().collect::<Vec<_>>());
//     if let Some(mut children) = children
//         && !children.is_empty()
//     {
//         filter.filter_entities(world, &mut children);
//         ui.label("Children");
//         for child in children {
//             let id = id.with(child);

//             let child_entity_name = guess_entity_name(world, child);
//             egui::CollapsingHeader::new(&child_entity_name)
//                 .id_salt(id)
//                 .show(ui, |ui| {
//                     ui.label(&child_entity_name);

//                     ui_for_entity_with_children_inner(world, child, ui, id, type_registry, filter);
//                 });
//         }
//     }

//     queue.apply(world);
// }

// /// Display the components of the given entity
// pub fn ui_for_entity_components(
//     world: &mut RestrictedWorldView<'_>,
//     mut queue: Option<&mut CommandQueue>,
//     entity: Entity,
//     ui: &mut egui::Ui,
//     id: egui::Id,
//     type_registry: &TypeRegistry,
// ) {
//     let Ok(components) = components_of_entity(world, entity) else {
//         errors::nonexistent_entity(ui, entity);
//         return;
//     };

//     for (name, component_id, component_type_id, size) in components {
//         let id = id.with(component_id);

//         let header = egui::CollapsingHeader::new(&name).id_salt(id);

//         let Some(component_type_id) = component_type_id else {
//             header.show(ui, |ui| errors::missing_type_id(ui, &name));
//             continue;
//         };

//         // #[cfg(feature = "documentation")]
//         // let type_docs = type_registry
//         //     .get_type_info(component_type_id)
//         //     .and_then(|info| info.docs());

//         if size == 0 {
//             ui.indent(id, |ui| {
//                 let _response = ui.label(&name);
//                 // #[cfg(feature = "documentation")]
//                 // crate::egui_utils::show_docs(_response, type_docs);
//             });
//             continue;
//         }

//         // create a context with access to the world except for the currently viewed component
//         let (mut component_view, world) = world.split_off_component((entity, component_type_id));
//         let mut cx = bevy_inspector_egui::reflect_inspector::Context {
//             world: Some(world),
//             #[allow(clippy::needless_option_as_deref)]
//             queue: queue.as_deref_mut(),
//         };

//         let value = match component_view.get_entity_component_reflect(
//             entity,
//             component_type_id,
//             type_registry,
//         ) {
//             Ok(value) => value,
//             Err(e) => {
//                 ui.indent(id, |ui| {
//                     let response = ui.label(egui::RichText::new(&name).underline());
//                     response.on_hover_ui(|ui| errors::no_access(e, ui, &name));
//                 });
//                 continue;
//             }
//         };

//         let changed_by = match &value {
//             ReflectBorrow::Mutable(val) => val.changed_by().into_option(),
//             ReflectBorrow::Immutable(_) => None,
//         };

//         // #[cfg(feature = "highlight_changes")]
//         // if value.is_changed() {
//         //     set_highlight_style(ui);
//         // }

//         let _response = header.show(ui, |ui| {
//             ui.reset_style();

//             let mut env = InspectorUi::for_bevy(type_registry, &mut cx);
//             let id = id.with(component_id);
//             let options = &();

//             match value {
//                 ReflectBorrow::Mutable(mut value) => {
//                     let changed = env.ui_for_reflect_with_options(
//                         value.bypass_change_detection().as_partial_reflect_mut(),
//                         ui,
//                         id,
//                         options,
//                     );

//                     if changed {
//                         value.set_changed();
//                     }
//                 }
//                 ReflectBorrow::Immutable(value) => env.ui_for_reflect_readonly_with_options(
//                     value.as_partial_reflect(),
//                     ui,
//                     id,
//                     options,
//                 ),
//             };
//         });

//         let response = _response.header_response;

//         if let Some(location) = changed_by {
//             response.context_menu(|ui| {
//                 ui.label("Last change:");
//                 let path = Path::new(location.file());
//                 // let pretty = utils::trim_cargo_registry_path(path);

//                 ui.label(format!(
//                     "{}:{}:{}",
//                     // pretty.as_deref().unwrap_or(path).display(),
//                     path.display(),
//                     location.line(),
//                     location.column()
//                 ));
//                 // if ui
//                 //     .button(format!(
//                 //         "{}:{}:{}",
//                 //         pretty.as_deref().unwrap_or(path).display(),
//                 //         location.line(),
//                 //         location.column()
//                 //     ))
//                 //     .clicked()
//                 // {
//                 //     if let Err(e) = utils::open_file_at(location) {
//                 //         bevy_log::error!("Failed to open last change location: {}", e);
//                 //     } else {
//                 //         bevy_log::info!("Successfully opened {location}");
//                 //     }
//                 // }
//             });
//         }

//         // #[cfg(feature = "documentation")]
//         // crate::egui_utils::show_docs(response, type_docs);

//         ui.reset_style();
//     }
// }

// pub fn components_of_entity(
//     world: &mut RestrictedWorldView<'_>,
//     entity: Entity,
// ) -> Result<Vec<(String, ComponentId, Option<TypeId>, usize)>> {
//     let entity_ref = world.world().get_entity(entity)?;

//     let archetype = entity_ref.archetype();
//     let mut components: Vec<_> = archetype
//         .components()
//         .iter()
//         .map(|component_id| {
//             let info = world.world().components().get_info(*component_id).unwrap();
//             let name = utils::pretty_type_name_str(&info.name().to_string());

//             (name, *component_id, info.type_id(), info.layout().size())
//         })
//         .collect();
//     components.sort_by(|(name_a, ..), (name_b, ..)| name_a.cmp(name_b));
//     Ok(components)
// }

// pub mod utils {
// use super::*;

// use std::{
//     panic::Location,
//     path::{Path, PathBuf},
// };

// use bevy_ecs::error::Result;

// pub fn pretty_type_name<T>() -> String {
//     format!("{:?}", disqualified::ShortName::of::<T>())
// }
// pub fn pretty_type_name_str(val: &str) -> String {
//     format!("{:?}", disqualified::ShortName(val))
// }

// pub mod guess_entity_name {
//     use bevy_ecs::prelude::Name;
//     use bevy_ecs::{archetype::Archetype, prelude::*, world::unsafe_world_cell::UnsafeWorldCell};

//     use crate::restricted_world_view::RestrictedWorldView;

//     /// Guesses an appropriate entity name like `Light (6)` or falls back to `Entity (8)`
//     pub fn guess_entity_name(world: &World, entity: Entity) -> String {
//         match world.get_entity(entity) {
//             Ok(entity_ref) => {
//                 if let Some(name) = entity_ref.get::<Name>() {
//                     return format!("{} ({})", name.as_str(), entity);
//                 }

//                 guess_entity_name_inner(
//                     world.as_unsafe_world_cell_readonly(),
//                     entity,
//                     entity_ref.archetype(),
//                 )
//             }
//             Err(_) => format!("Entity {} (inexistent)", entity.index()),
//         }
//     }

//     pub(crate) fn guess_entity_name_restricted(
//         world: &mut RestrictedWorldView<'_>,
//         entity: Entity,
//     ) -> String {
//         match world.world().get_entity(entity) {
//             Ok(cell) => {
//                 if world.allows_access_to_component((entity, std::any::TypeId::of::<Name>())) {
//                     // SAFETY: we have access and don't keep reference
//                     if let Some(name) = unsafe { cell.get::<Name>() } {
//                         return format!("{} ({})", name.as_str(), entity);
//                     }
//                 }
//                 guess_entity_name_inner(world.world(), entity, cell.archetype())
//             }
//             Err(_) => format!("Entity {} (inexistent)", entity.index()),
//         }
//     }

//     fn guess_entity_name_inner(
//         world: UnsafeWorldCell<'_>,
//         entity: Entity,
//         archetype: &Archetype,
//     ) -> String {
//         #[rustfmt::skip]
//         let associations = &[
//             ("bevy_window::window::PrimaryWindow", "Primary Window"),
//             ("bevy_camera::components::Camera3d", "Camera3d"),
//             ("bevy_camera::components::Camera2d", "Camera2d"),
//             ("bevy_light::point_light::PointLight", "PointLight"),
//             ("bevy_light::directional_light::DirectionalLight", "DirectionalLight"),
//             ("bevy_text::text::Text", "Text"),
//             ("bevy_ui::ui_node::Node", "Node"),
//             ("bevy_pbr::mesh_material::MeshMaterial3d<bevy_pbr::pbr_material::StandardMaterial>", "Pbr Mesh"),
//             ("bevy_window::window::Window", "Window"),
//             ("bevy_ecs::observer::distributed_storage::Observer", "Observer"),
//             ("bevy_window::monitor::Monitor", "Monitor"),
//             ("bevy_picking::pointer::PointerId", "Pointer"),
//         ];

//         let type_names = archetype.components().iter().filter_map(|id| {
//             let name = world.components().get_info(*id)?.name();
//             Some(name)
//         });

//         for component_type in type_names {
//             if let Some(name) = associations.iter().find_map(|&(name, matches)| {
//                 (component_type.to_string() == name).then_some(matches)
//             }) {
//                 return format!("{name} ({entity})");
//             }
//         }

//         format!("Entity ({entity})")
//     }
// }

// pub fn trim_cargo_registry_path(path: &Path) -> Option<PathBuf> {
//     let mut components = path.components().peekable();
//     while let Some(c) = components.next() {
//         if c.as_os_str() == ".cargo" {
//             if components.next()?.as_os_str() != "registry" {
//                 return None;
//             }
//             if components.next()?.as_os_str() != "src" {
//                 return None;
//             }
//             components.next()?;
//             return Some(components.collect());
//         }
//     }

//     None
// }

// pub fn open_file_at(location: &Location<'_>) -> Result<()> {
//     let path = Path::new(location.file());

//     // try editors supporting opening file:col first (in order of most likely to be explicitly installed)
//     if std::process::Command::new("zeditor")
//         .arg(location.to_string())
//         .spawn()
//         .is_ok()
//     {
//         return Ok(());
//     }

//     if std::process::Command::new("code")
//         .arg("--goto")
//         .arg(location.to_string())
//         .spawn()
//         .is_ok()
//     {
//         return Ok(());
//     }

//     opener::open(path)?;

//     Ok(())
// }

// }   // mod utils






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
