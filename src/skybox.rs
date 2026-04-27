use bevy::platform::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;

use bevy::core_pipeline::Skybox;
use bevy_asset_loader::prelude::*;
use image::imageops::FilterType;

use crate::WorldCamera;
use crate::ConfigureBeforePlaying;
use crate::CommonSkyboxAssets;

use super::states_sets::ProgramState;
use super::texutils::CubemapMapping;
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
                        check_load_reflection_probe.run_if(resource_exists::<SkyboxSetup>),
                        check_skybox_setup.run_if(resource_exists::<SkyboxSetup>),
                    )
                    .chain()
                    .run_if(in_state(ProgramState::InGame))
            )
            .configure_loading_state(
                LoadingStateConfig::new(ProgramState::Initializing)
                    .load_collection::<CommonSkyboxAssets>()
            )
        ;
    }
}

/// Set this component when you wish to load a skybox asynchronously
/// (given that it may take a long time to load the texture).
/// The `Skybox::image` will be scaled to the desired video settings'
/// resolution, converted to a cubemap, then provide a Skybox directly.
///
/// If the reflection probe option is set, apply it with the given brightness.
///
/// The [ConfigureBeforePlaying] component will be removed upon setup.
#[derive(Component, Reflect)]
#[require(ConfigureBeforePlaying)]
#[reflect(Component, Default)]
#[type_path = "game"]
pub struct SkyboxModel{
    /// The path to an [Image] asset (typically a `.ktx2` file).
    pub path: String,
    /// A loaded image. This is set automatically if needed, from [path].
    pub image: Option<Handle<Image>>,

    /// Toggle enablement of the skybox.
    pub enabled: bool,

    /// How the specified cubemap image is laid out.
    pub mapping: CubemapMapping,

    /// If set, apply [LightProbe] and [EnvironmentMapLight] to the given Camera entity.
    pub with_reflection_probe: Option<(Entity, f32)>,

    /// Scale factor applied to the skybox image.
    /// After applying this multiplier to the image samples, the resulting values should
    /// be in units of [cd/m^2](https://en.wikipedia.org/wiki/Candela_per_square_metre).
    /// see: [Skybox::brightness]
    pub brightness: f32,

    /// View space rotation applied to the skybox cubemap.
    /// This is useful for users who require a different axis, such as the Z-axis, to serve
    /// as the vertical axis.
    /// see: [Skybox::rotation]
    pub rotation: Quat,
}

impl Default for SkyboxModel {
    fn default() -> Self {
        Self {
            enabled: true,
            brightness: 500.0,
            path: default(),
            image: default(),
            mapping: default(),
            with_reflection_probe: default(),
            rotation: default(),
        }
    }
}

/// Add this resource when you want to load a new skybox,
/// which is configured by a [SkyboxModel] component on a [Camera3d].
/// This can take several frames for larger images.
/// Resource removed when complete.
#[derive(Resource, Debug, Default, Reflect, PartialEq)]
#[reflect(Resource, Default)]
#[type_path = "game"]
pub enum SkyboxSetup {
    WaitingSkybox,
    WaitingReflections,
    #[default]
    Finished,
}

#[derive(Resource, Default, Reflect)]
#[reflect(Resource, Default)]
#[type_path = "game"]
struct SkyboxCache {
    /// Cache of width (narrow dimension) to cubemapped image.
    mapped_skyboxes: HashMap<(Handle<Image>, u32), Handle<Image>>,
}

