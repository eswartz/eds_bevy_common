//! This plugin and systems help spawn items which are modeled
//! by [SpawnShape] and [SpawnMaterial].
use std::hash::Hash;

use bevy::image::ImageLoaderSettings;
use bevy::image::ImageSampler;
use bevy::image::ImageSamplerDescriptor;
use bevy::mesh::PlaneMeshBuilder;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::world_serialization::WorldInstance;
use bevy::world_serialization::WorldInstanceReady;
use rustc_hash::FxHashMap;

use crate::prelude::*;

pub struct SpawnHelpersPlugin;

impl Plugin for SpawnHelpersPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(SpawnMaterialHandles::default())
            .insert_resource(SpawnMeshHandles::default())
            .add_message::<RefreshImages>()
            .add_systems(
                PreUpdate,
                (
                    handle_spawn_material,
                    handle_spawn_shape,
                ).chain()
            )
            .add_systems(
                PostUpdate,
                (
                    refresh_materials,
                    apply_uv_box_map,
                )
            )

            .add_systems(
                OnEnter(LevelState::Advance),
                (
                    cleanup_materials,
                    cleanup_meshes,
                )
            )
            .add_observer(twiddle_spawn_materials)
        ;
    }
}

pub fn make_image_loader_settings_applier(
    params: TextureParams,
    assume_is_srgb: bool,
) -> impl Fn(&mut ImageLoaderSettings) + Send + Sync + 'static {
    move |settings: &mut ImageLoaderSettings| {
        settings.is_srgb = params.is_srgb.unwrap_or(assume_is_srgb);
        let desc = match &params.filter {
            TextureFilter::Linear => {
                let mut desc = ImageSamplerDescriptor::linear();
                desc.set_address_mode(params.address_mode.into());
                desc
            }
            TextureFilter::Nearest => {
                let mut desc = ImageSamplerDescriptor::linear();
                desc.set_address_mode(params.address_mode.into());
                desc
            }
        };
        settings.sampler = ImageSampler::Descriptor(desc);
    }
}

pub fn make_image_settings_applier(
    params: TextureParams,
) -> impl Fn(&mut Image) + Send + Sync + 'static {
    move |settings: &mut Image| {
        settings.sampler = match &params.filter {
            TextureFilter::Linear => {
                let mut desc = ImageSamplerDescriptor::linear();
                desc.set_address_mode(params.address_mode.into());
                ImageSampler::Descriptor(desc)
            }
            TextureFilter::Nearest => {
                let mut desc = ImageSamplerDescriptor::nearest();
                desc.set_address_mode(params.address_mode.into());
                ImageSampler::Descriptor(desc)
            }
        }
    }
}

/// When added or modified on an entity with a [MeshMaterial3d<StandardMaterial>],
/// ensures [StandardMaterial::depth_map] references a loaded handle for `source`
/// along with the other related fields.
#[derive(Component, Debug, Clone, Reflect)]
#[component(storage = "SparseSet")]
#[reflect(Component, Default, Clone)]
#[type_path = "game"]
pub struct EnsureDepthMapMaterial {
    pub source: TextureSource,
    pub parallax_depth_scale: f32,
    pub parallax_mapping_method: ParallaxMappingMethod,
    pub max_parallax_layer_count: f32,

    ticks: u32,
}

impl Default for EnsureDepthMapMaterial {
    fn default() -> Self {
        Self {
            source: TextureSource::default(),
            parallax_depth_scale: 0.01,
            parallax_mapping_method: ParallaxMappingMethod::default(),
            max_parallax_layer_count: 8.0,
            ticks: 0,
        }
    }
}

#[derive(Resource, Default)]
pub struct DepthMapStorage {
    /// Map of original material to cloned then depth-map-applied material.
    pub orig_to_edited: FxHashMap<Handle<StandardMaterial>, Handle<StandardMaterial>>,
}

