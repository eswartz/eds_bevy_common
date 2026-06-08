use bevy::prelude::*;
use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    render::render_resource::*,
};

use crate::{ProgramState, SplitIntoCubes};

pub struct DetailNormalPlugin;

impl Plugin for DetailNormalPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<
            ExtendedMaterial<StandardMaterial, DetailNormalExtension>,
        >::default())
            .add_systems(
                PreUpdate,
                handle_assign_detail_normals.run_if(in_state(ProgramState::InGame)),
            )
        ;
    }
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
// This is only used for egui inspector.
#[derive(Component)]
struct DetailNormalExtension {
    // We need to ensure that the bindings of the base material and the extension do not conflict,
    // so we start from binding slot 100, leaving slots 0-99 for the base material.
    #[uniform(100)]
    uv_scale: Vec2,

    /// How much to blend (1.0 = all)
    #[uniform(100)]
    blend: f32,

    #[sampler(102)]
    #[texture(103)]
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
    #[reflect(ignore)]
    _webgl2_padding_12b: u32,
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
    pub blend: f32,
}
impl AssignDetailNormal {
    fn make_extension(&self, assets: &AssetServer) -> DetailNormalExtension {
         DetailNormalExtension {
            uv_scale: self.uv_scale,
            blend: self.blend,
            normal_texture: assets.load(&self.asset_path),

            _webgl2_padding_12b: default(),
        }
    }
}

impl Default for AssignDetailNormal {
    fn default() -> Self {
        Self {
            asset_path: default(),
            uv_scale: Vec2::splat(8.0),
            blend: 0.25,
        }
    }
}

fn handle_assign_detail_normals(
    mut commands: Commands,
    meshes_q: Query<
        (
            Entity,
            &AssignDetailNormal,
            Option<&MeshMaterial3d<StandardMaterial>>,
            Option<&CustomMaterialNormalExtension>,
        ),
        Or<(
            Without<SplitIntoCubes>,
            Changed<AssignDetailNormal>,
            Changed<CustomMaterialNormalExtension>,
        )>
    >,
    materials: Res<Assets<StandardMaterial>>,
    mut ext_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, DetailNormalExtension>>>,
    assets: Res<AssetServer>,
) {
    for (ent, dec, mat_opt, cust_opt) in meshes_q.iter() {
        let std_mat = match cust_opt {
            Some(cust) => {
                cust.std.clone()
            }
            None => {
                let Some(mat) = mat_opt else {
                    continue;
                };
                let Some(std) = materials.get(&mat.0) else {
                    continue;
                };
                std.clone()
            }
        };

        let extension = dec.make_extension(&assets);
        let ext_mat = ExtendedMaterial {
            base: std_mat.clone(),
            extension: extension.clone(),
        };

        let new_handle = ext_materials.add(ext_mat);

        let mut ent_commands = commands.entity(ent);
        ent_commands.remove::<MeshMaterial3d<StandardMaterial>>();
        ent_commands.insert((
            MeshMaterial3d(new_handle.clone()),
            // for egui inspector
            CustomMaterialNormalExtension {
                std: std_mat,
                ext: extension,
            },
        ));
    }
}

// This is only used for egui inspector.
#[derive(Component, Reflect, Clone)]
struct CustomMaterialNormalExtension {
    std: StandardMaterial,
    ext: DetailNormalExtension,
}
