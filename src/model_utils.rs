
use core::f32;
use std::collections::HashMap;

use bevy::camera::primitives::Aabb;
use bevy::prelude::*;
use bevy::mesh::Indices;
use bevy::mesh::MeshTrianglesError;
use rustc_hash::FxHashMap;

// Turned off until we get consistent nalgebra crates

// pub fn unwrap_uvs_uvgen(mesh: &mut Mesh) {
//     if let Some(pos_values) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)
//     && let Some(pts) = pos_values.as_float3() {
//         let mut uvs = Vec::with_capacity(pts.len());

//         // This "works" but is intended for packing, not sensibility.
//         let patch = uvgen::generate_uvs(
//             pts.iter().map(|v| nalgebra::Vector3::new(v[0], v[1], v[2])),
//             (0u32..pts.len() as u32).array_chunks::<3>(),
//             0.005,
//         ).unwrap();

//         for tc in patch.second_tex_coords {
//             uvs.push([tc[0], tc[1]]);
//         }

//         *mesh = mesh.clone().with_inserted_attribute(
//             Mesh::ATTRIBUTE_UV_0,
//             uvs,
//         );
//     }
// }


/// Maps each triangle from surface to appropriate side of box.
/// This is so-called box mapping.
pub fn generate_uv_box(vertices: &[[f32; 3]], triangles: &[[u32; 3]]) -> UvBox {
    let mut uv_box = UvBox::default();
    for (i, triangle) in triangles.iter().enumerate() {
        let a = Vec3::from_array(vertices[triangle[0] as usize]);
        let b = Vec3::from_array(vertices[triangle[1] as usize]);
        let c = Vec3::from_array(vertices[triangle[2] as usize]);
        let normal = (b - a).cross(c - a);
        let class = classify_plane(normal);
        // dbg!(normal, class);
        match class {
            PlaneClass::XY => {
                if normal.z < 0.0 {
                    uv_box.nz.push(i);
                    uv_box.projections.push([a.yx(), b.yx(), c.yx()])
                } else {
                    uv_box.pz.push(i);
                    uv_box.projections.push([a.xy(), b.xy(), c.xy()]);
                }
            }
            PlaneClass::XZ => {
                if normal.y < 0.0 {
                    uv_box.ny.push(i);
                    uv_box.projections.push([a.xz(), b.xz(), c.xz()])
                } else {
                    uv_box.py.push(i);
                    uv_box.projections.push([a.zx(), b.zx(), c.zx()])
                }
            }
            PlaneClass::YZ => {
                if normal.x < 0.0 {
                    uv_box.nx.push(i);
                    uv_box.projections.push([a.zy(), b.zy(), c.zy()])
                } else {
                    uv_box.px.push(i);
                    uv_box.projections.push([a.yz(), b.yz(), c.yz()])
                }
            }
        }
    }
    uv_box
}

