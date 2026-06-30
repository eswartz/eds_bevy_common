use bevy::gltf::GltfMaterial;
use bevy::gltf::GltfMesh;
use bevy::image::ImageAddressMode;
use bevy::image::ImageLoaderSettings;
use bevy::image::ImageSampler;
use bevy::image::ImageSamplerDescriptor;
use bevy::mesh::PlaneMeshBuilder;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use rustc_hash::FxHashMap;
use wgpu::Face;
use wgpu::TextureFormat;

use crate::LevelState;
use crate::MeshQuality;
use crate::VideoSettings;
use crate::create_uvmapped_mesh_scaled;

pub struct MaterialsPlugin;

impl Plugin for MaterialsPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(SpawnMaterialHandles::default())
            .add_message::<RefreshImages>()
            .add_systems(
                FixedPreUpdate,
                (
                    handle_spawn_shape,
                    handle_spawn_texture,
                    handle_spawn_material,
                )
            )
            .add_systems(
                PostUpdate,
                (
                    refresh_materials,
                )
            )

            .add_systems(
                OnEnter(LevelState::Advance),
                cleanup_materials,
            )
        ;
    }
}

/// Reflectable version of Face.
#[derive(Debug, Default, Clone, Reflect)]
#[reflect(Default, Clone)]
#[type_path = "game"]
pub enum MaterialCullFace {
    None,
    Front,
    #[default]
    Back,
}


#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect)]
#[reflect(Default, Clone)]
#[type_path = "game"]
pub struct TextureParams {
    /// Tell if the image content should be interpreted as sRGB.
    pub is_srgb: Option<bool>,
    /// Tell how the image should be filtered for rendering.
    pub filter: TextureFilter,
    /// Tell how out-of-bounds UVs are handled.
    pub address_mode: TextureAddressMode,
}