pub fn handle_depth_map(
    assets: ResMut<AssetServer>,
    images: Res<Assets<Image>>,
    mut std_mats: ResMut<Assets<StandardMaterial>>,
    mut dm_storage: ResMut<DepthMapStorage>,
    mut mat_q: Query<(Entity, &mut EnsureDepthMapMaterial, &mut MeshMaterial3d<StandardMaterial>),
    Or<(
        Changed<EnsureDepthMapMaterial>,
        Changed<MeshMaterial3d<StandardMaterial>>,
    )>>,
) -> Result {
    for (entity, mut depth_map_mat, mut mesh_mat) in mat_q.iter_mut() {
        let orig_std_mat_handle = mesh_mat.0.clone();
        let orig_std_mat = std_mats
            .get(&orig_std_mat_handle)
            .ok_or_else(|| anyhow::anyhow!("no StandardMaterial on {entity}"))?
            .clone();

        // Ensure we have (or start loading) the texture.
        let new_depth_map = depth_map_mat.source.get_handle(&assets, false);
        if !depth_map_mat.source.is_handle() || images.get(&new_depth_map).is_none() {
            // Wait for it to be loaded to avoid wgpu validation crash
            // `Incompatible sample count: the RenderPass uses textures with sample count 1 but the RenderPipeline with 'pbr_opaque_mesh_pipeline' label uses attachments with format 4`
            info!("Waiting...");
            depth_map_mat.source = TextureSource::Handle(new_depth_map);
            depth_map_mat.ticks = 10;
            continue;
        }
        else if depth_map_mat.source.is_handle() && depth_map_mat.ticks != 0 {
            debug!("Waiting...");
            continue;
        }

        let new_mat_handle = if let Some(exist_handle) = dm_storage.orig_to_edited.get(&mesh_mat)
            && let Some(depth_mat) = std_mats.get(exist_handle)
            && depth_mat.depth_map.as_ref().is_some_and(|dm| dm.id() == new_depth_map.id())
        {
            info!("Reusing {exist_handle:?} on depth map in {entity}");
            exist_handle.clone()
        } else {
            let new_mat = StandardMaterial {
                depth_map: Some(new_depth_map),
                parallax_depth_scale: depth_map_mat.parallax_depth_scale,
                parallax_mapping_method: depth_map_mat.parallax_mapping_method,
                max_parallax_layer_count: depth_map_mat.max_parallax_layer_count,
                ..orig_std_mat
            };
            let new_mat_handle = std_mats.add(new_mat);

            info!("New depth map material {:?}", new_mat_handle);
            dm_storage.orig_to_edited.insert(
                mesh_mat.0.clone(),
                new_mat_handle.clone(),
            );
            new_mat_handle.clone()
        };

        // Just update.
        mesh_mat.set_if_neq(MeshMaterial3d(new_mat_handle));
    }

    Ok(())
}

pub fn tick_depth_map(
    images: Res<Assets<Image>>,
    mut depth_q: Query<&mut EnsureDepthMapMaterial>,
) -> Result {
    for mut depth_map in depth_q.iter_mut() {
        if depth_map.ticks > 0
        && let TextureSource::Handle(handle) = &depth_map.source
        && images.get(handle).is_some() {
            depth_map.ticks -= 1;
        }
    }

    Ok(())
}

/// Marker used to apply a "box" UV mapping -- i.e. taking
/// the AABB of the mesh and assigning UV coordinates
/// for each of the 6 axis-aligned faces, chosen by distance
/// from the center.
#[derive(Component, Debug, Default, Clone, Reflect)]
#[component(storage = "SparseSet")]
#[reflect(Component, Default, Clone)]
#[type_path = "game"]
pub struct ApplyUvBoxMap {
    /// How many times per face to see the texture.
    pub repeats: Vec3,
}

fn apply_uv_box_map(
    mut meshes: ResMut<Assets<Mesh>>,
    mut mesh_q: Query<(Entity, &ApplyUvBoxMap, &mut Mesh3d), Or<(Changed<ApplyUvBoxMap>, Changed<Mesh3d>)>>,
) -> Result {
    for (_entity, uv_map, mut mesh3d) in mesh_q.iter_mut() {
        let Some(mesh) = meshes.get(mesh3d.id()) else { continue };

        let mesh = create_uvmapped_mesh_scaled(mesh.clone(), uv_map.repeats);

        mesh3d.0 = meshes.add(mesh);
    }

    Ok(())
}

/// A component that builds the given Mesh when needed.
#[derive(Component, Debug, Default, Clone, Reflect, PartialEq, Eq, Hash)]
#[component(storage = "SparseSet")]
#[reflect(Component, Default, Clone)]
#[type_path = "game"]
pub struct SpawnShape {
    pub kind: SpawnShapeKind,
    pub info: SpawnShapeInfo,
}

#[derive(Default, Debug, Clone, Reflect, PartialEq)]
#[reflect(Default, Clone)]
#[type_path = "game"]
pub enum SpawnShapeKind {
    Cube(Vec3),
    Sphere(f32),
    Plane(Vec2),
    Model(String),
    MeshMaterial{ mesh: String, material: String },
    #[default]
    None,
}