/// Get a list of this Mesh's triangle indices as an iterator if possible.
/// (adapted from Mesh::triangles())
///
/// Returns an error if any of the following conditions are met (see [`MeshTrianglesError`]):
/// * The Mesh's [primitive topology] is not `TriangleList` or `TriangleStrip`.
/// * The Mesh is missing position or index data.
/// * The Mesh's position data has the wrong format (not `Float32x3`).
pub fn mesh_triangle_indices(mesh: &Mesh) -> Result<Vec<[u32; 3]>, MeshTrianglesError> {
    use wgpu::*;
    use bevy::mesh::Indices;

    let Some(position_data) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else {
        return Err(MeshTrianglesError::PositionsFormat);
    };

    let Some(indices) = mesh.indices() else {
        // Assume just grouped by 3s.
        return Ok((0u32..position_data.len() as u32).array_chunks::<3>().collect::<Vec<_>>());
    };

    let mut ret_vec = vec![];
    match mesh.primitive_topology() {
        PrimitiveTopology::TriangleList => {
            // When indices reference out-of-bounds vertex data, the triangle is omitted.
            // This implicitly truncates the indices to a multiple of 3.
             match indices {
                Indices::U16(vec) =>
                    ret_vec.extend(vec.as_slice()
                        .chunks_exact(3)
                        .flat_map(move |indices| indices.iter().map(|i| *i as u32)
                        .array_chunks::<3>())),
                    Indices::U32(vec) =>
                        ret_vec.extend(vec.as_slice()
                        .chunks_exact(3)
                        .flat_map(move |indices| indices.iter().map(|i| *i as u32)
                        .array_chunks::<3>())),
            };

        }

        PrimitiveTopology::TriangleStrip => {
            // When indices reference out-of-bounds vertex data, the triangle is omitted.
            // If there aren't enough indices to make a triangle, then an empty vector will be
            // returned.
            match indices {
                Indices::U16(vec) => {
                    ret_vec.extend(vec.as_slice().windows(3).enumerate().flat_map(
                        move |(i, indices)| {
                            if i % 2 == 0 {
                                [indices[0] as u32, indices[1] as u32, indices[2] as u32]
                            } else {
                                [indices[1] as u32, indices[0] as u32, indices[2] as u32]
                            }
                        },
                    ).array_chunks::<3>())
                }
                Indices::U32(vec) => {
                    ret_vec.extend(vec.as_slice().windows(3).enumerate().flat_map(
                        move |(i, indices)| {
                            if i % 2 == 0 {
                                [indices[0] as u32, indices[1] as u32, indices[2] as u32]
                            } else {
                                [indices[1] as u32, indices[0] as u32, indices[2] as u32]
                            }
                        },
                    ).array_chunks::<3>())
                }
            };
        }

        _ => {
            return Err(MeshTrianglesError::WrongTopology);
        }


    };

    return Ok(ret_vec);
}

/// Map each of the six orthogonal planes of the mesh to the
/// given UV ranges per side (nominally 0-1,0-1).
/// Unmentioned sides get the nomimal mapping.
pub fn unwrap_uvs_planar(mesh: &mut Mesh, side_corners: HashMap<SideClass, Rect>) {
    {
        let Some(pv) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else { return };
        let Some(_) = pv.as_float3() else { return };
    }

    mesh.duplicate_vertices();

    let pos_values = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
    let pts = pos_values.as_float3().unwrap();

    let tris = mesh_triangle_indices(mesh).unwrap();

    let uv_box = generate_uv_box(pts, &tris);

    let mut uvs = vec![Vec2::ZERO; uv_box.num_uvs()];
    uv_box.assign_sides_to_faces_with(&mut uvs, |side| *side_corners.get(&side).unwrap_or(&full(side)));

    let indices = Indices::U32((0u32..pos_values.len() as u32).collect::<Vec<_>>());

    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        uvs,
    );

    mesh.insert_indices(indices);
}

