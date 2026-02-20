use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;

use bevy::core_pipeline::Skybox;
use image::imageops::FilterType;

use crate::LevelState;
use crate::WorldMarkerEntity;

use super::states_sets::ProgramState;
use super::texutils::SkyboxTransform;
use super::texutils::convert_strip_to_cubemap;
use super::texutils::resize_for_quality;
use super::video::TextureQuality;
use super::video::VideoSettings;

pub struct SkyboxPlugin;

impl Plugin for SkyboxPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<SkyboxCache>()
            .add_systems(
                PreUpdate,
                    (
                        check_load_skybox,
                        check_load_reflection_probe,
                        check_skybox_setup,
                    )
                    .chain()
                    .run_if(resource_exists::<SkyboxSetup>)
                    .run_if(in_state(ProgramState::InGame))
            )
        ;
    }
}

/// Add this resource when you want to load a new skybox,
/// which is configured by a [SkyboxModel] component on a [Camera3d].
/// This can take several frames for larger images.
/// Resource removed when complete.
#[derive(Resource, Debug, Default, Reflect, PartialEq)]
#[reflect(Resource, Default)]
#[type_path = "game"]
pub struct SkyboxSetup {
    pub waiting_skybox: bool,
    pub waiting_reflections: bool,
}

#[derive(Resource, Default, Reflect)]
#[reflect(Resource, Default)]
#[type_path = "game"]
pub struct SkyboxCache {
    /// Cache of width (narrow dimension) to cubemapped image.
    mapped_skyboxes: HashMap<(Handle<Image>, u32), Handle<Image>>,
}

impl SkyboxCache {
    pub fn get_openexr_skybox(&mut self, images: &mut Assets<Image>, source_image: Handle<Image>, quality: TextureQuality, transform: SkyboxTransform)
    -> Handle<Image> {
        let side_res = match quality {
            TextureQuality::Low => 256,
            TextureQuality::Medium => 512,
            TextureQuality::High => 1024,
            TextureQuality::Ultra => 1200,
        };

        // Already cached?
        let key = (source_image.clone(), side_res);
        if let Some(skybox_image) = self.mapped_skyboxes.get(&key) {
            return skybox_image.clone();
        }

        let Some(source_image) = images.get(&source_image) else {
            // This can persist for many frames...
            return default()
        };

        let resized_image = if let Some(dyn_image) = resize_for_quality(
            source_image, side_res, side_res * 6, FilterType::Nearest) {
            &Image::from_dynamic(dyn_image, true,
                // since we convert it again just below
                RenderAssetUsages::MAIN_WORLD)
        } else {
            // Don't resize or let any error propagate.
            source_image
        };
        let image = convert_strip_to_cubemap(resized_image, transform).unwrap();
        let skybox_image = images.add(image);

        self.mapped_skyboxes.insert(key, skybox_image.clone());
        skybox_image
    }
}

/// Set this component when you wish to load a skybox asynchronously
/// (given that it may take a long time to load the texture).
/// The `Skybox::image` will be scaled to the desired video settings'
/// resolution, converted to a cubemap, then provide a Skybox directly
/// in place of the component.
/// If the reflection probe option is set, apply it with the given brightness.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct SkyboxModel{
    pub skybox: Skybox,
    pub xfrm: SkyboxTransform,
    pub with_reflection_probe: Option<(Entity, f32)>,
    pub enabled: bool,
}

fn check_skybox_setup(
    mut commands: Commands,
    setup: Res<SkyboxSetup>,
) {
    // Done?
    if *setup == SkyboxSetup::default() {
        commands.remove_resource::<SkyboxSetup>();
        commands.set_state(LevelState::Playing);
    }
}

