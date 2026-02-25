/// Server-side player movement.
use std::time::Duration;

use avian3d::math::*;
use avian3d::prelude::*;
use bevy::prelude::*;

use crate::*;

pub struct PlayerMovementPlugin;

impl Plugin for PlayerMovementPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<PlayerInputSettings>()
            .register_type::<PlayerMovement>()
            .register_type::<PlayerLook>()
            .register_type::<PlayerCamera>()
            .add_systems(
                OnEnter(OverlayState::DebugGuiVisible),
                clear_player_velocity
                    .run_if(not(is_paused))
            )
            .add_systems(
                FixedPostUpdate,
                (
                    check_player_environment_fps,
                    check_player_environment_space,
                    process_player_input_movement_for_cheats.run_if(is_cheating),
                    process_player_input_movement_for_fps
                        .run_if(not(is_cheating))
                        ,
                    process_player_input_movement_for_space
                        .run_if(not(is_cheating))
                        ,
                    process_player_input_non_movement,
                ).chain()
                .before(TransformSystems::Propagate)
                .after(PhysicsSystems::Writeback)
                .run_if(not(is_paused))
                .run_if(in_state(GameplayState::Playing))
            )
        ;
    }
}

#[derive(Resource, Debug, Clone, Copy, Default, Reflect, PartialEq)]
#[reflect(Resource, Clone, Default)]
#[type_path = "game"]
pub enum PlayerMode {
    /// Move as in an FPS, with gravity and world friction,
    /// moving in user controlled X-Z with jump/crouch/fall in Y.
    #[default]
    Fps,
    /// Move as in a space ship / sim, moving in XYZ via
    /// impulses in the direction of the Player.
    Space,
}

fn is_cheating() -> bool {
    false
}

#[derive(Resource, Debug, Clone, Default, Reflect)]
#[reflect(Resource, Clone, Default)]
#[type_path = "game"]
pub struct PlayerInputSettings {
    /// multiplier, +ve
    pub move_scale: Vec3,
    /// multiplier, +ve
    pub turn_scale: Vec3,
    /// How much movement is accelerated when shift-moving.
    pub accelerate_scale: f32,
    /// How slowly movement is accelerated when shift-moving.
    pub velocity_ramp_scale: f32,
    /// How movement is scaled in air (i.e. usually < 1.0).
    pub air_scale: f32,
    /// Velocity scale for X/Z movement (m/s).
    pub base_xz_speed: u8,
    /// Velocity scale for jump (m/s).
    pub jump_accel: u16,
    /// Allow this many jumps.
    pub jump_max_count: u16,
    /// Maximum speed for X/Z movement (m/s).
    pub max_xz_speed: u8,
    /// Maximum speed for +Y movement (m/s).
    pub max_up_speed: u8,
    /// Maximum speed for -Y movement (m/s).
    pub max_down_speed: u8,
    /// Y velocity to consider "not falling or flying".
    pub grounded_y_speed: u8,
    /// Crouch depth.
    pub crouch_depth: f32,
    /// How long it takes for movement to decay after the user stops walking.
    pub movement_decay_time_secs: f32,
    /// How quickly to slow down when "flying" over a bump.
    pub fly_decay_time_secs: f32,
    /// How long it takes for turning to decay after the user stops turning.
    pub angular_decay_time_secs: f32,
    pub small_turn_time_secs: f32,
    pub large_turn_time_secs: f32,
}

impl PlayerInputSettings {
    pub fn for_fps() -> Self {
        Self {
            move_scale: Vec3::new(1.25, 1.0, 1.0), // strafe more
            turn_scale: Vec3::splat(0.05),
            velocity_ramp_scale: 1.0 / 8.0,
            accelerate_scale: 1.5,

            base_xz_speed: 8,
            jump_accel: 256,
            jump_max_count: 1,
            max_xz_speed: 16,
            max_up_speed: 96,
            max_down_speed: 96, // b/t 55 m/s for skydiver, 150 m/s competition
            crouch_depth: 0.5,
            grounded_y_speed: 1,
            air_scale: 0.125,

            movement_decay_time_secs: 1.0 / 30.0,
            fly_decay_time_secs: 1.0 / 8.0,
            angular_decay_time_secs: 1.0 / 60.0,
            small_turn_time_secs: 0.125,
            large_turn_time_secs: 0.5,
        }
    }