impl SkyboxCache {
    pub fn get_openexr_skybox(&mut self, images: &mut Assets<Image>, source_image: Handle<Image>, quality: TextureQuality, mapping: CubemapMapping)
    -> Result<Handle<Image>> {
        let side_res = match quality {
            TextureQuality::Low => 256,
            TextureQuality::Medium => 512,
            TextureQuality::High => 1024,
            TextureQuality::Ultra => 1200,
        };

        // Already cached?
        let key = (source_image.clone(), side_res);
        if let Some(skybox_image) = self.mapped_skyboxes.get(&key) {
            return Ok(skybox_image.clone());
        }

        let Some(source_image) = images.get(&source_image) else {
            // This can persist for many frames...
            return Ok(default())
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
        let image = convert_strip_to_cubemap(resized_image, mapping)?;
        let skybox_image = images.add(image);

        self.mapped_skyboxes.insert(key, skybox_image.clone());
        Ok(skybox_image)
    }
}


fn check_skybox_setup(
    mut commands: Commands,
    skybox_q: Query<Entity, (With<ConfigureBeforePlaying>, With<SkyboxModel>)>,
    setup: Res<SkyboxSetup>,
) {
    // Done?
    if *setup == SkyboxSetup::Finished {
        commands.remove_resource::<SkyboxSetup>();
        skybox_q.iter().for_each(|ent| {
            commands.entity(ent).remove::<ConfigureBeforePlaying>();
        });
    }
}

/// Generic system to check for any [SkyboxModel] component, and if found,
/// make sure its image is loaded. Once loaded, convert it to a cubemap
/// and apply to the camera, then remove the component.
fn check_load_skybox(
    mut params: ParamSet<(
        Query<&mut SkyboxModel>,
        Query<Entity, Changed<SkyboxModel>>
    )>,
    cam_q: Query<Entity, With<WorldCamera>>,
    has_skybox_q: Query<Has<Skybox>>,
    mut commands: Commands,
    video_settings: Res<VideoSettings>,
    assets: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut skyboxes: ResMut<SkyboxCache>,
) -> Result {
    let Ok(cam) = cam_q.single() else {
        // Keep waiting.
        return Ok(())
    };

    let any_changed = !params.p1().is_empty();

    // Do we have any skybox?
    let mut p0 = params.p0();
    let Some(model) = p0.iter_mut().next() else {
        // Keep waiting.
        return Ok(())
    };

    // Do we populate the components for skybox modeling?
    if !model.enabled {
        // Nope, remove 'em.
        commands.entity(cam).try_remove::<Skybox>();
        commands.entity(cam).try_remove::<LightProbe>();
        commands.entity(cam).try_remove::<EnvironmentMapLight>();
        commands.insert_resource(SkyboxSetup::Finished);
        return Ok(())
    }

    if let Ok(true) = has_skybox_q.get(cam) && !any_changed {
        // Already here and unchanged.
        return Ok(())
    }

    // Continue to await the asset loader. It may take a few frames.
    let quality = video_settings.texture_quality;
    let image = if let Some(image) = &model.image {
        image.clone()
    } else {
        assets.load::<Image>(&model.path)
    };
    let skybox_image = skyboxes.get_openexr_skybox(&mut images, image, quality, model.mapping)?;

    if skybox_image == Handle::default() {
        // Still waiting. Try again next tick.
        return Ok(());
    }

    // Here's the actual work. (Yes, I'm sure the above could be done in a better way.)
    commands.entity(cam).insert(Skybox {
        image: skybox_image.clone(),
        brightness: model.brightness,
        rotation: model.rotation,
    });

    // Do we want a reflection probe?
    let Some((ent, brightness)) = model.with_reflection_probe else {
        // Nope, all done!
        commands.insert_resource(SkyboxSetup::Finished);
        return Ok(())
    };

    // Set up reflection probe machinery.
    commands.entity(ent).insert((
        ReflectionProbeModel{
            image: skybox_image,
            brightness,
        },
    ));
    commands.insert_resource(SkyboxSetup::WaitingReflections);
    Ok(())
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
fn check_load_reflection_probe(
    load_probe_q: Query<(Entity, &ReflectionProbeModel), Changed<ReflectionProbeModel>>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut setup: ResMut<SkyboxSetup>,
) {
    use bevy::render::render_resource::*;
    let Some((cam_entity, ReflectionProbeModel{ image, brightness })) = load_probe_q.iter().next() else {
        *setup = SkyboxSetup::Finished;
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

    const B: u8 = 255;
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

    commands.entity(cam_entity).insert((
        LightProbe,
        EnvironmentMapLight {
            diffuse_map: diffuse.clone(),
            specular_map: image.clone(),
            intensity: *brightness,
            affects_lightmapped_mesh_diffuse: false,
            ..default()
        },
    ));

    // commands.spawn((
    //     Name::new("Reflection Probe"),
    //     LightProbe,
    //     EnvironmentMapLight {
    //         diffuse_map: diffuse.clone(),
    //         specular_map: image.clone(),
    //         intensity: *brightness,
    //         affects_lightmapped_mesh_diffuse: false,
    //         ..default()
    //     },
    //     xfrm,
    //     ChildOf(world.0),
    // ));

    *setup = SkyboxSetup::Finished;
}