// Copied from uvgen

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
enum PlaneClass {
    XY,
    YZ,
    XZ,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SideClass {
    NegX,
    PosX,
    NegY,
    PosY,
    NegZ,
    PosZ,
}
impl SideClass {
    fn restrict_to_plane(&self, v: Vec3) -> Vec2 {
        match self {
            SideClass::NegX |
            SideClass::PosX => v.yz(),
            SideClass::NegY |
            SideClass::PosY => v.zx(),
            SideClass::NegZ |
            SideClass::PosZ => v.xy(),
        }
    }
}

#[inline]
#[allow(clippy::useless_let_if_seq)]
fn classify_plane(normal: impl Into<Vec3>) -> PlaneClass {
    let normal = normal.into();
    let mut longest = 0.0f32;
    let mut class = PlaneClass::XY; // it will be assigned definitely

    if normal.x.abs() >= longest {
        longest = normal.x.abs();
        class = PlaneClass::YZ;
    }

    if normal.y.abs() > longest {
        longest = normal.y.abs();
        class = PlaneClass::XZ;
    }

    if normal.z.abs() > longest {
        class = PlaneClass::XY;
    }

    class
}

#[inline]
#[allow(clippy::useless_let_if_seq)]
fn classify_side(normal: impl Into<Vec3>) -> SideClass {
    let normal = normal.into();
    let mut longest = 0.0f32;
    let mut class = SideClass::NegX; // it will be assigned definitely

    if normal.x.abs() >= longest {
        longest = normal.x.abs();
        class = if normal.x >= 0.0 { SideClass::PosX } else { SideClass::NegX };
    }

    if normal.y.abs() > longest {
        longest = normal.y.abs();
        class = if normal.y >= 0.0 { SideClass::PosY } else { SideClass::NegY };
    }

    if normal.z.abs() > longest {
        class = if normal.z >= 0.0 { SideClass::PosZ } else { SideClass::NegZ };
    }

    class
}

/// A set of faces with triangles belonging to faces.
#[derive(Default, Debug)]
pub struct UvBox {
    px: Vec<usize>,
    nx: Vec<usize>,
    py: Vec<usize>,
    ny: Vec<usize>,
    pz: Vec<usize>,
    nz: Vec<usize>,
    projections: Vec<[Vec2; 3]>,
}

impl UvBox {
    fn num_uvs(&self) -> usize {
        self.projections.len() * 3
    }
}

fn full(_: SideClass) -> Rect {
    Rect::from_corners(Vec2::ZERO, Vec2::ONE)
}

impl UvBox {
    /// Assign each side of the receiver to fill the provided UV coordinate space.
    ///  get_uv_range accepts a PlaneClass and a negative (true) / positive (false) flag.
    pub fn assign_sides_to_faces_with(&self, uvs: &mut [Vec2], get_uv_range: impl Fn (SideClass) -> Rect) {
        self.assign_side_to_face(&self.nx, SideClass::NegX, &get_uv_range, uvs);
        self.assign_side_to_face(&self.px, SideClass::PosX, &get_uv_range, uvs);
        self.assign_side_to_face(&self.ny, SideClass::NegY, &get_uv_range, uvs);
        self.assign_side_to_face(&self.py, SideClass::PosY, &get_uv_range, uvs);
        self.assign_side_to_face(&self.nz, SideClass::NegZ, &get_uv_range, uvs);
        self.assign_side_to_face(&self.pz, SideClass::PosZ, &get_uv_range, uvs);
    }

    /// Assign each side of the receiver to fill the full UV coordinate space.
    pub fn assign_sides_to_faces(&self, uvs: &mut [Vec2]) {
        self.assign_side_to_face(&self.nx, SideClass::NegX, &full, uvs);
        self.assign_side_to_face(&self.px, SideClass::PosX, &full, uvs);
        self.assign_side_to_face(&self.ny, SideClass::NegY, &full, uvs);
        self.assign_side_to_face(&self.py, SideClass::PosY, &full, uvs);
        self.assign_side_to_face(&self.nz, SideClass::NegZ, &full, uvs);
        self.assign_side_to_face(&self.pz, SideClass::PosZ, &full, uvs);
    }

