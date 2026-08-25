/// This defines a model for Bevy [StandardMaterial] but has these advantages:
///
/// * Reflectable
/// * Defaultable
/// * PartialEq-able
///
/// To do this, several types are mirrored and all [Handle<Image>] are
/// converted to [TextureSource].
use std::hash::Hash;

use bevy::image::ImageAddressMode;
use bevy::material::OpaqueRendererMethod;
use bevy::math::Affine2;
use bevy::mesh::UvChannel;
use bevy::prelude::*;
use bevy::reflect::enums::Enum;
use wgpu::Face;
use wgpu::TextureFormat;

use crate::prelude::*;

/// Reflectable version of Face.
#[derive(Debug, Default, Clone, Reflect, PartialEq, Eq, Hash)]
#[reflect(Default, Clone)]
#[type_path = "game"]
pub enum MaterialCullFace {
    None,
    Front,
    #[default]
    Back,
}

impl From<Option<Face>> for MaterialCullFace {
    fn from(value: Option<Face>) -> Self {
        match value {
            Some(Face::Back) => Self::Back,
            Some(Face::Front) => Self::Front,
            None => Self::None,
        }
    }
}

impl From<MaterialCullFace> for Option<Face> {
    fn from(value: MaterialCullFace) -> Self {
        match value {
            MaterialCullFace::None => None,
            MaterialCullFace::Front => Some(Face::Front),
            MaterialCullFace::Back => Some(Face::Back),
        }
    }
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
    #[default] // different for script!
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
#[derive(Debug, Clone, Reflect, PartialEq, Eq, Hash)]
#[reflect(Clone)]
#[type_path = "game"]
pub enum TextureSource {
    Load { path: String, params: TextureParams },
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

impl TextureSource {
    /// For `Load` variants, load the image from the AssetServer if needed.
    /// Otherwise return the handle as-is.
    pub fn get_handle(&self, assets: &AssetServer, assume_is_srgb: bool) -> Handle<Image> {
        match self {
            TextureSource::Load { path, params } => assets
                .load_builder()
                .with_settings(make_image_loader_settings_applier(
                    params.clone(),
                    assume_is_srgb,
                ))
                .load(path),
            TextureSource::Handle(handle) => handle.clone(),
        }
    }
}

/// Material model, a superset of [StandardMaterial] with all fields
/// available independent of bevy features, with Eq/Hash, and with all textures
/// replaced with [`TextureSource`].
#[derive(Debug, Clone, Reflect, PartialEq)]
#[reflect(Default, Clone)]
#[type_path = "game"]
pub struct StandardMaterialModel {
    pub base_color: Color,
    pub base_color_channel: UvChannel,
    pub base_color_texture: Option<TextureSource>,
    pub emissive: LinearRgba,
    pub emissive_exposure_weight: Float32,
    pub emissive_channel: UvChannel,
    pub emissive_texture: Option<TextureSource>,
    pub perceptual_roughness: Float32,
    pub metallic: Float32,
    pub metallic_roughness_channel: UvChannel,
    pub metallic_roughness_texture: Option<TextureSource>,
    pub reflectance: Float32,
    pub specular_tint: Color,
    pub diffuse_transmission: Float32,
    pub diffuse_transmission_channel: UvChannel,
    pub diffuse_transmission_texture: Option<TextureSource>,
    pub specular_transmission: Float32,
    pub specular_transmission_channel: UvChannel,
    pub specular_transmission_texture: Option<TextureSource>,
    pub thickness: Float32,
    pub thickness_channel: UvChannel,
    pub thickness_texture: Option<TextureSource>,
    pub ior: Float32,
    pub attenuation_distance: Float32,
    pub attenuation_color: Color,
    pub normal_map_channel: UvChannel,
    pub normal_map_texture: Option<TextureSource>,
    pub flip_normal_map_y: bool,
    pub occlusion_channel: UvChannel,
    pub occlusion_texture: Option<TextureSource>,
    pub specular_channel: UvChannel,
    pub specular_texture: Option<TextureSource>,
    pub specular_tint_channel: UvChannel,
    pub specular_tint_texture: Option<TextureSource>,
    pub clearcoat: Float32,
    pub clearcoat_channel: UvChannel,
    pub clearcoat_texture: Option<TextureSource>,
    pub clearcoat_perceptual_roughness: Float32,
    pub clearcoat_roughness_channel: UvChannel,
    pub clearcoat_roughness_texture: Option<TextureSource>,
    pub clearcoat_normal_channel: UvChannel,
    pub clearcoat_normal_texture: Option<TextureSource>,
    pub anisotropy_strength: Float32,
    pub anisotropy_rotation: Float32,
    pub anisotropy_channel: UvChannel,
    pub anisotropy_texture: Option<TextureSource>,
    pub double_sided: bool,
    pub cull_mode: MaterialCullFace,
    pub unlit: bool,
    pub fog_enabled: bool,
    pub alpha_mode: AlphaMode,
    pub depth_bias: Float32,
    pub depth_map: Option<TextureSource>,
    pub parallax_depth_scale: Float32,
    pub parallax_mapping_method: ParallaxMappingMethod,
    pub max_parallax_layer_count: Float32,
    pub lightmap_exposure: Float32,
    pub opaque_render_method: OpaqueRendererMethod,
    pub deferred_lighting_pass_id: u8,
    pub uv_transform: Affine2,
}

impl Default for StandardMaterialModel {
    fn default() -> Self {
        let def_mat = StandardMaterial::default();
        Self {
            base_color: def_mat.base_color.clone(),
            base_color_channel: default(),
            base_color_texture: None,
            emissive: def_mat.emissive.clone(),
            emissive_exposure_weight: default(),
            emissive_channel: default(),
            emissive_texture: None,
            perceptual_roughness: def_mat.perceptual_roughness.into(),
            metallic: def_mat.metallic.into(),
            metallic_roughness_channel: default(),
            metallic_roughness_texture: None,
            reflectance: def_mat.reflectance.into(),
            specular_tint: def_mat.specular_tint.clone(),
            diffuse_transmission: def_mat.diffuse_transmission.into(),
            diffuse_transmission_channel: default(),
            diffuse_transmission_texture: None,
            specular_transmission: def_mat.specular_transmission.into(),
            specular_transmission_channel: default(),
            specular_transmission_texture: None,
            thickness: def_mat.thickness.into(),
            thickness_channel: default(),
            thickness_texture: None,
            ior: def_mat.ior.into(),
            attenuation_distance: def_mat.attenuation_distance.into(),
            attenuation_color: def_mat.attenuation_color.clone(),
            normal_map_channel: default(),
            normal_map_texture: None,
            flip_normal_map_y: def_mat.flip_normal_map_y.clone(),
            occlusion_channel: default(),
            occlusion_texture: None,
            specular_channel: default(),
            specular_texture: None,
            specular_tint_channel: default(),
            specular_tint_texture: None,
            clearcoat: def_mat.clearcoat.into(),
            clearcoat_channel: default(),
            clearcoat_texture: None,
            clearcoat_perceptual_roughness: def_mat.clearcoat_perceptual_roughness.into(),
            clearcoat_roughness_channel: default(),
            clearcoat_roughness_texture: None,
            clearcoat_normal_channel: default(),
            clearcoat_normal_texture: None,
            anisotropy_strength: def_mat.anisotropy_strength.into(),
            anisotropy_rotation: def_mat.anisotropy_rotation.into(),
            anisotropy_channel: default(),
            anisotropy_texture: None,
            double_sided: def_mat.double_sided.clone(),
            cull_mode: def_mat.cull_mode.into(),
            unlit: def_mat.unlit.clone(),
            fog_enabled: def_mat.fog_enabled.clone(),
            alpha_mode: def_mat.alpha_mode.clone(),
            depth_bias: def_mat.depth_bias.into(),
            depth_map: None,
            parallax_depth_scale: def_mat.parallax_depth_scale.into(),
            parallax_mapping_method: def_mat.parallax_mapping_method.clone(),
            max_parallax_layer_count: def_mat.max_parallax_layer_count.into(),
            lightmap_exposure: def_mat.lightmap_exposure.into(),
            opaque_render_method: def_mat.opaque_render_method.clone(),
            deferred_lighting_pass_id: def_mat.deferred_lighting_pass_id.clone(),
            uv_transform: def_mat.uv_transform.clone(),
        }
    }
}

impl Eq for StandardMaterialModel {}

pub fn hash_color<H: std::hash::Hasher>(color: Color, state: &mut H) {
    let l = color.to_linear();
    Float32(l.red).hash(state);
    Float32(l.green).hash(state);
    Float32(l.blue).hash(state);
    Float32(l.alpha).hash(state);
}

pub fn hash_channel<H: std::hash::Hasher>(channel: UvChannel, state: &mut H) {
    channel.variant_index().hash(state);
}

pub fn hash_affine2<H: std::hash::Hasher>(aff: Affine2, state: &mut H) {
    Float32(aff.matrix2.determinant()).hash(state);
    Float32(aff.translation.x).hash(state);
    Float32(aff.translation.y).hash(state);
}

impl Hash for StandardMaterialModel {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        hash_color(self.base_color, state);
        hash_channel(self.base_color_channel.clone(), state);
        self.base_color_texture.hash(state);
        hash_color(self.emissive.into(), state);
        self.emissive_exposure_weight.hash(state);
        hash_channel(self.emissive_channel.clone(), state);
        self.emissive_texture.hash(state);
        self.perceptual_roughness.hash(state);
        self.metallic.hash(state);
        hash_channel(self.metallic_roughness_channel.clone(), state);
        self.metallic_roughness_texture.hash(state);
        self.reflectance.hash(state);
        hash_color(self.specular_tint, state);
        self.diffuse_transmission.hash(state);
        hash_channel(self.diffuse_transmission_channel.clone(), state);
        self.diffuse_transmission_texture.hash(state);
        self.specular_transmission.hash(state);
        hash_channel(self.specular_transmission_channel.clone(), state);
        self.specular_transmission_texture.hash(state);
        self.thickness.hash(state);
        hash_channel(self.thickness_channel.clone(), state);
        self.thickness_texture.hash(state);
        self.ior.hash(state);
        self.attenuation_distance.hash(state);
        hash_color(self.attenuation_color, state);
        hash_channel(self.normal_map_channel.clone(), state);
        self.normal_map_texture.hash(state);
        self.flip_normal_map_y.hash(state);
        hash_channel(self.occlusion_channel.clone(), state);
        self.occlusion_texture.hash(state);
        hash_channel(self.specular_channel.clone(), state);
        self.specular_texture.hash(state);
        hash_channel(self.specular_tint_channel.clone(), state);
        self.specular_tint_texture.hash(state);
        self.clearcoat.hash(state);
        hash_channel(self.clearcoat_channel.clone(), state);
        self.clearcoat_texture.hash(state);
        self.clearcoat_perceptual_roughness.hash(state);
        hash_channel(self.clearcoat_roughness_channel.clone(), state);
        self.clearcoat_roughness_texture.hash(state);
        hash_channel(self.clearcoat_normal_channel.clone(), state);
        self.clearcoat_normal_texture.hash(state);
        self.anisotropy_strength.hash(state);
        self.anisotropy_rotation.hash(state);
        hash_channel(self.anisotropy_channel.clone(), state);
        self.anisotropy_texture.hash(state);
        self.double_sided.hash(state);
        self.cull_mode.hash(state);
        self.unlit.hash(state);
        self.fog_enabled.hash(state);
        (self.alpha_mode.variant_index()).hash(state);
        self.depth_bias.hash(state);
        self.depth_map.hash(state);
        self.parallax_depth_scale.hash(state);
        (self.parallax_mapping_method.variant_index()).hash(state);
        self.max_parallax_layer_count.hash(state);
        self.lightmap_exposure.hash(state);
        (self.opaque_render_method.variant_index()).hash(state);
        self.deferred_lighting_pass_id.hash(state);
        hash_affine2(self.uv_transform, state);
    }
}

impl StandardMaterialModel {
    pub fn into_standard_material(&self, assets: &AssetServer) -> StandardMaterial {
        #[allow(unused_mut, reason = "conditional edits")]
        let mut std_mat = StandardMaterial {
            base_color: self.base_color,
            base_color_channel: self.base_color_channel.clone(),
            base_color_texture: self
                .base_color_texture
                .as_ref()
                .map(|i| i.get_handle(assets, true)),
            emissive: self.emissive,
            emissive_exposure_weight: self.emissive_exposure_weight.into(),
            emissive_channel: self.emissive_channel.clone(),
            emissive_texture: self
                .emissive_texture
                .as_ref()
                .map(|i| i.get_handle(assets, true)),
            perceptual_roughness: self.perceptual_roughness.into(),
            metallic: self.metallic.into(),
            metallic_roughness_channel: self.metallic_roughness_channel.clone(),
            metallic_roughness_texture: self
                .metallic_roughness_texture
                .as_ref()
                .map(|i| i.get_handle(assets, false)),
            reflectance: self.reflectance.into(),
            specular_tint: self.specular_tint,
            diffuse_transmission: self.diffuse_transmission.into(),
            specular_transmission: self.specular_transmission.into(),
            thickness: self.thickness.into(),
            ior: self.ior.into(),
            attenuation_distance: self.attenuation_distance.into(),
            attenuation_color: self.attenuation_color,
            normal_map_channel: self.normal_map_channel.clone(),
            normal_map_texture: self
                .normal_map_texture
                .as_ref()
                .map(|i| i.get_handle(assets, false)),
            flip_normal_map_y: self.flip_normal_map_y,
            occlusion_channel: self.occlusion_channel.clone(),
            occlusion_texture: self
                .occlusion_texture
                .as_ref()
                .map(|i| i.get_handle(assets, false)),
            clearcoat: self.clearcoat.into(),
            clearcoat_perceptual_roughness: self.clearcoat_perceptual_roughness.into(),
            anisotropy_strength: self.anisotropy_strength.into(),
            anisotropy_rotation: self.anisotropy_rotation.into(),
            double_sided: self.double_sided,
            cull_mode: self.cull_mode.clone().into(),
            unlit: self.unlit,
            fog_enabled: self.fog_enabled,
            alpha_mode: self.alpha_mode,
            depth_bias: self.depth_bias.into(),
            depth_map: self.depth_map.as_ref().map(|i| i.get_handle(assets, false)),
            parallax_depth_scale: self.parallax_depth_scale.into(),
            parallax_mapping_method: self.parallax_mapping_method,
            max_parallax_layer_count: self.max_parallax_layer_count.into(),
            lightmap_exposure: self.lightmap_exposure.into(),
            opaque_render_method: self.opaque_render_method,
            deferred_lighting_pass_id: self.deferred_lighting_pass_id,
            uv_transform: self.uv_transform,

            ..default()
        };

        #[cfg(feature = "pbr_transmission_textures")]
        {
            std_mat.diffuse_transmission_channel = self.diffuse_transmission_channel.clone();
            std_mat.diffuse_transmission_texture = self
                .diffuse_transmission_texture
                .as_ref()
                .map(|i| i.get_handle(assets, false));
            std_mat.specular_transmission_channel = self.specular_transmission_channel.clone();
            std_mat.specular_transmission_texture = self
                .specular_transmission_texture
                .as_ref()
                .map(|i| i.get_handle(assets, false));
            std_mat.thickness_channel = self.thickness_channel.clone();
            std_mat.thickness_texture = self
                .thickness_texture
                .as_ref()
                .map(|i| i.get_handle(assets, false));
        }
        #[cfg(feature = "pbr_specular_textures")]
        {
            std_mat.specular_channel = self.specular_channel.clone();
            std_mat.specular_texture = self
                .specular_texture
                .as_ref()
                .map(|i| i.get_handle(assets, false));
            std_mat.specular_tint_channel = self.specular_tint_channel.clone();
            std_mat.specular_tint_texture = self
                .specular_tint_texture
                .as_ref()
                .map(|i| i.get_handle(assets, false));
        }
        #[cfg(feature = "pbr_multi_layer_material_textures")]
        {
            std_mat.clearcoat_channel = self.clearcoat_channel.clone();
            std_mat.clearcoat_texture = self
                .clearcoat_texture
                .as_ref()
                .map(|i| i.get_handle(assets, false));
            std_mat.clearcoat_roughness_channel = self.clearcoat_roughness_channel.clone();
            std_mat.clearcoat_roughness_texture = self
                .clearcoat_roughness_texture
                .as_ref()
                .map(|i| i.get_handle(assets, false));
            std_mat.clearcoat_normal_channel = self.clearcoat_normal_channel.clone();
            std_mat.clearcoat_normal_texture = self
                .clearcoat_normal_texture
                .as_ref()
                .map(|i| i.get_handle(assets, false));
        }
        #[cfg(feature = "pbr_anisotropy_texture")]
        {
            std_mat.anisotropy_channel = self.anisotropy_channel.clone();
            std_mat.anisotropy_texture = self
                .anisotropy_texture
                .as_ref()
                .map(|i| i.get_handle(assets, false));
        }
        std_mat
    }
}