    pub fn for_space() -> Self {
        Self {
            move_scale: Vec3::splat(1.0),
            turn_scale: Vec3::splat(0.1),
            velocity_ramp_scale: 1.0 / 4.0,
            accelerate_scale: 2.0,

            base_xz_speed: 8,
            jump_accel: 256,
            jump_max_count: 256,
            max_xz_speed: 32,
            max_up_speed: 128,
            max_down_speed: 128,
            crouch_depth: 0.0,
            grounded_y_speed: 0,
            air_scale: 0.99,

            movement_decay_time_secs: 1.0 / 10.0,
            fly_decay_time_secs: 1.0 / 8.0,
            angular_decay_time_secs: 1.0 / 60.0,
            small_turn_time_secs: 0.5,
            large_turn_time_secs: 1.0,
        }
    }
}

#[derive(Debug, Default, Reflect, Clone, Copy, PartialEq, Eq)]
#[reflect(Clone, Default)]
#[type_path = "game"]
pub enum MovementState {
    /// Touching ground (or close enough).
    Grounded,
    /// Touching ground and walking.
    Walking,
    /// Touching ground and running.
    Running,
    /// On a slope to steep to be considered "ground".
    OnSlope,
    /// In the air and not moving (much) vertically.
    Floating,
    /// Jumping in the air.
    Jumping,
    /// Moving rapidly up.
    Flying,
    /// Moving rapidly down.
    Falling,
    /// Scripted movement (ignoring movement inputs).
    #[default]
    Scripted,
}

impl MovementState {
    pub fn is_on_surface(&self) -> bool {
        matches!(
            *self,
            MovementState::Grounded | MovementState::Walking | MovementState::Running
        )
    }
    #[allow(unused)]
    pub fn is_moving(&self) -> bool {
        matches!(
            *self,
            MovementState::Grounded
                | MovementState::Walking
                | MovementState::Running
                | MovementState::OnSlope
        )
    }

    fn to_grounded(&self) -> MovementState {
        match self {
            MovementState::Grounded
            | MovementState::Walking
            | MovementState::Running
            | MovementState::Scripted => *self,
            MovementState::OnSlope
            | MovementState::Floating
            | MovementState::Jumping
            | MovementState::Flying
            | MovementState::Falling => MovementState::Grounded,
        }
    }
}

/// This represents the state of player-driven movement.
///
#[derive(Debug, Component, Reflect, Clone)]
#[reflect(Component, Clone, Default)]
#[require(Saveable)]
#[type_path = "game"]
pub struct PlayerMovement {
    /// Current velocity.
    pub velocity: f32,
    /// Current velocity rampup.
    pub velocity_ramp: f32,
    /// Current state.
    pub state: MovementState,
    /// Previous state for purposes of sound.
    pub prev_state: MovementState,

    /// Represents how dense is the medium the player is in.
    /// I.e. 0.0 means empty space, 1.0 means encased on rock.
    pub medium_friction: f32,
    /// Counts how many player jumps are allowed still.
    /// (Decremented form a start [PlayerInputSettings::jump_count].
    pub allowed_jumps: u16,
    pub jumping_out: bool,

    pub turn_time_secs: f32,
    pub turn_deadline_secs: f32,
    pub turn_curve: Option<EasingCurve<Quat>>,
    pub turn_sets_look: bool,
    /// Current area of feet.
    pub area: AreaContent,
}

impl Default for PlayerMovement {
    fn default() -> Self {
        Self {
            velocity: 0.0,
            velocity_ramp: 0.0,
            state: MovementState::Falling,
            prev_state: MovementState::Falling,
            medium_friction: 1.0,
            allowed_jumps: 0,
            jumping_out: false,
            turn_time_secs: 0.0,
            turn_deadline_secs: 0.0,
            turn_curve: None,
            turn_sets_look: false,
            area: AreaContent::Air,
        }
    }
}

impl PlayerMovement {
    #[allow(unused)]
    pub fn set_rotation(&mut self, to_rot: Quat, transform: &mut Transform) {
        transform.rotation = to_rot;
    }

    /// Tell if an animated turn is active.
    pub fn is_turning(&self) -> bool {
        self.turn_curve.is_some()
    }

    /// Initiate an animated turn sequence.
    pub fn turn_toward(&mut self, time: f32, from_rot: Quat, to_rot: Quat) {
        self.turn_time_secs = 0.0;
        self.turn_deadline_secs = time.max(0.001);

        self.turn_curve = Some(EasingCurve::new(from_rot, to_rot, EaseFunction::CubicInOut));
    }

