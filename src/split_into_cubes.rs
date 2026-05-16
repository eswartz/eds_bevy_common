use avian3d::math::Vector;
use bevy::asset::RenderAssetUsages;
use bevy::camera::primitives::Aabb;
use bevy::mesh::VertexAttributeValues;

use std::ops::Mul;

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::AssignDetailNormal;
use crate::ConfigureBeforePlaying;
use crate::LevelState;

pub struct SplitIntoCubesPlugin;

impl Plugin for SplitIntoCubesPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(
                PreUpdate,
                handle_split_into_cubes
                    .run_if(in_state(LevelState::Configuring))
            )

        ;
    }
}

/// Mark a mesh that needs to be split into cubes (and then [TrimeshFromMesh]).
///
/// If set to 0.0, this is ignored.
#[derive(Component, Clone, Reflect)]
#[require(ConfigureBeforePlaying)]
#[reflect(Component, Clone, Default)]
#[type_path = "game"]
pub struct SplitIntoCubes {
    pub size: f32,
}

impl Default for SplitIntoCubes {
    fn default() -> Self {
        Self{ size: 128.0 }
    }
}

#[expect(clippy::unwrap_used, reason = "shouldn't fail unless other stuff is falling over, want to see panic")]
fn handle_split_into_cubes(
    split_q: Query<(
        Entity,
        &SplitIntoCubes,
        &Mesh3d,
        &MeshMaterial3d<StandardMaterial>,
        &Transform,
        &Aabb,
        Option<&Name>,
        Option<&Friction>,
        Option<&RigidBody>,
        Option<&CollisionLayers>,
        Option<&AssignDetailNormal>,
    ), Added<Aabb>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut commands: Commands,
) {
    for (
        ent, split, mesh, mat, xfrm, aabb,
        name_opt, friction_opt, rigid_opt, layers_opt, adn_opt
    ) in split_q.iter() {
        let Some(mesh) = meshes.get(&mesh.0) else { continue };
        let mesh = mesh.clone();

        let full_extents = aabb.half_extents.mul(2.0).to_vec3() * xfrm.scale;
        let (xn, yn, zn) = if split.size == 0. {
            (1, 1, 1)
        } else {
            let split = split.size.max(64.);
            let zn = (full_extents.z / split).ceil() as i32;
            let yn = (full_extents.y / split).ceil() as i32;
            let xn = (full_extents.x / split).ceil() as i32;
            (xn, yn, zn)
        };

        let zs = full_extents.z / zn as f32;
        let ys = full_extents.y / yn as f32;
        let xs = full_extents.x / xn as f32;

        let cube_half_size = Vec3::new(xs, ys, zs);

        let root = ent;
        let mut count = 0;

        let aabb_min = aabb.min().to_vec3();
        for zi in 0..zn {
            let z0 = aabb_min.z + zs * zi as f32;
            for yi in 0..yn {
                let y0 = aabb_min.y + ys * yi as f32;
                for xi in 0..xn {
                    let x0 = aabb_min.x + xs * xi as f32;

                    let cube_center = Vec3::new(x0, y0, z0) + cube_half_size;

                    if let Some((partial_mesh, indices, vertices)) = extract_mesh_cube(&mesh, cube_center, cube_half_size) {
                        let mut ent_commands = commands.spawn((
                            ChildOf(root),
                            Mesh3d(meshes.add(partial_mesh)),

                            Name::new(if let Some(name) = name_opt {
                                format!("{name} split {xi}.{yi}.{zi}")
                            } else {
                                "split".to_string()
                            }),
                            ColliderConstructor::TrimeshWithConfig {
                                indices, vertices,
                                flags: TrimeshFlags::FIX_INTERNAL_EDGES
                            },
                        ));

                        ent_commands.insert(mat.clone());

                        if let Some(c) = friction_opt {
                            ent_commands.insert(*c);
                        }
                        if let Some(c) = rigid_opt {
                            ent_commands.insert(*c);
                        }
                        if let Some(c) = layers_opt {
                            ent_commands.insert(*c);
                        }
                        if let Some(c) = adn_opt {
                            ent_commands.insert(c.clone());
                        }

                        count += 1;
                    }
                }
            }
        }

        info!("Split {ent} into {count} cubes");

        // Remove the original large object.
        let mut ent_commands = commands.entity(ent);
        ent_commands.remove::<Mesh3d>();
        ent_commands.remove::<MeshMaterial3d<StandardMaterial>>();
        ent_commands.remove::<RigidBody>();
        ent_commands.remove::<ColliderConstructor>();
        ent_commands.remove::<SplitIntoCubes>();

        ent_commands.remove::<ConfigureBeforePlaying>();
    }
}

