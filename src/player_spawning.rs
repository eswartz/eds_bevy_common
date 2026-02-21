use avian3d::math::*;
use avian3d::prelude::*;
use bevy::asset::uuid::Uuid;
use bevy::prelude::*;

use crate::*;

// This matches the eye height of a Quake-like player. A small figure.
pub const QUAKE_SCALE: Vec3 = Vec3::new(0.5, 1.5, 0.3);


/// Spawn (or respawn) a player entity into the world.
///
/// This makes the physics colliders which work best for
/// FPS scenarios. The player body is a capsule sized like
/// a Quake player.
///
/// Expects [PlayerMode] to be registered.
///
/// world: the world.
/// user_id: for multiplayer, a unique ID, else use [Uuid::default()].
///
pub fn spawn_fps_player(world: &mut World, user_id: Uuid, player_scale: Vec3, initial_xfrm: Transform) -> Entity {
    let mut exist_ent = None;
    {
        let mut player_q = world.query::<(Entity, &Player)>();
        for (ent, player) in player_q.query(world) {
            if player.0 == user_id {
                exist_ent = Some(ent);
                break;
            }
        }
    }
    if let Some(ent) = exist_ent {
        // Already here, so kill it.
        world.despawn(ent);
    }

    let radius = 0.333;
    let body_shape = Collider::capsule(
        radius as Scalar,
        (player_scale.y - radius * 2. - player_scale.z).max(0.25) as Scalar,
    );

    let rounded_size = 0.125 as Scalar;
    let head_size = (player_scale.z as Scalar) - rounded_size;
    let compound_shape = Collider::compound(vec![
        (
            Vector::ZERO,
            Quaternion::IDENTITY,
            body_shape,
        ),
        (
            Vector::new(0., (player_scale.y - player_scale.z * 2.0) as Scalar - head_size, 0.),
            Quaternion::IDENTITY,
            Collider::round_cuboid(head_size, head_size, head_size, rounded_size),
        ),

    ]);

    let mode = world.get_resource::<PlayerMode>().unwrap().clone();

    let player = world.spawn((
        Name::new("Player"),
        DespawnOnExit(ProgramState::InGame),
        (
            Player(user_id),
            PlayerMovement::default(),
            PlayerLook::default(),

            initial_xfrm,
            Visibility::Inherited,  // needed if no Mesh*
        ),
        (
            RigidBody::Dynamic,

            (
                Mass(75.),
                CenterOfMass(player_scale / 2.),
                Restitution::new(0.0),
                Friction::ZERO.with_dynamic_coefficient(0.0).with_static_coefficient(0.5),
            ),

            // Do not let physics modify rotation.
            LockedAxes::new()
                .lock_rotation_x()
                .lock_rotation_y()
                .lock_rotation_z(),

            compound_shape.clone(),
            default_player_collision_layers(),

            // Try to avoid falling through trimesh floor.
            CollisionMargin(0.01),
            SweptCcd::default(),

            // Avoid flying too much when e.g. colliding with a projectile.
            MaxLinearSpeed(4096.0),

            GravityScale(if mode == PlayerMode::Fps { 1.0 } else { 0.0 }),
        ),
    ))
    .id();

    player
}

/// If needed, add a child collider to the [Player] entity which
/// only responds to [GameLayer::Gameplay], independently of the body.
///
/// This child component is used to:
/// (1) interact with tiles/areas/buttons
/// (2) provide a collider shape that extends into the
/// ground so that when we step on e.g. a tile or sensor in the ground,
/// we don't lose contact with it as the player moves over it,
/// potentially bouncing slightly.
/// (3) leave the player "body" collider more amenable to
/// ordinary movement in a world.
/// (4) modify collisions to avoid "entering" world areas like
/// water/lava when standing above or near the edge.
///
pub fn add_fps_foot_gameplay_collider(mut commands: Commands, player: Entity, player_scale: Vec3) {

    commands.entity(player).with_children(|b| {
        b.spawn((
            Name::new("Game Collider"),
            Transform::from_translation(Vec3::new(0., -player_scale.x * 0.05, -player_scale.x)),
            Collider::cuboid(player_scale.x as Scalar, player_scale.y as Scalar, player_scale.x /* yes */ as Scalar),
			CollisionLayers::new([GameLayer::Player], [GameLayer::Gameplay]),
            ActiveCollisionHooks::MODIFY_CONTACTS,
        ));
    });

}

pub fn default_player_collision_layers() -> CollisionLayers {
    CollisionLayers::new(GameLayer::Player, [
        GameLayer::Default, GameLayer::World,
        // GameLayer::Gameplay, // set this on a child Game Collider does this
        GameLayer::Projectiles,
    ])
}
