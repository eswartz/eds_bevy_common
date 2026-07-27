use bevy::prelude::*;

use crate::physics::*;
use crate::prelude::*;

pub struct PlayerEnvironmentPlugin;

impl Plugin for PlayerEnvironmentPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(
                FixedPreUpdate,
                (
                    check_player_environment_fps
                        .run_if(resource_exists_and_equals(PlayerMode::Fps))
                    ,
                    check_player_environment_space
                        .run_if(resource_exists_and_equals(PlayerMode::Space))
                    ,
                ).chain()
                .before(TransformSystems::Propagate)
                .after(PhysicsSystems::Writeback)
                .run_if(not(is_paused))
                .run_if(in_state(GameplayState::Playing))
            )
        ;
    }
}

pub(crate) fn check_player_environment_fps(
    mut player_q: Query<
        (
            Entity,
            &mut PlayerMovement,
            &mut LinearVelocity,
            &GlobalTransform,
            &ColliderAabb,
        ),
        With<Player>,
    >,
    collisions: Collisions,
    collider_q: Query<&CollisionLayers>,
    parent_q: Query<&ChildOf>,
    mut raycast: MeshRayCast,
    settings: Res<PlayerInputSettings>,
    gravity: Res<Gravity>,
) {
    for (player_ent, mut movement, mut vel, gxfrm, aabb) in player_q.iter_mut() {
        if movement.state == MovementState::Scripted {
            continue;
        }

        // Jump out of water.
        if movement.area.in_liquid()
            && movement.state == MovementState::Jumping
            && !movement.jumping_out
            && vel.xz().length() < settings.base_xz_speed as Scalar
        {
            for pair in collisions.collisions_with(player_ent) {
                if pair.total_normal_impulse().length() < 0.8 {
                    // Make sure the player is pushing, not just brushing.
                    continue;
                }

                for manifold in pair.manifolds.iter() {
                    if manifold
                        .normal
                        .dot(gxfrm.rotation() * Vector::Y)
                        .abs()
                        <= 0.25
                    {
                        // warn!("jump ignoring {:?}", manifold.normal);
                        continue;
                    }

                    // Jump up a little more, once.
                    vel.0.y = vel.0.y.max(settings.grounded_y_speed as Scalar) + 1.0;
                    movement.state = MovementState::Jumping;
                    movement.jumping_out = true;
                    movement.medium_friction = 1.0;
                    break;
                }
            }
        }

        let is_falling = vel.y <= -(settings.grounded_y_speed as Scalar);
        let is_flying = vel.y >= settings.grounded_y_speed as Scalar;

        let mut try_to_land = false;

        if is_falling {
            // Falling, must be in air.
            // We land when physics says we collide with something.
            movement.medium_friction = 0.0;
            movement.state = MovementState::Falling;
            movement.jumping_out = false;
            try_to_land = true;

        } else if is_flying {
            movement.medium_friction = 0.0;
            if movement.state != MovementState::Jumping {
                // Jumping is an intentional state.
                // If we're moving vertically in another state,
                // perhaps physics has bounced us off across a bump.
                try_to_land = true;
            }
        } else {
            // Stuck in air?
            movement.medium_friction = 1.0;
            try_to_land = true;

            // I.e. stop flying.
            if vel.y > 0.0 && gravity.0.y < 0.0 {
                vel.y *= 0.99;
            }
        }

        // Try to land if needed.
        if try_to_land {
            let mut colliding = false;
            let mut floor_steepness = 1.0f32;
            for c in collisions.entities_colliding_with(player_ent) {
                let Some(coll) = collisions.get(player_ent, c) else { continue };

                // See if we're on a floor or close enough.

                for manifold in coll.manifolds.iter() {
                    let angle = manifold.normal.angle_between(Vec3::NEG_Y);
                    let steepness = 1.0 - angle / std::f32::consts::PI;
                    if steepness > 0.25 {
                        // Ignore very steep floors, walls, etc.
                        continue;
                    }

                    floor_steepness = floor_steepness.min(steepness);
                    colliding = true;
                }
            }

            if !colliding {
                // Are we close to the ground at least?
                let is_player_collider = |ent| {
                    let Some(layers) = (if let Ok(layers) = collider_q.get(ent) {
                        Some(layers)
                    } else {
                        parent_q
                            .iter_ancestors(ent)
                            .filter_map(|ent| collider_q.get(ent).ok())
                            .next()
                    }) else {
                        return false;
                    };
                    (layers.filters & GameLayer::Player) != 0
                };
                let rc_settings = MeshRayCastSettings::default().with_filter(&is_player_collider);

                // Start from a little bit above the feet.
                let feet = player_feet(gxfrm.translation(), aabb);
                const RAY_DIST: f32 = 0.125;

                let ray = Ray3d::new(feet + Vec3::new(0.0, RAY_DIST, 0.0), Dir3::NEG_Y);
                let results = raycast.cast_ray(ray, &rc_settings);
                if results.is_empty() {
                    movement.state = MovementState::Falling;
                } else if let Some(first) = results.first() {
                    // Assume we hit the ground if within range of the feet or legs.
                    let hit_distance = (first.1.distance - RAY_DIST).max(0.0);
                    let feet_range = aabb.size().y / 4.0;
                    if hit_distance < feet_range {
                        // OK, we should contact with the ground.
                        if movement.state != MovementState::Jumping || (!movement.had_jump_event && vel.0.y >= 0.0 && vel.0.y < 0.01) {
                            movement.state = movement.state.to_grounded();
                            colliding = false;
                        }
                        vel.y = vel.y.min(-0.01);
                    } else if is_flying {
                        movement.state = MovementState::Flying;
                    } else if vel.0.y.abs() < 0.01 {
                        let angle = first.1.normal.angle_between(Vec3::Y);
                        let steepness = angle / std::f32::consts::PI;

                        if steepness <= 0.25 {
                            floor_steepness = floor_steepness.min(steepness);
                            vel.y = vel.y.min(-0.01);
                            colliding = true;
                        }
                    }
                }
            }

            if colliding {
                if floor_steepness < 0.25 {
                    movement.state = movement.state.to_grounded();
                } else {
                    // Don't allow creeping up slopes.
                    vel.0.y = vel.0.y.min(0.0);
                    movement.state = MovementState::OnSlope;
                }
            }
        }
    }
}

pub(crate) fn check_player_environment_space(
    mut player_q: Query<
        (
            Entity,
            &PlayerMovement,
            &mut LinearVelocity,
        ),
        With<Player>,
    >,
    settings: Res<PlayerInputSettings>,
) {
    for (_player_ent, movement, mut vel) in player_q.iter_mut() {
        if movement.state == MovementState::Scripted {
            continue;
        }

        if movement.velocity.length() < 0.01 {
            // Lose speed gradually.
            vel.0 *= settings.air_scale as Scalar;
        }
    }
}