/// Generic system to check for any [SkyboxModel] component, and if found,
/// make sure its image is loaded. Once loaded, convert it to a cubemap
/// and apply to the camera, then remove the component.
pub fn check_load_skybox(
    load_skybox_q: Query<(Entity, &SkyboxModel), Changed<SkyboxModel>>,
    mut commands: Commands,
    video_settings: Res<VideoSettings>,
    mut images: ResMut<Assets<Image>>,
    mut skyboxes: ResMut<SkyboxCache>,
    mut setup: ResMut<SkyboxSetup>,
) {
    // use bevy::render::render_resource::*;
    let Some((cam, SkyboxModel{ skybox, xfrm, with_reflection_probe, enabled })) = load_skybox_q.iter().next() else {
        setup.waiting_skybox = false;
        return
    };

    if !*enabled {
        commands.entity(cam).remove::<Skybox>();
        commands.entity(cam).remove::<LightProbe>();
        commands.entity(cam).remove::<EnvironmentMapLight>();
        setup.waiting_skybox = false;
        return;
    }

    let quality = video_settings.texture_quality;
    let skybox_image = skyboxes.get_openexr_skybox(&mut images, skybox.image.clone(), quality, *xfrm);

    if skybox_image == Handle::default() {
        // Still waiting.
        return;
    }

    let mut sky = skybox.clone();
    sky.image = skybox_image.clone();
    commands.entity(cam).insert(sky);
    setup.waiting_skybox = false;

    if let Some((ent, brightness)) = with_reflection_probe {
        commands.entity(*ent).insert((
            ReflectionProbeModel{
                image: skybox_image,
                brightness: *brightness,
            },
        ));
        setup.waiting_reflections = true;
    }
}

/// Set this component when you wish to load a reflection probe asynchronously
/// (given that it may take a long time to load the texture).
#[derive(Component)]
pub struct ReflectionProbeModel {
    pub image: Handle<Image>,
    pub brightness: f32,
}

/// Generic system to check for any LoadSkybox component, and if found,
/// make sure its image is loaded. Once loaded, convert it to a cubemap
/// and apply to the camera, then remove the component.
pub fn check_load_reflection_probe(
    load_probe_q: Query<(Entity, &ReflectionProbeModel), Changed<ReflectionProbeModel>>,
    world: Res<WorldMarkerEntity>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut setup: ResMut<SkyboxSetup>,
) {
    use bevy::render::render_resource::*;
    let Some((entity, ReflectionProbeModel{ image, brightness })) = load_probe_q.iter().next() else {
        setup.waiting_reflections = false;
        return
    };

    if *image == Handle::default() {
        return
    }

    if images.get(image).is_none() {
        // This can persist for many frames...
        return
    };

    // Make a solid diffuse map.
    let extents = Extent3d {
        width: 1,
        height: 1,
        depth_or_array_layers: 6
    };

    const B: u8 = 192;
    let mut diffuse = Image::new_fill(
        extents,
        TextureDimension::D2,
        &[
            B, B, B, 255,
            B, B, B, 255,
            B, B, B, 255,
            B, B, B, 255,
            B, B, B, 255,
            B, B, B, 255,
        ],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    diffuse.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        ..default()
    });
    let diffuse = images.add(diffuse);

    let reflection_image = images.get_mut(image).unwrap();

    reflection_image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        ..default()
    });

    commands.entity(entity).insert((
        LightProbe,
        EnvironmentMapLight {
            diffuse_map: diffuse.clone(),
            specular_map: image.clone(),
            intensity: *brightness,
            affects_lightmapped_mesh_diffuse: false,
            ..default()
        },
    ));

    commands.spawn((
        Name::new("Reflection Probe"),
        LightProbe,
        EnvironmentMapLight {
            diffuse_map: diffuse.clone(),
            specular_map: image.clone(),
            intensity: *brightness,
            affects_lightmapped_mesh_diffuse: false,
            ..default()
        },
        Transform::from_scale(Vec3::splat(100.0)),
        ChildOf(world.0),
    ));

    setup.waiting_reflections = false;
}