    fn assign_side_to_face(&self, side_indices: &[usize], class: SideClass, get_uv_range: &dyn Fn(SideClass) -> Rect, uvs: &mut [Vec2]) {
        debug_assert_eq!(uvs.len(), self.projections.len() * 3);

        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for side in side_indices {
            let proj = &self.projections[*side];
            min_x = min_x.min(proj[0].x).min(proj[1].x).min(proj[2].x);
            min_y = min_y.min(proj[0].y).min(proj[1].y).min(proj[2].y);
            max_x = max_x.max(proj[0].x).max(proj[1].x).max(proj[2].x);
            max_y = max_y.max(proj[0].y).max(proj[1].y).max(proj[2].y);
        }
        // dbg!(Vec2::new(min_x, max_x), Vec2::new(min_y, max_y));

        let rect = get_uv_range(class);
        let scale_x = rect.width() / (max_x - min_x);
        let scale_y = rect.height() / (max_y - min_y);
        // dbg!(scale_x, scale_y, min_x, max_x, min_y, max_y);

        let remap = |v: Vec2| -> Vec2 {
            match class {
                SideClass::NegX |
                SideClass::PosX => Vec2::new(
                    1.0 - ((v.x - min_x) * scale_x + rect.min.x),
                    1.0 - ((v.y - min_y) * scale_y + rect.min.y),
                ),
                SideClass::PosY |
                SideClass::NegY => Vec2::new(
                    1.0 - ((v.x - min_x) * scale_x + rect.min.x),
                    1.0 - ((v.y - min_y) * scale_y + rect.min.y),
                ),
                SideClass::NegZ |
                SideClass::PosZ => Vec2::new(
                    (v.x - min_x) * scale_x + rect.min.x,
                    1.0 - ((v.y - min_y) * scale_y + rect.min.y),
                ),
            }
        };
        for side in side_indices {
            let proj = &self.projections[*side];
            let uvidx = *side * 3;
            uvs[uvidx] = remap(proj[0]);
            uvs[uvidx + 1] = remap(proj[1]);
            uvs[uvidx + 2] = remap(proj[2]);
        }
    }
}

/// Manually construct the AABB for a given mesh under the given transform.
pub fn get_mesh_aabb(mesh: &Mesh, xfrm: &Transform) -> Aabb {
    if let Some(pos_values) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    && let Some(pts) = pos_values.as_float3() {
        Aabb::enclosing(
            pts
            .iter()
            .map(|pt| xfrm.transform_point(Vec3::new(pt[0], pt[1], pt[2]))))
            .unwrap()
    } else {
        Aabb::default()
    }
}

pub fn convert_trimesh_to_obj(mesh: &avian3d::parry::shape::TriMesh) -> obj::ObjData {
    use obj::ObjData;
    use obj::Object;
    use obj::Group;
    use obj::SimplePolygon;
    use obj::IndexTuple;

    let polys = mesh
        .indices()
        .iter()
        .map(|vs| SimplePolygon(vec![
            IndexTuple(vs[0] as _, None, None),
            IndexTuple(vs[1] as _, None, None),
            IndexTuple(vs[2] as _, None, None),
        ])).collect::<Vec<_>>();

    ObjData {
        position: mesh.vertices().iter().map(|ps| [ps[0] as f32, ps[1] as f32, ps[2] as f32]).collect::<Vec<_>>(),
        objects: vec![
            Object { name: "test".to_owned(), groups: vec![
                Group {
                    name: "test".to_owned(),
                    index: 0,
                    material: None,
                    polys,
                }
            ]},
        ],
        .. default()
    }
}

pub fn convert_bevy_mesh_to_obj(mesh: &Mesh) -> obj::ObjData {
    use obj::ObjData;
    use obj::Object;
    use obj::Group;
    use obj::SimplePolygon;
    use obj::IndexTuple;
    use obj::ObjMaterial;

    let inds = mesh.indices().unwrap();
    let uvs = match mesh.attribute(Mesh::ATTRIBUTE_UV_0).unwrap() {
        bevy::mesh::VertexAttributeValues::Float32x2(items) => Some(items),
        _ => None,
    };
    let polys = inds.iter()
        .array_chunks::<3>()
        .map(|vs| SimplePolygon(vec![
            IndexTuple(vs[0], Some(vs[0]), Some(vs[0])),
            IndexTuple(vs[1], Some(vs[1]), Some(vs[1])),
            IndexTuple(vs[2], Some(vs[2]), Some(vs[2])),
        ])).collect::<Vec<_>>();


    ObjData {
        position: mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap().as_float3().unwrap().to_vec(),
        normal: mesh.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap().as_float3().unwrap().to_vec(),
        texture: uvs.map_or(vec![], |uvs| uvs.to_vec()),
        objects: vec![
            Object { name: "test".to_owned(), groups: vec![
                Group {
                    name: "test".to_owned(),
                    index: 0,
                    material: Some(ObjMaterial::Ref("foo".to_string())),
                    polys,
                }
            ]},
        ],
        .. default()
    }
}

pub fn export_obj(path: impl Into<std::path::PathBuf>, data: obj::ObjData) -> anyhow::Result<()> {
    use obj::Obj;

    let obj = Obj{ data, path: default() };
    let path = path.into();
    obj.save(&path).map_err(|e| anyhow::anyhow!("{e}"))?;
    info!("Wrote {path:?}");
    Ok(())
}

#[derive(Debug, Default)]
pub struct SideSpans {
    sides: FxHashMap<SideClass, SideSpan>,
}

impl SideSpans {
    fn remap(&self, pos: impl Into<Vec3>, normal: impl Into<Vec3>) -> Vec2 {
        let pos = pos.into();
        let side = classify_side(normal.into());
        self.sides.get(&side).map_or(Vec2::default(), |s| s.remap(pos, side))
    }
}

#[derive(Debug)]
pub struct SideSpan {
    min: Vec3,
    max: Vec3,
    pos: Vec<Vec3>,
    uvs: Vec<Vec2>,
}

impl Default for SideSpan {
    fn default() -> Self {
        SideSpan {
            min: Vec3::MAX,
            max: Vec3::MIN,
            pos: default(),
            uvs: default(),
        }
    }
}

impl SideSpan {
    /// Given a point which may or may not be in the side, map the UV.
    fn remap(&self, pt: Vec3, side: SideClass) -> Vec2 {
        let pt = side.restrict_to_plane(pt);
        let mut pt_min = Vec2::MAX;
        let mut pt_max = Vec2::MIN;
        let mut uv_min = Vec2::MAX;
        let mut uv_max = Vec2::MIN;
        for index in 0..self.uvs.len() {
            let pos = side.restrict_to_plane(self.pos[index]);
            let uv = self.uvs[index];
            if (pos.x <= pt.x && pos.x <= pt_min.x)
            && (pos.y <= pt.y && pos.y <= pt_min.y)
            {
                pt_min = pos;
                uv_min = uv;
            }
            if (pos.x >= pt.x && pos.x >= pt_max.x)
            && (pos.y >= pt.y && pos.y >= pt_max.y)
            {
                pt_max = pos;
                uv_max = uv;
            }
        }

        let x_diff = pt_max.x - pt_min.x;
        let y_diff = pt_max.y - pt_min.y;
        let uv_x_diff = uv_max.x - uv_min.x;
        let uv_y_diff = uv_max.y - uv_min.y;
        Vec2::new(
            if x_diff > 0. { uv_min.x + uv_x_diff * (pt.x - pt_min.x) / x_diff } else { uv_min.x },
            if y_diff > 0. { uv_min.y + uv_y_diff * (pt.y - pt_min.y) / y_diff } else { uv_min.y },
        )
    }
}

pub fn get_uv_maps(mesh: &Mesh) -> (SideSpans, SideSpans) {
    let pos = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap().as_float3().unwrap();
    let normals = mesh.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap().as_float3().unwrap();
    let pts0 = if let Some(uv0s) = match mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
        Some(bevy::mesh::VertexAttributeValues::Float32x2(items)) => Some(items),
        _ => None,
    } {
        map_uvs(pos, normals, &uv0s[..])
    } else {
        default()
    };
    let pts1 = if let Some(uv1s) = match mesh.attribute(Mesh::ATTRIBUTE_UV_1) {
        Some(bevy::mesh::VertexAttributeValues::Float32x2(items)) => Some(items),
        _ => None,
    } {
        map_uvs(pos, normals, &uv1s[..])
    } else {
        default()
    };
    (pts0, pts1)
}