/// We don't care about NaNs.
impl Eq for SpawnShapeKind {}

impl std::hash::Hash for SpawnShapeKind {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            SpawnShapeKind::Cube(vec3) => {
                Float32(vec3.x).hash(state);
                Float32(vec3.y).hash(state);
                Float32(vec3.z).hash(state);
            }
            SpawnShapeKind::Sphere(rad) => {
                Float32(*rad).hash(state);
            }
            SpawnShapeKind::Plane(vec2) => {
                Float32(vec2.x).hash(state);
                Float32(vec2.y).hash(state);
            }
            SpawnShapeKind::Model(path) => {
                path.hash(state);
            }
            SpawnShapeKind::MeshMaterial { mesh, material } => {
                mesh.hash(state);
                material.hash(state);
            }
            SpawnShapeKind::None => {
                0u32.hash(state);
            }
        }
    }
}

#[derive(Default, Debug, Clone, Reflect, PartialEq, Eq, Hash)]
#[reflect(Default, Clone)]
#[type_path = "game"]
pub struct SpawnShapeInfo {
    pub subdivisions: u16,
}

#[derive(Resource, Default)]
struct SpawnMeshHandles(HashMap<(SpawnShape, MeshQuality), Handle<Mesh>>);

fn handle_spawn_shape(
    mut commands: Commands,

    assets: Res<AssetServer>,
    mut meshes: If<ResMut<Assets<Mesh>>>,
    vid_settings: Res<VideoSettings>,

    shape_q: Query<(Entity, &SpawnShape)>,
    mut mesh_cache: ResMut<SpawnMeshHandles>,
) {
    // Spawn the appropriate mesh and remove the SpawnShape when complete.
    for (ent, shape) in shape_q.iter() {
        let mut ent_commands = commands.entity(ent);
        match &shape.kind {
            SpawnShapeKind::Cube(vec3) => {
                let mesh = mesh_cache.0.entry((shape.clone(), MeshQuality::Low))
                    .or_insert_with(|| {
                        let shape = Cuboid::new(vec3.x, vec3.y, vec3.z);
                        let mut mesh: Mesh = shape.mesh().into();
                        mesh.generate_tangents().unwrap();
                        meshes.add(mesh)
                    });
                ent_commands.try_insert(Mesh3d(mesh.clone()));
            }
            SpawnShapeKind::Sphere(rad) => {
                let mesh = mesh_cache.0.entry((shape.clone(), vid_settings.mesh_quality))
                    .or_insert_with(|| {
                        let (sectors, stacks) = {
                            {
                                match vid_settings.mesh_quality {
                                    MeshQuality::Low => (12, 6),
                                    MeshQuality::Medium => (24, 12),
                                    MeshQuality::High => (32, 18),
                                    MeshQuality::Ultra => (48, 24),
                                }
                            }
                        };
                        let mut mesh = Sphere::new(*rad).mesh().uv(sectors, stacks);
                        // mesh.compute_smooth_normals();   // BUG: breaks lower hemisphere
                        let _ = mesh.generate_tangents();
                        meshes.add(mesh)
                    });

                ent_commands.try_insert(Mesh3d(mesh.clone()));
            }
            SpawnShapeKind::Plane(vec2) => {
                let mesh = mesh_cache.0.entry((shape.clone(), MeshQuality::Low))
                    .or_insert_with(|| {
                        let mesh = PlaneMeshBuilder::from_size(Vec2::new(vec2.x, vec2.y))
                            .subdivisions(shape.info.subdivisions as u32)
                            .build();
                        meshes.add(mesh)
                    });

                ent_commands.try_insert(Mesh3d(mesh.clone()));
            }
            SpawnShapeKind::Model(path) => {
                let scene = assets.load::<WorldAsset>(path);
                ent_commands.try_insert(WorldAssetRoot(scene));
            }
            SpawnShapeKind::MeshMaterial{ mesh, material } => {
                if !mesh.contains("#Mesh") {
                    error!("unexpected Mesh Model path {mesh}");
                    ent_commands.try_remove::<SpawnShape>();
                    return
                }
                if !material.contains("#Material") {
                    error!("unexpected Material path {material}");
                    ent_commands.try_remove::<SpawnShape>();
                    return
                }

                let mesh_handle = assets.load::<Mesh>(mesh);
                let mat_handle = assets.load::<StandardMaterial>(material);

                ent_commands.try_insert((
                    Mesh3d(mesh_handle),
                    MeshMaterial3d(mat_handle),
                ));
            }

            // 'twas just a placeholder.
            SpawnShapeKind::None => (),
        };

        // All the successful paths lead here.
        ent_commands.try_remove::<SpawnShape>();
    }
}