    /// Initiate an animated turn but lock the view.
    #[allow(unused)]
    pub fn turn_toward_locking_view(&mut self, time: f32, from_rot: Quat, to_rot: Quat) {
        self.turn_time_secs = 0.0;
        self.turn_deadline_secs = time.max(0.001);

        self.turn_curve = Some(EasingCurve::new(from_rot, to_rot, EaseFunction::CubicInOut));
    }

    pub fn apply_turn(
        &mut self,
        dt: f32,
        rot_delta: Vec3,
        transform: &mut Transform,
    ) -> bool {
        if let Some(turn_curve) = &mut self.turn_curve {
            // Scripted case.
            if rot_delta != Vec3::ZERO {
                // Nudge source and target accordingly.
                let adj = Quat::from_euler(EulerRot::YXZ, rot_delta.y, rot_delta.x, rot_delta.z);
                let from_rot = transform.rotation * adj;
                let to_rot = turn_curve.sample_clamped(1.0) * adj;
                *turn_curve = EasingCurve::new(from_rot, to_rot, EaseFunction::CubicInOut);
            }
            let new_time = self.turn_time_secs + dt;
            transform.rotation = turn_curve.sample_clamped(new_time / self.turn_deadline_secs);

            if new_time >= self.turn_deadline_secs {
                self.turn_time_secs = 0.0;
                self.turn_curve = None;
            } else {
                self.turn_time_secs = new_time;
            }
            true
        } else {
            // Incremental case. Only update if it should change.
            if rot_delta != Vec3::ZERO {
                let new_quat = {
                    let (ey, ex, ez) = transform.rotation.to_euler(EulerRot::YXZ);
                    let mut look_angles = Vec3::new(ex, ey, ez) + rot_delta;
                    look_angles.x = look_angles
                        .x
                        .clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);
                    look_angles.y %= std::f32::consts::TAU;
                    look_angles.z = look_angles
                        .z
                        .clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);
                    Quat::from_euler(EulerRot::YXZ, look_angles.y, look_angles.x, look_angles.z)
                };
                transform.rotation = new_quat;
            }
            false
        }
    }
}

/// This marks the Camera representing the player entity camera's point of view.
#[derive(Component, Default, Reflect)]
#[require(Saveable)]
#[reflect(Default)]
#[type_path = "game"]
pub struct PlayerCamera(pub CameraMode);

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Reflect)]
#[reflect(Clone, Default)]
#[type_path = "game"]
pub enum CameraMode {
    #[default]
    FirstPerson,
    ThirdPerson,
    Stationary,
    LookingAt,
}
impl CameraMode {
    pub fn next(&self) -> Self {
        match self {
            Self::FirstPerson => Self::ThirdPerson,
            Self::ThirdPerson => Self::LookingAt,
            Self::LookingAt => Self::Stationary,
            Self::Stationary => Self::FirstPerson,
        }
    }
}

#[derive(Debug, Component, Reflect, Clone)]
#[require(Saveable)]
#[reflect(Component, Clone, Default)]
#[type_path = "game"]
pub struct PlayerLook {
    /// Where we're looking.
    pub rotation: Quat,
    /// Current dynamic crouch distance (moving eyes down)
    pub crouch_y: f32,
    pub crouch_y_dir: f32,
    pub turn_time_secs: f32,
    pub turn_deadline_secs: f32,
    pub turn_curve: Option<EasingCurve<Quat>>,
    pub crouching: bool,
}

impl Default for PlayerLook {
    fn default() -> Self {
        Self {
            rotation: default(),
            crouch_y: 0.0,
            crouch_y_dir: 0.0,
            turn_time_secs: 0.0,
            turn_deadline_secs: 0.0,
            turn_curve: None,
            crouching: false,
        }
    }
}

impl PlayerLook {
    /// Initiate an animated turn sequence.
    pub fn turn_toward(&mut self, time: f32, from_rot: Quat, to_rot: Quat) {
        self.turn_time_secs = 0.0;
        self.turn_deadline_secs = time.max(0.001);

        self.turn_curve = Some(EasingCurve::new(from_rot, to_rot, EaseFunction::CubicInOut));
    }