fn map_uvs(pos: &[[f32; 3]], normals: &[[f32; 3]], uvs: &[[f32; 2]]) -> SideSpans {
    let mut side_spans = SideSpans::default();
    for index in 0..pos.len() {
        let pt: Vec3 = pos[index].into();
        let side = classify_side(normals[index]);
        let side_span = side_spans.sides.entry(side).or_default();
        side_span.min = side_span.min.min(pt);
        side_span.max = side_span.max.max(pt);
        side_span.pos.push(pt);
        side_span.uvs.push(uvs[index].into());
    }

    side_spans
}

pub fn update_uv_maps(mesh: &mut Mesh, (orig_uv0, orig_uv1): (SideSpans, SideSpans)) {
    let pos = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap().as_float3().unwrap();
    let normals = mesh.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap().as_float3().unwrap();
    let mut uvs0: Vec<[f32; 2]> = vec![];
    let mut uvs1: Vec<[f32; 2]> = vec![];

    for new_index in 0..pos.len() {
        let uv = orig_uv0.remap(pos[new_index], normals[new_index]);
        uvs0.push([uv.x, uv.y]);
        let uv = orig_uv1.remap(pos[new_index], normals[new_index]);
        uvs1.push([uv.x, uv.y]);
    }

    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs0);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, uvs1);
}