/// A component that applies a material to the [Mesh3d] directly on it
/// or in the child hierarchy if placed on [WorldInstance].
#[derive(Component, Debug, Default, Clone, Reflect)]
#[component(storage = "SparseSet")]
#[reflect(Component, Default, Clone)]
#[type_path = "game"]
pub enum SpawnMaterial {
    StdMat(StandardMaterialModel),
    #[default]
    None,
}

/// Record which materials we generated.
#[derive(Resource, Default)]
pub struct SpawnMaterialHandles (
    HashMap<StandardMaterialModel, Handle<StandardMaterial>>,
);

impl SpawnMaterialHandles {
    pub fn allocate(&mut self,
        mat: &SpawnMaterial,
        assets: &AssetServer,
        mut mats: Mut<Assets<StandardMaterial>>,
    ) -> Handle<StandardMaterial> {
        match mat {
            SpawnMaterial::StdMat(mat_model) => {
                let std_mat = self.0.entry(mat_model.clone())
                    .or_insert_with(|| {
                        let mat: StandardMaterial = mat_model.into_standard_material(assets);
                        mats.add(mat)
                    });

                std_mat.clone()
            }
            SpawnMaterial::None => default(),
        }
    }
}

/// When we detect a world instance was loaded, make sure we (re)check the
/// SpawnMaterial component and apply it to the (new) children.
fn twiddle_spawn_materials(
    event: On<WorldInstanceReady>,
    mut spawn_mat_q: Query<&mut SpawnMaterial>,
    child_q: Query<&Children>,
) {
    if let Ok(mut spawn_mat) = spawn_mat_q.get_mut(event.event_target()) {
        spawn_mat.set_changed();
    }
    for kid in child_q.iter_descendants(event.event_target()) {
        if let Ok(mut spawn_mat) = spawn_mat_q.get_mut(kid) {
            spawn_mat.set_changed();
        }
    }
}

fn handle_spawn_material(
    mut commands: Commands,
    mut mats: If<ResMut<Assets<StandardMaterial>>>,
    assets: Res<AssetServer>,
    spawn_mat_q: Query<(Entity, &SpawnMaterial), Or<(Added<Mesh3d>, Changed<SpawnMaterial>)>>,
    mesh_q: Query<&Mesh3d>,
    child_q: Query<&Children>,
    glass_q: Query<&GlassTweak>,
    scene_q: Query<&WorldInstance>,
    mut mat_cache: If<ResMut<SpawnMaterialHandles>>,
) {
    for (ent, spawn_mat) in spawn_mat_q.iter() {
        let std_mat = mat_cache.allocate(spawn_mat, &assets, mats.reborrow());

        // Simple mesh?
        if mesh_q.contains(ent) && !scene_q.contains(ent) {
            commands.entity(ent).try_insert(MeshMaterial3d(std_mat.clone()));
        } else {
            // Else, assume it's a complex model and all the mesh children need to be affected.
            for kid in child_q.iter_descendants(ent) {
                if scene_q.contains(kid) {
                    break;
                }
                if mesh_q.contains(kid) {
                    commands.entity(kid).try_insert(MeshMaterial3d(std_mat.clone()));
                    if glass_q.contains(ent) {
                        commands.entity(kid).try_insert(GlassTweak);
                    }
                }
            }
        };
    }
}

fn cleanup_materials(
    mut mats: If<ResMut<SpawnMaterialHandles>>,
) {
    mats.0.0.clear();
}

fn cleanup_meshes(
    mut meshes: If<ResMut<SpawnMeshHandles>>,
) {
    meshes.0.0.clear();
}

#[derive(Debug, Default, Clone, Message)]
pub struct RefreshImages;

pub fn refresh_materials(
    reader: MessageReader<RefreshImages>,
    mut std_mats: If<ResMut<Assets<StandardMaterial>>>,
) {
    if !reader.is_empty() {
        // Bug (?): need to probe all the materials using the Image so they'll re-render.
        for mat in std_mats.iter_mut() {
            let _ = mat;
        }
    }
}