impl Default for TextureParams {
    fn default() -> Self {
        Self {
            is_srgb: None,
            filter: TextureFilter::Linear,
            address_mode: TextureAddressMode::Repeat,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Reflect)]
#[reflect(Default, Clone)]
#[type_path = "game"]
pub enum TextureImageFormat {
    #[default]
    Rgba8Unorm,
    Rgba8Snorm,
    Rgba16Float,
    Rgba32Float,
}

impl From<TextureImageFormat> for TextureFormat {
    fn from(value: TextureImageFormat) -> Self {
        match value {
            TextureImageFormat::Rgba8Unorm => TextureFormat::Rgba8Unorm,
            TextureImageFormat::Rgba8Snorm => TextureFormat::Rgba8Snorm,
            TextureImageFormat::Rgba16Float => TextureFormat::Rgba16Float,
            TextureImageFormat::Rgba32Float => TextureFormat::Rgba32Float,
        }
    }
}


#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
#[reflect(Default, Clone)]
#[type_path = "game"]
pub enum TextureFilter {
    #[default]
    Linear,
    Nearest,
}

/// Our default is `Repeat` unlike Bevy.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
#[reflect(Default, Clone)]
#[type_path = "game"]
pub enum TextureAddressMode {
    /// Clamp the value to the edge of the texture.
    ///
    /// -0.25 -> 0.0
    /// 1.25  -> 1.0
    //#[default]
    ClampToEdge,
    /// Repeat the texture in a tiling fashion.
    ///
    /// -0.25 -> 0.75
    /// 1.25 -> 0.25
    #[default]      // different for script!
    Repeat,
    /// Repeat the texture, mirroring it every repeat.
    ///
    /// -0.25 -> 0.25
    /// 1.25 -> 0.75
    MirrorRepeat,
    /// Clamp the value to the border of the texture
    /// Requires the wgpu feature [`Features::ADDRESS_MODE_CLAMP_TO_BORDER`].
    ///
    /// -0.25 -> border
    /// 1.25 -> border
    ClampToBorder,
}

impl From<TextureAddressMode> for ImageAddressMode {
    fn from(value: TextureAddressMode) -> Self {
        match value {
            TextureAddressMode::ClampToEdge => ImageAddressMode::ClampToEdge,
            TextureAddressMode::Repeat => ImageAddressMode::Repeat,
            TextureAddressMode::MirrorRepeat => ImageAddressMode::MirrorRepeat,
            TextureAddressMode::ClampToBorder => ImageAddressMode::ClampToBorder,
        }
    }
}

/// The way a texture can be loaded.
#[derive(Debug, Clone, Reflect)]
#[reflect(Clone)]
#[type_path = "game"]
pub enum TextureSource {
    Load{
        path: String,
        params: TextureParams,
    },
    Handle(Handle<Image>),
}

impl Default for TextureSource {
    fn default() -> Self {
        TextureSource::Handle(default())
    }
}

impl TextureSource {
    pub fn is_handle(&self) -> bool {
        matches!(self, TextureSource::Handle(_))
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

impl TextureSource {
    /// For `Load` variants, load the image from the AssetServer if needed.
    /// Otherwise return the handle as-is.
    pub fn get_handle(&self, assets: &AssetServer, assume_is_srgb: bool) -> Handle<Image> {
        match self {
            TextureSource::Load{ path, params } => assets
                .load_builder()
                .with_settings(
                    make_image_loader_settings_applier(
                        params.clone(), assume_is_srgb,
                    ),
                )
                .load(path),
            TextureSource::Handle(handle) => handle.clone(),
        }
    }
}

/// A component that prompts loading images and populating
/// them in a corresponding [SpawnMaterial].
#[derive(Component, Default, Clone, Reflect)]
#[component(storage = "SparseSet")]
#[reflect(Component, Default, Clone)]
#[type_path = "game"]
pub struct TextureSources {
    /// Asset path to color texture.
    pub base_color: Option<TextureSource>,
    pub normal_map: Option<TextureSource>,
    pub emissive_color: Option<TextureSource>,
    pub metallic_roughness: Option<TextureSource>,
    pub diffuse_transmission: Option<TextureSource>,
    pub specular_transmission: Option<TextureSource>,
    pub thickness: Option<TextureSource>,
    pub occlusion: Option<TextureSource>,
    pub specular: Option<TextureSource>,
    pub specular_tint: Option<TextureSource>,
    pub clearcoat: Option<TextureSource>,
    pub clearcoat_roughness: Option<TextureSource>,
    pub clearcoat_normal: Option<TextureSource>,
    pub anisotropy: Option<TextureSource>,
    pub depth_map: Option<TextureSource>,
}

impl TextureSources {
    pub fn is_empty(&self) -> bool {
        self.base_color.is_none() &&
        self.normal_map.is_none() &&
        self.emissive_color.is_none() &&
        self.metallic_roughness.is_none() &&
        self.diffuse_transmission.is_none() &&
        self.specular_transmission.is_none() &&
        self.thickness.is_none() &&
        self.occlusion.is_none() &&
        self.specular.is_none() &&
        self.specular_tint.is_none() &&
        self.clearcoat.is_none() &&
        self.clearcoat_roughness.is_none() &&
        self.clearcoat_normal.is_none() &&
        self.anisotropy.is_none() &&
        self.depth_map.is_none() &&
        true
    }
}

pub fn handle_spawn_texture(
    mut commands: Commands,
    assets: If<ResMut<AssetServer>>,
    mut tex_q: Query<(Entity, &TextureSources, &mut SpawnMaterial), Without<SpawnShape>>,
) {
    for (ent, tex, mut mat) in tex_q.iter_mut() {
        let mut ent_commands = commands.entity(ent);
        match &mut *mat {
            SpawnMaterial::StdMat(mat, _cull) => {
                macro_rules! handle {
                    ($source:ident, $tex:ident, $is_srgb:expr) => {
                        if let Some(source) = &tex.$source {
                            mat.$tex = Some(source.get_handle(&assets, $is_srgb));
                        }
                    }
                }

                handle!(base_color, base_color_texture, true);
                handle!(normal_map, normal_map_texture, false);
                handle!(emissive_color, emissive_texture, true);
                handle!(metallic_roughness, metallic_roughness_texture, false);
                handle!(depth_map, depth_map, false);

                #[cfg(feature = "pbr_transmission_textures")]
                {
                    handle!(diffuse_transmission, diffuse_transmission_texture, false);
                    handle!(specular_transmission, specular_transmission_texture, false);
                    handle!(thickness, thickness_texture, false);
                }
                handle!(occlusion, occlusion_texture, false);
                #[cfg(feature = "pbr_specular_textures")]
                {
                    handle!(specular, specular_texture, false);
                    handle!(specular_tint, specular_tint_texture, false);
                }
                #[cfg(feature = "pbr_multi_layer_material_textures")]
                {
                    handle!(clearcoat, clearcoat_texture, false);
                    handle!(clearcoat_roughness, clearcoat_roughness_texture, false);
                    handle!(clearcoat_normal, clearcoat_normal_texture, false);
                }
                #[cfg(feature = "pbr_anisotropy_texture")]
                {
                    handle!(anisotropy, anisotropy_texture, false);
                }
            }
            SpawnMaterial::None => (),
        }

        ent_commands.try_remove::<TextureSources>();
    }
}

/// When added or modified on an entity with a `MeshMaterial3d<StandardMaterial>`,
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

pub fn apply_uv_box_map(
    mut meshes: ResMut<Assets<Mesh>>,
    mut mesh_q: Query<(Entity, &ApplyUvBoxMap, &mut Mesh3d), Changed<ApplyUvBoxMap>>,
) -> Result {
    for (_entity, uv_map, mut mesh3d) in mesh_q.iter_mut() {
        let Some(mesh) = meshes.get(mesh3d.id()) else { continue };

        let mesh = create_uvmapped_mesh_scaled(mesh.clone(), uv_map.repeats);

        mesh3d.0 = meshes.add(mesh);
    }

    Ok(())
}



/// A component that builds the given Mesh when needed.
#[derive(Component, Debug, Default, Clone, Reflect)]
#[component(storage = "SparseSet")]
#[reflect(Component, Default, Clone)]
#[type_path = "game"]
pub struct SpawnShape {
    pub kind: SpawnShapeKind,
    pub info: SpawnShapeInfo,
}

#[derive(Default, Debug, Clone, Reflect)]
#[reflect(Default, Clone)]
#[type_path = "game"]
pub enum SpawnShapeKind {
    Cube(Vec3),
    Sphere(f32),
    Plane(Vec2),
    Model(String),
    #[default]
    None,
}

#[derive(Default, Debug, Clone, Reflect)]
#[reflect(Default, Clone)]
#[type_path = "game"]
pub struct SpawnShapeInfo {
    pub subdivisions: u16,
}

pub fn handle_spawn_shape(
    mut commands: Commands,

    assets: Res<AssetServer>,
    gltf_meshes: If<Res<Assets<GltfMesh>>>,
    mut meshes: If<ResMut<Assets<Mesh>>>,
    gltf_mats: If<Res<Assets<GltfMaterial>>>,
    mut stdmats: If<ResMut<Assets<StandardMaterial>>>,
    vid_settings: Res<VideoSettings>,

    shape_q: Query<(Entity, &SpawnShape)>,

    // FIXME: need to do this to avoid delayed loading every single time the model is loaded...
    mut mesh_cache: Local<FxHashMap<String, Handle<GltfMesh>>>,
) {
    // Spawn the appropriate mesh and remove the SpawnShape when complete.
    for (ent, shape) in shape_q.iter() {
        let mut ent_commands = commands.entity(ent);
        match &shape.kind {
            SpawnShapeKind::Cube(vec3) => {
                let shape = Cuboid::new(vec3.x, vec3.y, vec3.z);
                let mut mesh: Mesh = shape.mesh().into();
                mesh.generate_tangents().unwrap();
                let mesh = meshes.add(mesh);
                ent_commands.try_insert(Mesh3d(mesh));
            }
            SpawnShapeKind::Sphere(rad) => {
                let (sectors, stacks) = {
                    {
                        match vid_settings.mesh_quality {
                            MeshQuality::Low => (12, 6),
                            MeshQuality::Medium => (16, 10),
                            MeshQuality::High => (20, 14),
                            MeshQuality::Ultra => (24, 18),
                        }
                    }
                };
                let mut mesh = Sphere::new(*rad).mesh().uv(sectors, stacks);
                mesh.compute_smooth_normals();
                let _ = mesh.generate_tangents();
                ent_commands.try_insert(Mesh3d(meshes.add(mesh)));
            }
            SpawnShapeKind::Plane(vec2) => {
                let mesh = PlaneMeshBuilder::from_size(Vec2::new(vec2.x, vec2.y))
                    .subdivisions(shape.info.subdivisions as u32)
                    .build();
                let mesh = meshes.add(mesh);
                ent_commands.try_insert(Mesh3d(mesh));
                ent_commands.try_remove::<SpawnShape>();
            }
            SpawnShapeKind::Model(path) => {
                if path.contains("#Scene") || !path.contains('#') {
                    let scene = assets.load::<WorldAsset>(path);
                    ent_commands.try_insert(WorldAssetRoot(scene));
                } else if path.contains("#Mesh") {
                    let mesh_handle;
                    if let Some(cached_mesh) = assets.get_handle(&*path) {
                        mesh_handle = cached_mesh.clone();
                    } else {
                        mesh_handle = assets.load::<GltfMesh>(path);
                        mesh_cache.insert(path.clone(), mesh_handle.clone());
                    }

                    // Might take a while to load, so keep iterating until we see it.
                    // (We need its precious data!)
                    let Some(gltf_mesh) = gltf_meshes.get(&mesh_handle) else {
                        debug!("{ent}: no GltfMesh found for {path} (waiting) [{mesh_handle:?}]");
                        continue
                    };

                    // Got it!
                    if gltf_mesh.primitives.len() == 0 {
                        warn!("{ent}: GltfMesh {path} is empty");
                    } else if gltf_mesh.primitives.len() == 1 {
                        // Single mesh, add directly.
                        let prim = &gltf_mesh.primitives[0];
                        ent_commands.try_insert(Mesh3d(prim.mesh.clone()));
                        if let Some(mat) = &prim.material
                            && let Some(mat) = gltf_mats.get(mat)
                        {
                            let stdmat = standard_material_from_gltf_material(mat);
                            ent_commands.try_insert(MeshMaterial3d(stdmats.add(stdmat)));
                        }
                    } else {
                        // Need to add each as a child.
                        ent_commands.with_children(|spawn| {
                            for prim in &gltf_mesh.primitives {
                                let mut kid_commands = spawn.spawn(Mesh3d(prim.mesh.clone()));
                                if let Some(mat) = &prim.material
                                    && let Some(mat) = gltf_mats.get(mat) {
                                    let stdmat = standard_material_from_gltf_material(mat);
                                    kid_commands.try_insert(MeshMaterial3d(stdmats.add(stdmat)));
                                }
                            }
                        });
                    }

                    // Done!
                    ent_commands.try_remove::<SpawnShape>();
                } else {
                    error!("unexpected Model path {path}");
                }
            }

            // 'twas just a placeholder.
            SpawnShapeKind::None => (),
        }

        // All the successful paths lead here.
        ent_commands.try_remove::<SpawnShape>();
    }
}

/// Converts a [`GltfMaterial`] to a [`StandardMaterial`]
// copied from bevy::pbr::gltf, which is private...
pub fn standard_material_from_gltf_material(material: &GltfMaterial) -> StandardMaterial {
    StandardMaterial {
        base_color: material.base_color,
        base_color_channel: material.base_color_channel.clone(),
        base_color_texture: material.base_color_texture.clone(),
        emissive: material.emissive,
        emissive_channel: material.emissive_channel.clone(),
        emissive_texture: material.emissive_texture.clone(),
        perceptual_roughness: material.perceptual_roughness,
        metallic: material.metallic,
        metallic_roughness_channel: material.metallic_roughness_channel.clone(),
        metallic_roughness_texture: material.metallic_roughness_texture.clone(),
        reflectance: material.reflectance,
        specular_tint: material.specular_tint,
        specular_transmission: material.specular_transmission,
        #[cfg(feature = "pbr_transmission_textures")]
        specular_transmission_channel: material.specular_transmission_channel.clone(),
        #[cfg(feature = "pbr_transmission_textures")]
        specular_transmission_texture: material.specular_transmission_texture.clone(),
        thickness: material.thickness,
        #[cfg(feature = "pbr_transmission_textures")]
        thickness_channel: material.thickness_channel.clone(),
        #[cfg(feature = "pbr_transmission_textures")]
        thickness_texture: material.thickness_texture.clone(),
        ior: material.ior,
        attenuation_distance: material.attenuation_distance,
        attenuation_color: material.attenuation_color,
        normal_map_channel: material.normal_map_channel.clone(),
        normal_map_texture: material.normal_map_texture.clone(),
        occlusion_channel: material.occlusion_channel.clone(),
        occlusion_texture: material.occlusion_texture.clone(),
        #[cfg(feature = "pbr_specular_textures")]
        specular_channel: material.specular_channel.clone(),
        #[cfg(feature = "pbr_specular_textures")]
        specular_texture: material.specular_texture.clone(),
        #[cfg(feature = "pbr_specular_textures")]
        specular_tint_channel: material.specular_tint_channel.clone(),
        #[cfg(feature = "pbr_specular_textures")]
        specular_tint_texture: material.specular_tint_texture.clone(),
        clearcoat: material.clearcoat,
        #[cfg(feature = "pbr_multi_layer_material_textures")]
        clearcoat_channel: material.clearcoat_channel.clone(),
        #[cfg(feature = "pbr_multi_layer_material_textures")]
        clearcoat_texture: material.clearcoat_texture.clone(),
        clearcoat_perceptual_roughness: material.clearcoat_perceptual_roughness,
        #[cfg(feature = "pbr_multi_layer_material_textures")]
        clearcoat_roughness_channel: material.clearcoat_roughness_channel.clone(),
        #[cfg(feature = "pbr_multi_layer_material_textures")]
        clearcoat_roughness_texture: material.clearcoat_roughness_texture.clone(),
        #[cfg(feature = "pbr_multi_layer_material_textures")]
        clearcoat_normal_channel: material.clearcoat_normal_channel.clone(),
        #[cfg(feature = "pbr_multi_layer_material_textures")]
        clearcoat_normal_texture: material.clearcoat_normal_texture.clone(),
        anisotropy_strength: material.anisotropy_strength,
        anisotropy_rotation: material.anisotropy_rotation,
        #[cfg(feature = "pbr_anisotropy_texture")]
        anisotropy_channel: material.anisotropy_channel.clone(),
        #[cfg(feature = "pbr_anisotropy_texture")]
        anisotropy_texture: material.anisotropy_texture.clone(),
        double_sided: material.double_sided,
        cull_mode: material.cull_mode,
        unlit: material.unlit,
        alpha_mode: material.alpha_mode,
        uv_transform: material.uv_transform,
        ..Default::default()
    }
}

/// A component that applies a material.
#[derive(Component, Debug, Default, Clone, Reflect)]
#[component(storage = "SparseSet")]
#[reflect(Component, Default, Clone)]
#[type_path = "game"]
pub enum SpawnMaterial {
    StdMat(StandardMaterial, MaterialCullFace),
    #[default]
    None,
}

/// Record which materials we generated.
#[derive(Resource, Default)]
pub(crate) struct SpawnMaterialHandles (
    HashMap<StandardMaterialHash, Handle<StandardMaterial>>,
);

#[derive(Debug, Clone, PartialEq, Eq,Hash)]
pub(crate) struct StandardMaterialHash(String);

impl StandardMaterialHash {
    fn new(mat: String) -> Self {
        Self(mat)
    }
}

fn hash_color(color: Color) -> String {
    format!("{}", color.to_linear().as_u32())
}
fn hash_image(image: &Option<Handle<Image>>) -> String {
    if let Some(image) = image {
        format!("{}", image.id())
    } else {
        default()
    }
}

fn hash_stdmat(m: &StandardMaterial) -> String {
    let basic = format!(
        "bc={}
        bcc={:?}
        bct={}
        e={}
        eew={:.2}
        ec={:?}
        et={}
        pr={:.2}
        m={:.2}
        mrgc={:?}
        mrt={}
        r={:.2}
        st={}
        dt={:.3}
        st={:.3}
        //
        t={}
        ior={}
        ad={}
        ac={}
        nmc={:?}
        nmt={}
        fn={}
        oc={:?}
        ot={}
        cc={:.2}
        as={:.3}
        ar={:.3}
        ds={}
        cm={:?}
        un={}
        fe={}
        am={:?}
        db={:.4}
        dm={:?}
        pds={:.3}
    ",
        hash_color(m.base_color),
        m.base_color_channel,
        hash_image(&m.base_color_texture),
        hash_color(m.emissive.into()),
        m.emissive_exposure_weight,
        m.emissive_channel,
        hash_image(&m.emissive_texture),
        m.perceptual_roughness,
        m.metallic,
        m.metallic_roughness_channel,
        hash_image(&m.metallic_roughness_texture),
        m.reflectance,
        hash_color(m.specular_tint),
        m.diffuse_transmission,
        m.specular_transmission,
        //
        m.thickness,
        m.ior,
        m.attenuation_distance,
        hash_color(m.attenuation_color),
        m.normal_map_channel,
        hash_image(&m.normal_map_texture),
        m.flip_normal_map_y,
        m.occlusion_channel,
        hash_image(&m.occlusion_texture),
        //
        m.clearcoat,
        //
        m.anisotropy_strength,
        m.anisotropy_rotation,
        m.double_sided,
        m.cull_mode,
        m.unlit,
        m.fog_enabled,
        m.alpha_mode,
        m.depth_bias,
        m.depth_map,
        m.parallax_depth_scale,
        //
    );

    #[cfg(not(feature = "pbr_transmission_textures"))]
    let ptt = "".to_string();

    #[cfg(feature = "pbr_transmission_textures")]
    let ptt = format!("dtc={:?} dtt={} stc={:?} tc={:?} tt={} ",
        m.diffuse_transmission_channel,
        hash_image(&m.diffuse_transmission_texture),
        m.specular_transmission_channel,
        hash_image(&m.specular_transmission_texture),
        m.thickness_channel,
        hash_image(&m.thickness_texture),
    );

    #[cfg(not(feature = "pbr_specular_textures"))]
    let pst = "".to_string();

    #[cfg(feature = "pbr_specular_textures")]
    let pst = format!("sc={:?} st={} stc={:?} stt={}",
        m.specular_channel,
        hash_image(&m.specular_texture),
        m.specular_tint_channel,
        hash_image(&m.specular_tint_texture),
    );

    #[cfg(not(feature = "pbr_multi_layer_material_textures"))]
    let pmlmt = "".to_string();

    #[cfg(feature = "pbr_multi_layer_material_textures")]
    let pmlmt = format!("",
        m.specular_channel,
        hash_image(&m.specular_texture),
        m.specular_tint_channel,
        hash_image(&m.specular_tint_texture),
    );

    format!("b={basic} ptt={ptt} pst={pst} pmlmt={pmlmt}")
}

pub(crate) fn handle_spawn_material(
    mut commands: Commands,
    mut mats: If<ResMut<Assets<StandardMaterial>>>,
    mat_q: Query<(Entity, &SpawnMaterial), (Without<TextureSources>, Without<SpawnShape>)>,

    mut std_mat_cache: ResMut<SpawnMaterialHandles>,
) {
    for (ent, mat) in mat_q.iter() {
        let mut ent_commands = commands.entity(ent);
        match mat {
            SpawnMaterial::StdMat(mat, cull) => {
                let mat = StandardMaterial {
                    cull_mode: match cull {
                        MaterialCullFace::None => None,
                        MaterialCullFace::Front => Some(Face::Front),
                        MaterialCullFace::Back => Some(Face::Back),
                    },
                    ..mat.clone()
                };
                let key = StandardMaterialHash::new(hash_stdmat(&mat));
                let std_mat = std_mat_cache.0
                    .entry(key)
                    .or_insert_with(|| mats.add(mat));

                ent_commands.try_insert(MeshMaterial3d(std_mat.clone()));
            }
            SpawnMaterial::None => (),
        }
        ent_commands.try_remove::<SpawnMaterial>();
    }
}

pub(crate) fn cleanup_materials(
    mats: Option<ResMut<SpawnMaterialHandles>>,
) {
    if let Some(mut mats) = mats {
        mats.0.clear();
    }
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