#[expect(clippy::unwrap_used, reason = "shouldn't fail unless other stuff is falling over, want to see panic")]
#[expect(clippy::unwrap_in_result, reason = "shouldn't fail unless other stuff is falling over, want to see panic")]
fn extract_mesh_cube(mesh: &Mesh, center: Vec3, half_size: Vec3) -> Option<(Mesh, Vec<[u32; 3]>, Vec<Vector>)> {
    let inds = mesh.indices()?;

    let full_pos = mesh.attribute(Mesh::ATTRIBUTE_POSITION)?.as_float3().unwrap();
    let full_normals = mesh.attribute(Mesh::ATTRIBUTE_NORMAL)?.as_float3().unwrap();
    let full_uvs = match mesh.attribute(Mesh::ATTRIBUTE_UV_0)? {
        VertexAttributeValues::Float32x2(values) => values,
        _ => return None
    };

    let transform_pt = |ptarr: [f32; 3]| -> [f32; 3] {
        // Vec3::from_array(ptarr)
        ptarr
    };

    let mut pos = vec![];
    let mut normals = vec![];
    let mut uvs = vec![];
    let mut indices = vec![];
    #[expect(clippy::indexing_slicing, reason = "shouldn't fail unless other stuff is falling over, want to see panic")]
    for [ind0, ind1, ind2] in inds.iter(). array_chunks::<3>() {
        let pos0 = full_pos[ind0];
        let pos1 = full_pos[ind1];
        let pos2 = full_pos[ind2];
        if contains_pt(&pos0, center, half_size)
        || contains_pt(&pos1, center, half_size)
        || contains_pt(&pos2, center, half_size) {
            let l = pos.len() as u32;
            indices.push([l, l + 1, l + 2]);

            pos.push(transform_pt(pos0));
            pos.push(transform_pt(pos1));
            pos.push(transform_pt(pos2));

            normals.push(full_normals[ind0]);
            normals.push(full_normals[ind1]);
            normals.push(full_normals[ind2]);

            uvs.push(full_uvs[ind0]);
            uvs.push(full_uvs[ind1]);
            uvs.push(full_uvs[ind2]);
        }
    }

    if pos.is_empty() {
        return None
    }

    let mut mesh = Mesh::new(wgpu::PrimitiveTopology::TriangleList, RenderAssetUsages::all())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, VertexAttributeValues::Float32x3(pos.clone()))
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, VertexAttributeValues::Float32x3(normals))
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, VertexAttributeValues::Float32x2(uvs));

    if let Err(err) = mesh.generate_tangents() {
        warn!("failed to generate tangents: {err}");
    }

    // Some(mesh)

    let positions = pos.into_iter().map(Vec3::from_array).collect::<Vec<_>>();
    Some((mesh, indices, positions))
}

fn contains_pt(pt: &[f32; 3], center: Vec3, half_size: Vec3) -> bool {
    pt[0] >= center.x - half_size.x && pt[0] <= center.x + half_size.x
    && pt[1] >= center.y - half_size.y && pt[1] <= center.y + half_size.y
    && pt[2] >= center.z - half_size.z && pt[2] <= center.z + half_size.z
}
