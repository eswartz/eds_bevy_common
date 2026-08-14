use std::{any::TypeId, sync::{Arc, atomic::AtomicBool}};
use bevy::{ecs::entity::EntityHashSet, prelude::*};
use crate::prelude::*;

pub struct WorldLoadSavePlugin;

impl Plugin for WorldLoadSavePlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<SavedResources>()
            .init_resource::<SavedComponents>()
            .register_type_data::<Arc<AtomicBool>, ReflectSerialize>()
            .register_type_data::<Arc<AtomicBool>, ReflectDeserialize>()
        ;
    }
}

/// Track the list of resources we want to save, using `TheResource::type_id()`.
#[derive(Resource, Clone)]
pub struct SavedResources(pub Vec<TypeId>);

impl Default for SavedResources {
    fn default() -> Self {
        let type_ids = vec![
            TypeId::of::<PlayerCameraSettings>(),
            TypeId::of::<FovZoomState>(),
            TypeId::of::<PlayerCameraViews>(),
            TypeId::of::<PlayerInputSettings>(),
            TypeId::of::<PlayerControllerSettings>(),
            TypeId::of::<PlayerMode>(),
            TypeId::of::<BaseVhacdParameters>(),
            TypeId::of::<CrosshairTargets>(),
            TypeId::of::<DeathboxFlags>(),
            TypeId::of::<DebugLayout>(),
            TypeId::of::<DebugEguiCamera>(),
            TypeId::of::<FlashlightOffset>(),
            TypeId::of::<FlashlightRotation>(),
            TypeId::of::<HighlightedItemStyle>(),
            TypeId::of::<HighlightingMode>(),
            TypeId::of::<GrabbedItemStyle>(),
            TypeId::of::<HighlightedIsGrabbable>(),
            TypeId::of::<GrabbedItem>(),
            TypeId::of::<GrabbingBehavior>(),
            TypeId::of::<UiFontPath>(),
            // TypeId::of::<UiFont>(),
            TypeId::of::<GuiState>(),
            // TypeId::of::<PauseState>(),
            TypeId::of::<PhysicsPaused>(),
            TypeId::of::<StatusVisible>(),
            TypeId::of::<InstructionText>(),
            TypeId::of::<ShowedInstructions>(),
            TypeId::of::<LevelList>(),
            TypeId::of::<CurrentLevel>(),
            TypeId::of::<SkyboxSetup>(),
            TypeId::of::<StatsOverlayVisible>(),
            TypeId::of::<StatsOverlayStyle>(),
            TypeId::of::<VideoSettings>(),
            TypeId::of::<FovDelta>(),
            TypeId::of::<PlayerCameraViews>(),
            TypeId::of::<StationaryCameraTransform>(),
        ];
        Self(type_ids)
    }
}

/// Track the list of resources we want to save, using `TheResource::type_id()`.
#[derive(Resource, Clone)]
pub struct SavedComponents(pub Vec<TypeId>);

impl Default for SavedComponents {
    fn default() -> Self {
        let type_ids = vec![
            TypeId::of::<ChildOf>(),
            TypeId::of::<Transform>(),
            TypeId::of::<Name>(),
            TypeId::of::<OurPlayer>(),
            TypeId::of::<OurCamera>(),
            TypeId::of::<BackgroundAudio>(),
        ];
        Self(type_ids)
    }
}


/// Get all the entities that should be saved.
pub fn fetch_saveable_entities(world: &mut World, from_world_marker: bool) -> Vec<Entity> {
    let mut saveable_query_state = world.query_filtered::<Entity, With<Saveable>>();
    let mut saveables = vec![];

    if from_world_marker {
        let Some(world_marker) = world.get_resource_ref::<WorldMarkerEntity>() else {
            error!("no WorldMarkerEntity");
            return saveables
        };

        let world_marker = world_marker.0;

        let mut parent_query_state = world.query::<&ChildOf>();
        let parent_query = parent_query_state.query(world);
        for saveable in saveable_query_state.iter(&world) {
            let found = parent_query.iter_ancestors(saveable).any(|e| e == world_marker);
            if found {
                saveables.push(saveable);
            }
        }
    } else {
        saveables = saveable_query_state.iter(&world).collect();
    }

    let mut visited = saveables.iter().cloned().collect::<EntityHashSet>();
    let mut to_visit = saveables.iter().cloned().collect::<Vec<_>>();

    let mut child_query = world.query::<&Children>();
    while let Some(saveable) = to_visit.pop() {
        if let Ok(children) = child_query.get(&world, saveable) {
            for kid in children.iter() {
                if visited.insert(kid) {
                    to_visit.push(kid);
                }
            }
        }
    }

    let to_save = visited.into_iter().collect::<Vec<_>>();
    dbg!(to_save)
}

/// Save world state for the given entities.
pub fn save_world_state(world: &mut World, ents: Vec<Entity>) -> anyhow::Result<String> {
    let app_type_registry = world.resource::<AppTypeRegistry>().clone();

    let type_registry = app_type_registry.read();

    let saved_resources = world.resource::<SavedResources>().clone();
    let saved_components = world.resource::<SavedComponents>().clone();

    let dynamic_world_builder = DynamicWorldBuilder::from_world(world, &type_registry);

    let resource_filter = saved_resources
        .0
        .into_iter()
        .fold(WorldFilter::deny_all(), |rf, type_id| rf.allow_by_id(type_id));

    let component_filter = saved_components
        .0
        .into_iter()
        .fold(WorldFilter::deny_all(), |rf, type_id| rf.allow_by_id(type_id));

    let dynamic_world = dynamic_world_builder
        .with_resource_filter(resource_filter)
        .with_component_filter(component_filter)
        .extract_entities(ents.into_iter())
        .extract_resources()
        .build();

    let type_registry = world.resource::<AppTypeRegistry>();
    let type_registry = type_registry.read();
    let serialized_world = dynamic_world.serialize(&type_registry)?;

    Ok(serialized_world)
}

/// Given a [DynamicWorld] handle (e.g. loaded from `.scn` or `.scn.ron`),
/// apply its `resources` to the `world`.
pub fn load_world_resources(world: &mut World, custom_resources: Handle<DynamicWorld>) -> Result<(), String> {
    let mut new_resources = vec![];

    let mut errors = String::new();
    let asset_id = custom_resources.id();

    let world_assets = world.resource::<Assets<DynamicWorld>>();
    let world_asset = world_assets.get(asset_id).unwrap();

    for refl in world_asset.resources.iter() {
        let Some(type_info) = refl.get_represented_type_info() else {
            errors += &format!("{asset_id:?}: type {refl:?} not reflectable\n");
            continue;
        };
        let type_id = type_info.type_id();
        match refl.reflect_clone() {
            Ok(resource) => {
                if let Some(resource_id) = world.components().get_id(type_id) {
                    new_resources.push((resource_id, resource));
                } else {
                    log::warn!("can't restore typeid {}", type_info.type_path());
                }
            }
            Err(e) => {
                errors += &format!("{asset_id:?}: {e}\n");
                continue;
            }
        }
    }

    for (resource_id, resource) in new_resources.into_iter() {
        world.insert_reflect_resource(resource_id, resource);
    }

    errors.is_empty().ok_or(errors)
}

pub fn load_world_state(_world: &mut World, _text: &str) -> anyhow::Result<()> {
    unimplemented!();
}
