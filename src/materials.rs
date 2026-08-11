use std::hash::Hash;

use bevy::image::ImageAddressMode;
use bevy::image::ImageLoaderSettings;
use bevy::image::ImageSampler;
use bevy::image::ImageSamplerDescriptor;
use bevy::math::FloatOrd;
use bevy::mesh::PlaneMeshBuilder;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use rustc_hash::FxHashMap;
use wgpu::Face;
use wgpu::TextureFormat;

use crate::prelude::LevelState;
use crate::prelude::MeshQuality;
use crate::prelude::VideoSettings;
use crate::prelude::create_uvmapped_mesh_scaled;

pub struct MaterialsPlugin;

impl Plugin for MaterialsPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(SpawnMaterialHandles::default())
            .insert_resource(SpawnMeshHandles::default())
            .add_message::<RefreshImages>()
            .add_systems(
                PreUpdate,
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
                FloatOrd(vec3.x).hash(state);
                FloatOrd(vec3.y).hash(state);
                FloatOrd(vec3.z).hash(state);
            }
            SpawnShapeKind::Sphere(rad) => {
                FloatOrd(*rad).hash(state);
            }
            SpawnShapeKind::Plane(vec2) => {
                FloatOrd(vec2.x).hash(state);
                FloatOrd(vec2.y).hash(state);
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
                                    MeshQuality::Medium => (16, 10),
                                    MeshQuality::High => (20, 14),
                                    MeshQuality::Ultra => (24, 18),
                                }
                            }
                        };
                        let mut mesh = Sphere::new(*rad).mesh().uv(sectors, stacks);
                        mesh.compute_smooth_normals();
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
struct SpawnMaterialHandles (
    HashMap<StandardMaterialHash, Handle<StandardMaterial>>,
);

#[derive(Debug, Clone, PartialEq, Eq,Hash)]
pub struct StandardMaterialHash(String);

impl StandardMaterialHash {
    pub fn new(mat: String) -> Self {
        Self(mat)
    }
}

pub fn hash_color(color: Color) -> String {
    format!("{}", color.to_linear().as_u32())
}
pub fn hash_image(image: &Option<Handle<Image>>) -> String {
    if let Some(image) = image {
        format!("{}", image.id())
    } else {
        default()
    }
}

pub fn hash_stdmat(m: &StandardMaterial) -> String {
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

fn handle_spawn_material(
    mut commands: Commands,
    mut mats: If<ResMut<Assets<StandardMaterial>>>,
    mat_q: Query<(Entity, &SpawnMaterial), (Without<TextureSources>, Without<SpawnShape>)>,
    mut mat_cache: If<ResMut<SpawnMaterialHandles>>,
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
                let mat_hash = StandardMaterialHash(hash_stdmat(&mat));
                let std_mat = (**mat_cache).0.entry(mat_hash)
                    .or_insert_with(|| mats.add(mat));
                ent_commands.try_insert(MeshMaterial3d(std_mat.clone()));
            }
            SpawnMaterial::None => (),
        }
        ent_commands.try_remove::<SpawnMaterial>();
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