    pub fn apply_turn(&mut self, dt: f32, rot_delta: Vec3) -> bool {
        if let Some(turn_curve) = &mut self.turn_curve {
            // Scripted case.
            if rot_delta != Vec3::ZERO {
                // Nudge source and target accordingly.
                let adj = Quat::from_euler(EulerRot::YXZ, rot_delta.y, rot_delta.x, rot_delta.z);
                let from_rot = self.rotation * adj;
                let to_rot = turn_curve.sample_clamped(1.0) * adj;
                *turn_curve = EasingCurve::new(from_rot, to_rot, EaseFunction::CubicInOut);
            }
            let new_time = self.turn_time_secs + dt;
            self.rotation = turn_curve.sample_clamped(new_time / self.turn_deadline_secs);

            if new_time >= self.turn_deadline_secs {
                self.turn_time_secs = 0.0;
                self.turn_curve = None;
            } else {
                self.turn_time_secs = new_time;
            }
            true
        } else {
            // Incremental case. Only update if it should change.
            if rot_delta != Vec3::ZERO {
                let new_quat = {
                    let (ey, ex, ez) = self.rotation.to_euler(EulerRot::YXZ);
                    let mut look_angles = Vec3::new(ex, ey, ez) + rot_delta;
                    look_angles.x = look_angles
                        .x
                        .clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);
                    look_angles.y %= std::f32::consts::TAU;
                    look_angles.z = look_angles
                        .z
                        .clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);
                    // look_angles.z %= std::f32::consts::TAU;
                    Quat::from_euler(EulerRot::YXZ, look_angles.y, look_angles.x, look_angles.z)
                };

                self.rotation = new_quat;
            }
            false
        }
    }
}

/// Stop moving player, e.g. when input is going to UI.
fn clear_player_velocity(player_q: Single<&mut LinearVelocity, With<PlayerMovement>>) {
    let mut vel = player_q.into_inner();
    vel.0 = Vector::ZERO;
}

/// Get the local position of the player's feet relative to the given player transform.
pub fn player_feet(transform: &Transform, aabb: &ColliderAabb) -> Vec3 {
    Vec3::new(
        transform.translation.x,
        aabb.min.y as f32,
        transform.translation.z,
    )
}

/// Get the local position of the player's eyes relative to the given player transform.
pub fn player_eyes(transform: &Transform, aabb: &ColliderAabb, look: &PlayerLook) -> Vec3 {
    // let center = aabb.center().as_vec3();
    // Eyes are in the middle of the head.
    // let inside_head_pt = Vec3::new(center.x, aabb.max.y as f32 - 0.5, center.z);
    // inside_head_pt
    // // // Eyes are a little in front of the middle of the head,
    // // // but not so far that it clips through walls we collide with.
    // // inside_head_pt + transform.rotation * (Vec3::NEG_Z * 0.125)

    Vec3::new(
        transform.translation.x,
        aabb.max.y as f32 - 0.25 + look.crouch_y,
        transform.translation.z,
    )
}

/// Get the local position of the player's gun relative to the given player transform.
#[allow(unused)]
pub fn player_gun(transform: &Transform, eyes: Vec3) -> Vec3 {
    (eyes + Vec3::new(0., -0.25, 0.)) + transform.rotation * Vec3::NEG_Z * 0.25
}

