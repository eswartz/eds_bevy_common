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
                Update,
                handle_assign_detail_normals.run_if(in_state(ProgramState::InGame)),
            )
            .add_systems(
                Update,
                sync_extended_material.run_if(in_state(ProgramState::InGame)),
            );
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
            &MeshMaterial3d<StandardMaterial>,
            &AssignDetailNormal,
        ),
        Without<SplitIntoCubes>,
    >,
    materials: Res<Assets<StandardMaterial>>,
    mut ext_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, DetailNormalExtension>>>,
    assets: Res<AssetServer>,
) {
    for (ent, mat, dec) in meshes_q.iter() {
        let Some(std_mat) = materials.get(&mat.0) else {
            continue;
        };
        let std_mat: StandardMaterial = std_mat.clone();

        let extension = dec.make_extension(&assets);
        let ext_mat = ExtendedMaterial {
            base: std_mat.clone(),
            extension: extension.clone(),
        };

        let new_handle = ext_materials.add(ext_mat);

        commands
            .entity(ent)
            .remove::<MeshMaterial3d<StandardMaterial>>();
        commands.entity(ent).insert((
            MeshMaterial3d(new_handle.clone()),
            // for egui inspector
            CustomMaterialNormalExtension {
                std: mat.clone(),
                ext: extension,
                // base: new_handle,
            },
        ));
    }
}

// This is only used for egui inspector.
#[derive(Component, Reflect, Clone)]
struct CustomMaterialNormalExtension {
    std: MeshMaterial3d<StandardMaterial>,
    ext: DetailNormalExtension,
    // #[reflect(ignore)]
    // base: Handle<ExtendedMaterial<StandardMaterial, DetailNormalExtension>>,
}

/// When modifying materials via [CustomMaterialNormalExtension],
/// update the original handle.
fn sync_extended_material() {}
// fn sync_extended_material(
//     mut commands: Commands,
//     custom_q: Query<&CustomMaterialNormalExtension, (With<CustomMaterialNormalExtension>, Changed<CustomMaterialNormalExtension>)>,
//     // mut ext_q: Query<(&mut Mesh3d, &mut MeshMaterial3d<ExtendedMaterial<StandardMaterial, DetailNormalExtension>>)>,
//     mut mesh_q: Query<(Entity, &mut Mesh3d, &MeshMaterial3d<ExtendedMaterial<StandardMaterial, DetailNormalExtension>>)>,
//     mut std_materials: ResMut<Assets<StandardMaterial>>,
//     mut ext_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, DetailNormalExtension>>>,
//     mut meshes: ResMut<Assets<Mesh>>,
// ) {
//     for custom_ext in custom_q.iter() {
//         if let Some(custom) = ext_materials.get_mut(&custom_ext.base) {
//             let ext_mat = ExtendedMaterial {
//                 base: custom.base.clone(),
//                 extension: custom.extension.clone(),
//             };
//             // custom.set = ext_mat;

//             // commands.write_message(AssetEvent::Modified{ id: custom_ext.base.id() });
//             // let new_mat = ext_materials.add(ext_mat);

//             for (ent, mut mesh_h, mat) in mesh_q.iter_mut() {
//             //     // if let Some(mesh) = meshes.get(&mesh_h.0) {
//             //     // ??? how to force reload???
//             //         // commands.entity(ent).insert(mesh_h.clone());
//             //         mesh_h.set_changed();
//             //         // commands.entity(ent).remove::<MeshMaterial3d<ExtendedMaterial<StandardMaterial, DetailNormalExtension>>>();
//             commands.entity(ent).insert(MeshMaterial3d(std_materials.add(custom.base.clone())));
//             //         commands.entity(ent).insert(MeshMaterial3d(new_mat.clone()));
//             //         // commands.entity(ent).insert(CustomMaterialNormalExtension {
//             //         //     base: new_mat.clone(),
//             //         //     .. custom_ext.clone()
//             //         // });
//             //     // }
//             }

//             info!("updated material");
//         } else {
//             warn!("could not find {:?}", &custom_ext.base);
//         }
//         // if let Some(custom) = ext_materials.get(&custom_ext.base) {
//         //     let ext_mat = ExtendedMaterial {
//         //         base: custom.base.clone(),
//         //         extension: custom.extension.clone(),
//         //     };
//         //     let new_handle = ext_materials.add(ext_mat);
//         //     for mut ext in ext_q.iter_mut() {
//         //         ext.0 = new_handle.clone();
//         //     }
//         //     info!("updated material on {}", ext_q.count());
//         // } else {
//         //     warn!("could not find {:?}", &custom_ext.base);
//         // }
//     }
// }
