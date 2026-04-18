
use bevy::prelude::*;
use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    render::render_resource::*,
};

use crate::{ProgramState, SplitIntoCubes};

pub struct DetailNormalPlugin;

impl Plugin for DetailNormalPlugin {
    fn build(&self, app: &mut App) {

        app
            .add_plugins(MaterialPlugin::<
                ExtendedMaterial<StandardMaterial, DetailNormalExtension>,
            >::default())
            .add_systems(
                Last,
                handle_assign_detail_normals
                .run_if(in_state(ProgramState::InGame)),
            )
        ;
    }
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Default)]
struct DetailNormalExtension {
    // We need to ensure that the bindings of the base material and the extension do not conflict,
    // so we start from binding slot 100, leaving slots 0-99 for the base material.
    #[uniform(100)]
    uv_scale: Vec2,

    #[sampler(101)]
    #[texture(102)]
    normal_texture: Handle<Image>,

    // // Web examples WebGL2 support: structs must be 16 byte aligned.
    // #[cfg(feature = "webgl2")]
    // #[uniform(100)]
    // _webgl2_padding_8b: u32,
    // #[cfg(feature = "webgl2")]
    // #[uniform(100)]
    // _webgl2_padding_12b: u32,
    // #[cfg(feature = "webgl2")]
    // #[uniform(100)]
    _webgl2_padding_12b: u32,
}
impl DetailNormalExtension {
    fn new(uv_scale: Vec2, normal_texture: Handle<Image>) -> Self {
        Self {
            uv_scale,
            normal_texture,
            ..default()
        }
    }
}

const SHADER_ASSET_PATH: &str = "common://shaders/normal_detail.wgsl";

impl MaterialExtension for DetailNormalExtension {
    fn fragment_shader() -> bevy::shader::ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn deferred_fragment_shader() -> bevy::shader::ShaderRef {
        SHADER_ASSET_PATH.into()
    }
}

#[derive(Component, Reflect, Clone)]
#[reflect(Component, Clone, Default)]
#[type_path = "game"]
/// A component on a Mesh that updates [StandardMaterial] with a subscaled texture.
pub struct AssignDetailNormal {
    pub asset_path: String,
    pub uv_scale: Vec2,
}

impl Default for AssignDetailNormal {
    fn default() -> Self {
        Self { asset_path: Default::default(), uv_scale: Vec2::splat(1.0) }
    }
}

fn handle_assign_detail_normals(
    mut commands: Commands,
    meshes_q: Query<(
        Entity,
        &MeshMaterial3d<StandardMaterial>,
        &AssignDetailNormal,
    ), Without<SplitIntoCubes>>,
    materials: Res<Assets<StandardMaterial>>,
    mut ext_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, DetailNormalExtension>>>,
    assets: Res<AssetServer>,
) {
    for (ent, mat, dec) in meshes_q.iter() {
        let Some(std_mat) = materials.get(&mat.0) else { continue };
        let std_mat: StandardMaterial = std_mat.clone();

        let ext_mat = ExtendedMaterial {
            base: std_mat.clone(),
            extension: DetailNormalExtension::new(
                dec.uv_scale,
                assets.load(&dec.asset_path),
            ),
        };

        let new_handle = ext_materials.add(ext_mat);

        commands.entity(ent).remove::<MeshMaterial3d<StandardMaterial>>();
        commands.entity(ent).insert(MeshMaterial3d(new_handle));
    }
}