pub fn create_uvmapped_mesh(shape: impl Into<Mesh>) -> Mesh {
    let mut mesh: Mesh = shape.into();

    let aabb = get_mesh_aabb(&mesh, &Transform::IDENTITY);

    let [xs, ys, zs] = aabb.half_extents.to_array();
    let (nx_span, px_span) = if ys <= zs {
        (
            Rect::new(0.0, 0.0, 1.0, ys / zs),
            Rect::new(0.0, 0.0, ys / zs, 1.0),
        )
    } else {
        (
            Rect::new(0.0, 0.0, 1.0, zs / ys),
            Rect::new(0.0, 0.0, zs / ys, 1.0),
        )
    };
    let (ny_span, py_span) = if xs <= zs {
        (
            Rect::new(0.0, 0.0, 1.0, xs / zs),
            Rect::new(0.0, 0.0, 1.0, xs / zs),
        )
    } else {
        (
            Rect::new(0.0, 0.0, 1.0, zs / xs),
            Rect::new(0.0, 0.0, 1.0, zs / xs),
        )
    };
    let (nz_span, pz_span) = if xs <= ys {
        (
            Rect::new(0.0, 0.0, xs / ys, 1.0),
            Rect::new(0.0, 0.0, 1.0, xs / ys),
        )
    } else {
        (
            Rect::new(0.0, 0.0, ys / xs, 1.0),
            Rect::new(0.0, 0.0, 1.0, ys / xs),
        )
    };

    // let x_span = Rect::new(0.0, 0.0, aabb.half_extents.y * 2.0, aabb.half_extents.z * 2.0);
    // let y_span = Rect::new(0.0, 0.0, aabb.half_extents.x * 2.0, aabb.half_extents.z * 2.0);
    // let z_span = Rect::new(0.0, 0.0, aabb.half_extents.x * 2.0, aabb.half_extents.y * 2.0);
    // let x_span = Rect::new(0.0, 0.0, 1.0, 1.0);
    // let y_span = Rect::new(0.0, 0.0, 1.0, 1.0);
    // let z_span = Rect::new(0.0, 0.0, 1.0, 1.0);
    unwrap_uvs_planar(&mut mesh, HashMap::from([
        (SideClass::NegX, nx_span),
        (SideClass::PosX, px_span),
        (SideClass::NegY, ny_span),
        (SideClass::PosY, py_span),
        (SideClass::NegZ, nz_span),
        (SideClass::PosZ, pz_span),
    ]));

    mesh.compute_smooth_normals();
    mesh.generate_tangents().unwrap();

    mesh
}
