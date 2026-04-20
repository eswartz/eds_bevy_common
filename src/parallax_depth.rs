use bevy::prelude::*;

use crate::ProgramState;

pub struct ParallaxDepthPlugin;

impl Plugin for ParallaxDepthPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(
                Update,
                handle_parallax_depth.run_if(in_state(ProgramState::InGame)),
            )
        ;
    }
}

#[derive(Component, Reflect)]
#[reflect(Component, Default)]
#[type_path = "game"]
pub struct ParallaxDepth {
    pub depth_scale: f32,
    pub mapping_method: ParallaxMappingMethod,
    pub max_layer_count: u32,
}

impl Default for ParallaxDepth {
    fn default() -> Self {
        Self {
            depth_scale: 0.25,
            mapping_method: ParallaxMappingMethod::Occlusion,
            max_layer_count: 4,
        }
    }
}

fn handle_parallax_depth(
    mut meshes_q: Query<(
        Entity,
        &mut MeshMaterial3d<StandardMaterial>,
        &ParallaxDepth,
    ),
    Or<(Added<ParallaxDepth>, Changed<ParallaxDepth>)>
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (ent, mut mat, parallax_depth) in meshes_q.iter_mut() {
        let Some(std_mat) = materials.get(&mat.0) else {
            continue;
        };

        if std_mat.parallax_depth_scale != parallax_depth.depth_scale
        || std_mat.parallax_mapping_method != parallax_depth.mapping_method
        || std_mat.max_parallax_layer_count != parallax_depth.max_layer_count as f32
        || std_mat.depth_map != std_mat.normal_map_texture {
            dbg!(ent);
            let mut std_mat = std_mat.clone();
            std_mat.parallax_depth_scale = parallax_depth.depth_scale;
            std_mat.parallax_mapping_method = parallax_depth.mapping_method;
            std_mat.max_parallax_layer_count = parallax_depth.max_layer_count as f32;
            std_mat.depth_map = std_mat.normal_map_texture.clone();

            let new_mat = materials.add(std_mat);
            mat.0 = new_mat;
        };
    }
}