fn check_player_environment_fps(
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
    mode: Res<PlayerMode>,
) {
    if *mode != PlayerMode::Fps {
        return
    }

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
                        .dot(gxfrm.rotation().adjust_precision() * Vector::Y)
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
        }

        // Try to land if needed.
        if try_to_land {
            let mut colliding = false;
            let mut floor_steepness = 1.0f32;
            for c in collisions.entities_colliding_with(player_ent) {
                let Some(coll) = collisions.get(player_ent, c) else { continue };

                // See if we're on a floor or close enough.

                for manifold in coll.manifolds.iter() {
                    let angle = manifold.normal.adjust_precision().angle_between(Vec3::NEG_Y);
                    let steepness = angle / std::f32::consts::PI;
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
                let ray = Ray3d::new(player_feet(&gxfrm.compute_transform(), aabb) + Vec3::new(0.0, 0.5, 0.0), Dir3::NEG_Y);
                let results = raycast.cast_ray(ray, &rc_settings);
                if results.is_empty() {
                    movement.state = MovementState::Falling;
                } else if results[0].1.distance < (((aabb.size().y / 4.0) as f32) - 0.5) {
                    // OK, we should contact with the ground.
                    if movement.state != MovementState::Jumping {
                        movement.state = movement.state.to_grounded();
                        colliding = false;
                    }
                    vel.y = vel.y.min(-0.01);
                } else if is_flying {
                    movement.state = MovementState::Flying;
                } else if vel.0.y.abs() < 0.01 {
                    let angle = results[0].1.normal.angle_between(Vec3::Y);
                    let steepness = angle / std::f32::consts::PI;

                    if steepness <= 0.25 {
                        floor_steepness = floor_steepness.min(steepness);
                        vel.y = vel.y.min(-0.01);
                        colliding = true;
                    }
                }
            }

            if colliding {
                if floor_steepness <= 0.125 {
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

fn check_player_environment_space(
    mut player_q: Query<
        (
            Entity,
            &PlayerMovement,
            &mut LinearVelocity,
        ),
        With<Player>,
    >,
    settings: Res<PlayerInputSettings>,
    mode: Res<PlayerMode>,
) {
    if *mode != PlayerMode::Space {
        return
    }

    for (_player_ent, movement, mut vel) in player_q.iter_mut() {
        if movement.state == MovementState::Scripted {
            continue;
        }

        if movement.velocity < 0.01 {
            // Lose speed gradually.
            vel.0 = vel.0 * settings.air_scale;
        }
    }
}

pub fn process_player_input_movement_for_cheats(
    mut player_q: Query<
        (
            Forces,
            &mut PlayerMovement,
            &PlayerLook,
            &Transform,
        ),
        With<Player>,
    >,
    mut inputs: MessageReader<PlayerInput>,
    time: Res<Time>,
    settings: Res<PlayerInputSettings>,
) {
    for input in inputs.read() {
        let res = player_q.get_mut(input.player_entity());

        let Ok((mut forces, mut movement, look, transform)) = res
        else {
            let e = unsafe { res.unwrap_err_unchecked() };
            warn!("invalid player entity {}: {:?}", input.player_entity(), e);
            continue;
        };

        let mut vel = forces.linear_velocity();

        let mut instant_thrust = Vec3::ZERO;
        let mut overall_speed = settings.base_xz_speed as f32;
        match input {
            PlayerInput::Move(_, input) => {
                instant_thrust.x = Into::<f32>::into(input.right_left) * settings.move_scale.x;
                instant_thrust.y = Into::<f32>::into(input.up_down) * settings.move_scale.y;
                instant_thrust.z = Into::<f32>::into(input.forward_back) * settings.move_scale.z;

                instant_thrust = instant_thrust.clamp_length_max(2.0);

                let move_speed = if !look.crouching {
                    input.speed
                } else {
                    input.speed.slower()
                };
                let accel_scale = match move_speed {
                    Speed::Fast => settings.accelerate_scale,
                    Speed::Slow => 1.0 / settings.accelerate_scale,
                    Speed::Crawl => 0.5 / settings.accelerate_scale,
                    Speed::Normal => 1.0,
                };
                overall_speed *= accel_scale;

                let dir_velocity = transform.rotation * instant_thrust;

                let delta = dir_velocity * overall_speed;
                if delta.length_squared() > 0.01 {
                    // Go!
                    vel = delta.adjust_precision();
                } else {
                    // Slow down when not actively moving.
                    let decay = (-0.5 * time.delta_secs()
                        / settings.movement_decay_time_secs
                        / accel_scale)
                        .exp() as Scalar;
                    vel = Vector::new(vel.x * decay, vel.y * decay, vel.z * decay);
                }
            }
            _ => ()
        }

        // Clamp speed.
        let cur_vel_xz = vel.xz();
        let cur_len_xz = cur_vel_xz.length();
        let clamped_vel_xz = if cur_len_xz < 0.1 {
            movement.velocity_ramp = 0.0;
            Vector2::splat(0.0)
        } else {
            cur_vel_xz.clamp_length_max(settings.max_xz_speed as Scalar)
        };

        // Do not fall or fly.
        *forces.linear_velocity_mut() = Vector::new(clamped_vel_xz.x, 0.0, clamped_vel_xz.y);
    }
}

pub fn process_player_input_movement_for_fps(
    mut player_q: Query<
        (
            Forces,
            // &PlayerCheats,
            &mut PlayerMovement,
            &mut PlayerLook,
            &mut Transform,
        ),
        With<Player>,
    >,
    mut inputs: MessageReader<PlayerInput>,
    time: Res<Time>,
    settings: Res<PlayerInputSettings>,
    mode: Res<PlayerMode>,
) {
    if *mode != PlayerMode::Fps {
        return
    }

    let dt = time.delta_secs();
    for input in inputs.read() {
        let res = player_q.get_mut(input.player_entity());

        let Ok((mut forces, /* cheats, */ mut movement, mut look, mut transform)) = res
        else {
            let e = unsafe { res.unwrap_err_unchecked() };
            warn!("invalid player entity {}: {:?}", input.player_entity(), e);
            continue;
        };

        let mut vel = forces.linear_velocity();
        let mut jump_impulse = Vector::ZERO;

        let mut instant_thrust = Vec3::ZERO;
        let mut overall_speed = settings.base_xz_speed as f32;
        match input {
            PlayerInput::Move(..) if movement.state == MovementState::Scripted => {
                // Ignore.
            }

            PlayerInput::Move(_, input) => {
                instant_thrust.x = Into::<f32>::into(input.right_left) * settings.move_scale.x;
                instant_thrust.y = Into::<f32>::into(input.up_down) * settings.move_scale.y;
                instant_thrust.z = Into::<f32>::into(input.forward_back) * settings.move_scale.z;

                // Extract up/down.
                let mut up_down = instant_thrust.y;
                instant_thrust.y = 0.0;

                instant_thrust = instant_thrust.clamp_length_max(1.0);
                let speed_type = if !look.crouching {
                    input.speed
                } else {
                    input.speed.slower()
                };
                let move_scale = match speed_type {
                    Speed::Fast => settings.accelerate_scale,
                    Speed::Slow => 1.0 / settings.accelerate_scale,
                    Speed::Crawl => 0.5 / settings.accelerate_scale,
                    Speed::Normal => 1.0,
                };
                overall_speed *= move_scale;

                if instant_thrust == Vec3::ZERO {
                    movement.velocity_ramp = 0.0;
                }
                movement.velocity_ramp = (movement.velocity_ramp
                    + settings.velocity_ramp_scale * move_scale)
                    .clamp(0.0, 1.0);

                let mut dir_velocity = transform.rotation * instant_thrust * movement.velocity_ramp;

                const MAX_JUMP_MEDIUM_FRICTION: f32 = 0.25;

                // See if we can jump.
                let std_jump = up_down > 0.
                    && (movement.state.is_on_surface() || movement.allowed_jumps > 0)  // but not OnSlope
                    && movement.medium_friction >= MAX_JUMP_MEDIUM_FRICTION;
                if std_jump {
                    if movement.allowed_jumps > 0 {
                        movement.allowed_jumps -= 1;
                        let sluggishness = move_scale.min(1.0);
                        // Jump strictly up.
                        jump_impulse = Vector::new(
                            0.,
                            settings.jump_accel as Scalar * sluggishness as Scalar,
                            0.,
                        );
                        movement.state = MovementState::Jumping;
                    }
                    // Consume for jump or failed re-jump.
                    up_down = 0.;
                } else if up_down <= 0. {
                    movement.allowed_jumps = settings.jump_max_count;
                }

                if up_down == 0. && vel.y > 0. && movement.state == MovementState::Flying {
                    // HACK: Since we're using physics for the character, we can sometimes "fly"
                    // just by running across a bump. Correct for that with prejudice.
                    vel.y = 0.0;
                    movement.state = movement.state.to_grounded();
                }

                // Apply unconsumed strict up/down movement.
                if up_down != 0. {
                    dir_velocity.y = up_down;
                }

                let dir_velocity = dir_velocity * Vec3::new(overall_speed, 1.0, overall_speed);
                if dir_velocity.length_squared() > 0.01 {
                    if movement.state.is_on_surface() {
                        vel.x = (vel.x + dir_velocity.x as Scalar) / 2.0;
                        vel.z = (vel.z + dir_velocity.z as Scalar) / 2.0;
                    } else {
                        let asc = settings.air_scale as Scalar;
                        let bs = (settings.base_xz_speed as Scalar) * asc;
                        if vel.x.abs() < bs {
                            vel.x = (dir_velocity.x as Scalar) * asc;
                        }
                        if vel.z.abs() < bs {
                            vel.z = (dir_velocity.z as Scalar) * asc;
                        }
                    }
                    vel.y += (dir_velocity.y * dt) as Scalar;
                } else {
                    // Apply friction while touching surface.
                    if movement.state.is_on_surface() {
                        let decay = (-0.5 * time.delta_secs()
                            / settings.movement_decay_time_secs
                            / move_scale)
                            .exp() as Scalar;

                        vel = Vector::new(vel.x * decay, vel.y, vel.z * decay);
                    }
                }
            }

            PlayerInput::HeadTurn(..) |
            PlayerInput::BodyTurn(..) |
            PlayerInput::TurnAround(..) |
            PlayerInput::Straighten(_) |
            PlayerInput::ToggleCrouch(..) |
            PlayerInput::StartFire(_) |
            PlayerInput::StopFire(_) => {
                // Ignore.
            }
        }

        // Apply any scripted movement.
        movement.apply_turn(dt, Vec3::ZERO, &mut transform);
        look.apply_turn(dt, Vec3::ZERO);

        // Crouch.
        look.crouch_y = look.crouch_y * 0.9
            - if look.crouching {
                settings.crouch_depth
            } else {
                0.0
            } * 0.1;

        // Clamp speed.
        let cur_vel_xz = vel.xz();
        let cur_len_xz = cur_vel_xz.length();
        let clamped_vel_xz = if cur_len_xz < 0.1 {
            movement.velocity_ramp = 0.0;
            Vector2::splat(0.0)
        } else {
            cur_vel_xz.clamp_length_max(settings.max_xz_speed as Scalar)
        };
        let clamped_y = {
            let clamped_y = vel.y.clamp(
                -(settings.max_down_speed as Scalar), // i.e. air/fluid resistance
                settings.max_up_speed as Scalar,      // i.e. flying/jumping
            );
            clamped_y
        };

        *forces.linear_velocity_mut() = Vector::new(clamped_vel_xz.x, clamped_y, clamped_vel_xz.y);
        // Add this outside since it modifies the velocity and we don't want it to clamp Y.
        forces.apply_linear_impulse(jump_impulse);

        if movement.state.is_on_surface() {
            let eff_speed = vel.xz().length() as f32;
            if eff_speed > settings.base_xz_speed as f32 {
                movement.state = MovementState::Running;
            } else if eff_speed >= settings.base_xz_speed as f32 / 2.0 {
                movement.state = MovementState::Walking;
            } else {
                movement.state = MovementState::Grounded;
            }
        }
    }
}

pub fn process_player_input_movement_for_space(
    mut player_q: Query<
        (
            Forces,
            // &PlayerCheats,
            &mut PlayerMovement,
            &mut PlayerLook,
            &mut Transform,
        ),
        With<Player>,
    >,
    mut inputs: MessageReader<PlayerInput>,
    time: Res<Time>,
    settings: Res<PlayerInputSettings>,
    mode: Res<PlayerMode>,
) {
    if *mode != PlayerMode::Space {
        return
    }

    let dt = time.delta_secs();
    for input in inputs.read() {
        let res = player_q.get_mut(input.player_entity());

        let Ok((mut forces, /* cheats, */ mut movement, mut look, mut transform)) = res
        else {
            let e = unsafe { res.unwrap_err_unchecked() };
            warn!("invalid player entity {}: {:?}", input.player_entity(), e);
            continue;
        };

        let mut vel = forces.linear_velocity();

        let mut instant_thrust = Vec3::ZERO;
        let mut overall_speed = settings.base_xz_speed as f32;
        match input {
            PlayerInput::Move(..) if movement.state == MovementState::Scripted => {
                // Ignore.
            }

            PlayerInput::Move(_, input) => {
                instant_thrust.x = Into::<f32>::into(input.right_left) * settings.move_scale.x;
                instant_thrust.y = Into::<f32>::into(input.up_down) * settings.move_scale.y;
                instant_thrust.z = Into::<f32>::into(input.forward_back) * settings.move_scale.z;

                instant_thrust = instant_thrust.clamp_length_max(2.0);

                let move_speed = if !look.crouching {
                    input.speed
                } else {
                    input.speed.slower()
                };
                let accel_scale = match move_speed {
                    Speed::Fast => settings.accelerate_scale,
                    Speed::Slow => 1.0 / settings.accelerate_scale,
                    Speed::Crawl => 0.5 / settings.accelerate_scale,
                    Speed::Normal => 1.0,
                };
                overall_speed *= accel_scale;

                // let dir_velocity = transform.rotation * instant_thrust;
                let dir_velocity = look.rotation * instant_thrust;

                let delta = dir_velocity * overall_speed;
                if delta.length_squared() > 0.01 {
                    vel = delta.adjust_precision();
                } else {
                    let decay = (-0.5 * time.delta_secs()
                        * settings.movement_decay_time_secs
                        / accel_scale)
                        .exp() as Scalar;

                    vel = Vector::new(vel.x * decay, vel.y * decay, vel.z * decay);
                }
            }

            PlayerInput::HeadTurn(..) |
            PlayerInput::BodyTurn(..) |
            PlayerInput::TurnAround(..) |
            PlayerInput::Straighten(_) |
            PlayerInput::ToggleCrouch(..) |
            PlayerInput::StartFire(_) |
            PlayerInput::StopFire(_) => {
                // Ignore.
            }
        }

        // Apply any scripted movement.
        movement.apply_turn(dt, Vec3::ZERO, &mut transform);
        look.apply_turn(dt, Vec3::ZERO);

        // Clamp speed.
        let cur_len = vel.length();
        let clamped_vel = if cur_len < 0.1 {
            movement.velocity_ramp = 0.0;
            Vector3::splat(0.0)
        } else {
            vel.clamp_length_max(settings.max_xz_speed as Scalar)
        };

        *forces.linear_velocity_mut() = clamped_vel;
    }
}

pub fn process_player_input_non_movement(
    mut player_q: Query<
        (
            &mut PlayerMovement,
            &mut PlayerLook,
            &mut Transform,
        ),
        With<Player>,
    >,
    mut inputs: MessageReader<PlayerInput>,
    time: Res<Time>,
    settings: Res<PlayerInputSettings>,
    mut next_fire_time: Local<Option<Duration>>,
) {
    let dt = time.delta_secs();
    for input in inputs.read() {
        let res = player_q.get_mut(input.player_entity());

        let Ok((mut movement, mut look, mut transform)) = res
        else {
            let e = unsafe { res.unwrap_err_unchecked() };
            warn!("invalid player entity {}: {:?}", input.player_entity(), e);
            continue;
        };

        match input {
            PlayerInput::HeadTurn(_, turn) => {
                let euler = turn.get_euler() * settings.turn_scale;
                look.apply_turn(dt, euler);
            }
            PlayerInput::BodyTurn(_, turn) => {
                let euler = turn.get_euler() * settings.turn_scale;
                movement.apply_turn(dt, euler, &mut transform);
            }
            PlayerInput::TurnAround(_player) => {
                if !movement.is_turning() {
                    let ey = transform.rotation.to_euler(EulerRot::YXZ).0;
                    let (_, ex, ez) = look.rotation.to_euler(EulerRot::YXZ);
                    let new_rot =
                        Quat::from_euler(EulerRot::YXZ, ey + std::f32::consts::PI, ex, ez)
                            .normalize();
                    movement.turn_toward(
                        settings.large_turn_time_secs,
                        transform.rotation,
                        new_rot,
                    );
                    look.turn_toward(settings.large_turn_time_secs, transform.rotation, new_rot);
                }
            }
            PlayerInput::Straighten(_) => {
                if !movement.is_turning() {
                    let (ey, _, _) = transform.rotation.to_euler(EulerRot::YXZ);
                    let new_rot = Quat::from_euler(EulerRot::YXZ, ey, 0., 0.).normalize();
                    movement.turn_toward(
                        settings.large_turn_time_secs,
                        transform.rotation,
                        new_rot,
                    );
                    look.turn_toward(settings.large_turn_time_secs, transform.rotation, new_rot);
                }
            }
            PlayerInput::ToggleCrouch(_entity) => {
                look.crouching = !look.crouching;
            }
            PlayerInput::StartFire(_) => {
                *next_fire_time = Some(Duration::ZERO);
            }
            PlayerInput::StopFire(_) => {
                *next_fire_time = None;
            }

            // Handled above.
            PlayerInput::Move(..) => (),
        }

        // // Fire before move.
        // if let Some(next_time) = next_fire_time.as_mut() {
        //     let left = next_time.saturating_sub(time.delta());
        //     if left.is_zero() {
        //         let eye_pos = player_eyes(&transform, aabb, &look);
        //         let gun_pos = player_gun(&transform, eye_pos);
        //         commands.spawn((
        //             Name::new("Projectile"),
        //             Transform::from_translation(gun_pos)
        //                 .with_rotation(look.rotation * Quat::from_rotation_x(-std::f32::consts::PI))
        //                 .with_scale(Vec3::ONE * 2.0),
        //             Projectile(ProjectileType::Bullet, input.player_entity()),
        //             LinearVelocity((look.rotation * Vec3::NEG_Z * 32.0).adjust_precision()),
        //         ));
        //         // *next_time = Duration::from_secs_f32(1.0 / 2.0);
        //         *next_fire_time = None;
        //     } else {
        //         *next_time = left;
        //     }
        // }
    }
}
