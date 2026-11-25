use bevy::prelude::*;

use crate::physics::*;
use crate::prelude::*;

pub struct CollidersPlugin;

impl Plugin for CollidersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BaseVhacdParameters>()
            .add_systems(FixedPreUpdate, apply_colliders);
    }
}

/// The global state of Vhacd collider generation.
#[derive(Resource, Reflect, Debug, Clone, PartialEq)]
#[reflect(Resource, Default)]
#[type_path = "game"]
pub struct BaseVhacdParameters(pub VhacdParameters);

impl Default for BaseVhacdParameters {
    fn default() -> Self {
        Self(VhacdParameters {
            resolution: 256,                 // Higher = more detail (but slower)
            concavity: 0.001,                // Lower = more parts but better fit
            max_convex_hulls: 64,            // Maximum number of convex parts
            plane_downsampling: 4,           // Precision of plane search
            convex_hull_downsampling: 4,     // Precision of convex hull generation
            alpha: 0.05,                     // Bias toward symmetrical splits
            beta: 0.05,                      // Bias toward revolution axis splits
            convex_hull_approximation: true, // Approximate for speed
            fill_mode: FillMode::FloodFill {
                detect_cavities: false,
            },
        })
    }
}

#[derive(Reflect, Debug, Clone, Copy, PartialEq)]
#[reflect(Default)]
#[type_path = "game"]
pub enum ColliderKind {
    TriMesh { downscale: f32 },
    ConvexHull,
    VoxelizeMesh { voxel_size: f32 },
}

impl Default for ColliderKind {
    fn default() -> Self {
        ColliderKind::TriMesh { downscale: 1.0 }
    }
}

/// Defines the layers in a script-friendly way and by
/// defining exclusion filtering (`ignores`) to prevent
/// requiring user script maintain an ever-growing list of things.
#[derive(Reflect, Debug, Clone)]
#[reflect(Default)]
#[type_path = "game"]
pub struct LayerConfig {
    /// Primary layer the item lives in.
    pub layer: GameLayer,

    /// Additional layers the item lives in.
    pub extra_layers: Vec<GameLayer>,

    /// Which layers the item does *not* interact with.
    pub ignores: Vec<GameLayer>,
}

impl LayerConfig {
    pub fn on(layer: GameLayer) -> Self {
        Self {
            layer,
            extra_layers: vec![],
            ignores: vec![],
        }
    }
    pub fn with_extra_layers(self, extra_layers: Vec<GameLayer>) -> Self {
        Self {
            extra_layers,
            ..self
        }
    }
    pub fn with_ignores(self, ignores: &[GameLayer]) -> Self {
        Self {
            ignores: ignores.to_vec(),
            ..self
        }
    }
}

impl LayerConfig {
    pub fn to_collision_layers(&self) -> CollisionLayers {
        let mut memberships: LayerMask = self.layer.into();
        for lay in &self.extra_layers {
            memberships |= lay.to_bits();
        }
        let mut filters: LayerMask = GameLayer::all_bits().into();
        for ign in &self.ignores {
            filters &= !ign.to_bits();
        }
        CollisionLayers {
            memberships,
            filters,
        }
    }
}

impl Default for LayerConfig {
    fn default() -> Self {
        Self {
            layer: GameLayer::World,
            extra_layers: vec![],
            ignores: vec![],
        }
    }
}

/// Marker that tells a system to generate and apply collision properties to a Mesh.
#[derive(Component, Reflect, Default, Debug, Clone)]
#[require(RigidBody::Static)]
#[reflect(Component, Default)]
#[type_path = "game"]
pub struct EnsureCollider {
    pub kind: ColliderKind,
    pub layer_config: LayerConfig,
}

#[derive(Component, Reflect)]
#[component(storage = "SparseSet")]
#[reflect(Component)]
#[type_path = "game"]
struct EnsureColliderStop;

/// Monitor any modified/added [EnsureCollider] components and
/// use [apply_collider] to update the tree of entities,
/// which is the first [EnsureColliderStop] found in a given
/// row of the tree.
///
/// Notwithstanding the name, these components are retained
/// and act as a "live component" for driving the otherwise difficult-to-access bits..
fn apply_colliders(
    mut commands: Commands,
    ensure_collider_q: Query<(Entity, &EnsureCollider)>,
    changed_collider_q: Query<Entity, Changed<EnsureCollider>>,
    stop_collider_q: Query<&EnsureColliderStop>,
    vhacd: Res<BaseVhacdParameters>,
    child_q: Query<&Children>,
    mesh_q: Query<&Mesh3d>,
) {
    for (ent, spawn) in ensure_collider_q.iter() {
        if changed_collider_q.contains(ent) || vhacd.is_changed() {
            apply_collider(
                commands.entity(ent),
                &vhacd,
                spawn.kind,
                &spawn.layer_config,
                &mesh_q,
            );
        }

        for kid in child_q.iter_descendants(ent) {
            if stop_collider_q.contains(kid) {
                break;
            }
            if changed_collider_q.contains(kid) || vhacd.is_changed() {
                apply_collider(
                    commands.entity(kid),
                    &vhacd,
                    spawn.kind,
                    &spawn.layer_config,
                    &mesh_q,
                );
            }
        }
    }
}

fn apply_collider(
    mut ent_commands: EntityCommands,
    vhacd: &BaseVhacdParameters,
    kind: ColliderKind,
    layer_config: &LayerConfig,
    mesh_q: &Query<&Mesh3d>,
) {
    let collider = match kind {
        ColliderKind::TriMesh { downscale } => {
            if downscale >= 1.0 {
                ColliderConstructor::TrimeshFromMeshWithConfig(TrimeshFlags::FIX_INTERNAL_EDGES)
            } else {
                let vhacd = &vhacd.0;
                let resolution = ((vhacd.resolution as f32 * downscale).ceil() as u32).max(16);
                let max_convex_hulls =
                    ((vhacd.max_convex_hulls as f32 * downscale).ceil() as u32).max(16);
                let concavity = (vhacd.concavity as f32 / downscale / downscale).min(1.0);
                ColliderConstructor::ConvexDecompositionFromMeshWithConfig(
                    // Apply base parameters but correct for some panic-inducing range errors.
                    VhacdParameters {
                        concavity,
                        resolution,
                        // plane_downsampling: vhacd.plane_downsampling.max(1),
                        // convex_hull_downsampling: vhacd.convex_hull_downsampling.max(1),
                        max_convex_hulls,
                        ..vhacd.clone()
                    },
                )
            }
        }
        ColliderKind::VoxelizeMesh { voxel_size } => {
            ColliderConstructor::VoxelizedTrimeshFromMesh {
                voxel_size,
                fill_mode: FillMode::FloodFill {
                    detect_cavities: true,
                },
            }
        }
        ColliderKind::ConvexHull => ColliderConstructor::ConvexHullFromMesh,
    };

    if !mesh_q.contains(ent_commands.id()) {
        ent_commands.try_insert(ColliderConstructorHierarchy::new(collider));
    } else {
        ent_commands.try_insert(collider);
    }
    ent_commands.try_insert(layer_config.to_collision_layers());
    ent_commands.try_remove::<Collider>();
}
