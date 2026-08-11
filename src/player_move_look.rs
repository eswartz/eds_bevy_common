/// Server-side player movement.
use std::time::Duration;

use bevy::ecs::system::{SystemParam, lifetimeless};
use bevy::prelude::*;

use crate::physics::*;
use crate::prelude::*;

pub struct PlayerMovementPlugin;

impl Plugin for PlayerMovementPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StationaryCameraTransform>()
            .add_systems(
            FixedPreUpdate,
            (
                clear_player_velocity
                    .run_if(window_changed_focus.or_else(resource_changed::<PlayerMode>)),
                process_player_input_movement,
                // process_player_input_movement_for_fps
                //     .run_if(not(is_cheating))
                //     .run_if(resource_exists_and_equals(PlayerMode::Fps))
                // ,
                // process_player_input_movement_for_space
                //     .run_if(not(is_cheating))
                //     .run_if(resource_exists_and_equals(PlayerMode::Space))
                // ,
                process_player_input_look,
                process_player_input_misc.run_if(not(is_grabbing_item)),
                sync_player_movement,
            )
                .chain()
                .before(TransformSystems::Propagate)
                .after(PhysicsSystems::Writeback)
                .run_if(not(is_paused))
                .run_if(in_state(GameplayState::Playing)),
        )
        .add_systems(
            FixedPreUpdate,
            (process_player_input_movement_for_cheats.run_if(is_cheating_or_paused),)
                .chain()
                .before(TransformSystems::Propagate)
                .after(PhysicsSystems::Writeback)
                .run_if(in_state(GameplayState::Playing)),
        );
    }
}

fn is_cheating_or_paused(physics_paused: Res<PhysicsPaused>) -> bool {
    is_cheating() || **physics_paused
}

fn is_cheating() -> bool {
    false
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
    /// How much movement is reduced when crouch-moving.
    pub crouch_scale: f32,
    /// How slowly movement is accelerated when shift-moving.
    pub velocity_ramp_scale: f32,
    /// When set, up/down movements are relative to rotation.
    pub move_up_down_abs: bool,
    /// How movement is scaled in air (i.e. usually < 1.0).
    pub air_scale: f32,
    /// Velocity scale for X/Z movement (m/s).
    pub base_xz_speed: u8,
    /// Velocity scale for jump (m/s).
    pub jump_accel: f32,
    /// Allow this many jumps.
    pub jump_max_count: u16,
    /// Maximum speed for X/Z movement (m/s).
    pub max_xz_speed: f32,
    /// Maximum speed for +Y movement (m/s).
    pub max_up_speed: f32,
    /// Maximum speed for -Y movement (m/s).
    pub max_down_speed: f32,
    /// Y velocity to consider "not falling or flying".
    pub grounded_y_speed: f32,
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
            crouch_scale: 0.5,
            move_up_down_abs: true,

            base_xz_speed: 8,
            jump_accel: 256.0,
            jump_max_count: 2,
            max_xz_speed: 16.0,
            max_up_speed: 96.0,
            max_down_speed: 96.0, // b/t 55 m/s for skydiver, 150 m/s competition
            crouch_depth: 0.5,
            grounded_y_speed: 1.0,
            air_scale: 0.75,

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
            crouch_scale: 1.0,
            move_up_down_abs: false,

            base_xz_speed: 8,
            jump_accel: 256.0,
            jump_max_count: 256,
            max_xz_speed: 32.0,
            max_up_speed: 128.0,
            max_down_speed: 128.0,
            crouch_depth: 0.0,
            grounded_y_speed: 0.0,
            air_scale: 0.99,

            movement_decay_time_secs: 1.0,
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
    pub fn is_moving(&self) -> bool {
        matches!(
            *self,
            MovementState::Walking | MovementState::Running | MovementState::OnSlope
        )
    }

    pub fn to_grounded(self) -> MovementState {
        match self {
            MovementState::Grounded
            | MovementState::Walking
            | MovementState::Running
            | MovementState::Scripted => self,
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
    /// Current velocity. (FIXME, not used)
    // pub velocity: f32,
    pub velocity: Vec3,
    /// Current velocity rampup.
    pub velocity_ramp: f32,
    /// Current state.
    pub state: MovementState,
    /// Previous state for purposes of sound.
    pub prev_state: MovementState,

    /// Represents how dense is the medium the player is in.
    /// I.e. 0.0 means empty space, 1.0 means encased on rock.
    pub medium_friction: f32,
    /// When set, player issued a jump in the previous frame.
    pub had_jump_event: bool,
    /// Counts how many player jumps are allowed still.
    /// (Decremented form a start [PlayerInputSettings::jump_count].
    pub allowed_jumps: u16,
    pub jumping_out: bool,

    pub turn_time_secs: f32,
    pub turn_deadline_secs: f32,
    pub turn_curve: Option<EasingCurve<Quat>>,
    /// Current area of feet.
    pub area: AreaContent,
}

impl Default for PlayerMovement {
    fn default() -> Self {
        Self {
            // velocity: 0.0,
            velocity: default(),
            velocity_ramp: 0.0,
            state: MovementState::Falling,
            prev_state: MovementState::Falling,
            medium_friction: 1.0,
            had_jump_event: false,
            allowed_jumps: 0,
            jumping_out: false,
            turn_time_secs: 0.0,
            turn_deadline_secs: 0.0,
            turn_curve: None,
            area: AreaContent::Air,
        }
    }
}

impl PlayerMovement {
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

    pub fn apply_turn(&mut self, dt: f32, rot_delta: Vec3, transform: &mut Transform) -> bool {
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
                    let lim = std::f32::consts::FRAC_PI_2 * 1.0;
                    look_angles.x = look_angles.x.clamp(-lim, lim);
                    look_angles.y %= std::f32::consts::TAU;
                    look_angles.z = look_angles.z.clamp(-lim, lim);
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
    /// Camera is inside player's head.
    #[default]
    FirstPerson,
    /// Camera follows over player's shoulder.
    ThirdPerson,
    /// Looking at player.
    LookingAt,
    /// Camera fixed at [StationaryCameraTransform].
    Stationary,
}

/// This serves as the world camera position for [CameraMode::Stationary].
#[derive(Resource, Debug, Default, Clone, PartialEq, Reflect, Deref, DerefMut)]
#[reflect(Resource, Clone, Default)]
#[type_path = "game"]
pub struct StationaryCameraTransform(pub Transform);

#[derive(Debug, Component, Reflect, Clone)]
#[require(Saveable)]
#[reflect(Component, Clone, Default)]
#[type_path = "game"]
pub struct PlayerLook {
    /// Where we're looking.
    pub rotation: Quat,
    /// Previous rotation, for use in FOV detection.
    pub prev_rotation: Quat,
    /// Current dynamic crouch distance (moving eyes down).
    pub crouch_y: f32,
    pub turn_time_secs: f32,
    pub turn_deadline_secs: f32,
    pub turn_curve: Option<EasingCurve<Quat>>,
    pub crouching: bool,
}

impl Default for PlayerLook {
    fn default() -> Self {
        Self {
            rotation: default(),
            prev_rotation: default(),
            crouch_y: 0.0,
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
        self.prev_rotation = self.rotation;

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
                    let lim = std::f32::consts::FRAC_PI_2 * 0.999;
                    look_angles.x = look_angles.x.clamp(-lim, lim);
                    look_angles.y %= std::f32::consts::TAU;
                    look_angles.z = look_angles.z.clamp(-lim, lim);
                    Quat::from_euler(EulerRot::YXZ, look_angles.y, look_angles.x, look_angles.z)
                };

                self.rotation = new_quat;
            }
            false
        }
    }
}

/// Stop moving player, e.g. when input is going to UI.
pub fn clear_player_velocity(mut player_q: Query<&mut LinearVelocity, With<PlayerMovement>>) {
    for mut vel in player_q.iter_mut() {
        vel.0 = Vector::ZERO;
    }
}

#[derive(SystemParam)]
pub struct PlayerInputParams<'w, 's> {
    // player_q: Query<'w, 's, Read<PlayerMovement>, With<Player>>,
    // projectile_q: Query<'w, 's, (), With<Projectile>>,
    // parent_q: Query<'w, 's, Read<ChildOf>>,

    pub player_q: Query<'w, 's,
        (
            Forces,
            lifetimeless::Write<PlayerMovement>,
            lifetimeless::Write<PlayerLook>,
            lifetimeless::Write<Transform>,
        ),
        With<Player>,
    >,

    pub time: Res<'w, Time>,
    pub input_settings: Res<'w, PlayerInputSettings>,
    // pub forces: Query<'w, 's, Forces>,
    // pub movement: Query<'w, 's, &'static mut PlayerMovement>,
    // pub look: Query<'w, 's, &'static mut PlayerLook>,
    // pub transform: Query<'w, 's, &'static mut Transform>,
}

/// Implement to
pub trait PlayerInputHandler: Send + Sync + 'static {
    fn handle(&mut self,
        entity: Entity,
        event: &PlayerInput,
        params: &PlayerInputParams<'_, '_>,
    ) -> bool;
}

#[derive(Resource)]
pub struct PlayerInputHandlers {
    pub common: Vec<Box<dyn PlayerInputHandler>>,
    pub fps: Vec<Box<dyn PlayerInputHandler>>,
    pub space: Vec<Box<dyn PlayerInputHandler>>,
    pub scripted: Vec<Box<dyn PlayerInputHandler>>,
}


pub fn process_player_input_movement(
    mut params: PlayerInputParams,
    // input_settings: Res<PlayerInputSettings>,
    // handlers: Res<PlayerInputHandlers>,
    mut inputs: MessageReader<PlayerInput>,
    mode: Option<Res<PlayerMode>>,
) {
    let player_mode = mode.map_or(PlayerMode::Fps, |mode| *mode);

    let dt = params.time.delta_secs();
    for input in inputs.read() {
        let player_entity = input.player_entity();
        let res = params.player_q.get_mut(player_entity);

        let Ok((mut forces, mut movement, mut look, mut transform)) = res else {
            continue
        };
        // let Ok(mut forces) = params.forces.get_mut(player_entity) else {
        //     continue
        // };

        // for mut handler in &mut handlers.common {
        //     handler.handle(player_entity, input, &*input_settings);
        // }

        let input_settings = &*params.input_settings;

        let mut vel = forces.linear_velocity();
        // let mut vel = forces.linear_velocity() + movement.velocity;
        let mut jump_impulse = Vector::ZERO;

        let mut instant_thrust = Vec3::ZERO;
        let mut overall_speed = input_settings.base_xz_speed as f32;

        match input {
            PlayerInput::Move(..) if movement.state == MovementState::Scripted => {
                // Ignore.
            }

            PlayerInput::Move(_, input) => {
                instant_thrust.x =
                    Into::<f32>::into(input.right_left) * input_settings.move_scale.x;
                instant_thrust.y = Into::<f32>::into(input.up_down) * input_settings.move_scale.y;
                instant_thrust.z =
                    Into::<f32>::into(input.forward_back) * input_settings.move_scale.z;

                // Extract up/down.
                let mut up_down = instant_thrust.y;
                if input_settings.move_up_down_abs {
                    instant_thrust.y = 0.0;
                }

                instant_thrust = instant_thrust.clamp_length_max(1.0);
                let speed_type = if !look.crouching {
                    input.speed
                } else {
                    input.speed.slower()
                };
                let move_scale = match speed_type {
                    Speed::Fast => input_settings.accelerate_scale,
                    Speed::Slow => input_settings.crouch_scale,
                    Speed::Crawl => input_settings.crouch_scale * 0.5,
                    Speed::Normal => 1.0,
                };
                overall_speed *= move_scale;

                if instant_thrust == Vec3::ZERO {
                    movement.velocity_ramp = 0.0;
                }
                movement.velocity_ramp = (movement.velocity_ramp
                    + input_settings.velocity_ramp_scale * move_scale)
                    .clamp(0.0, 1.0);

                let mut dir_velocity = transform.rotation * instant_thrust * movement.velocity_ramp;

                const MAX_JUMP_MEDIUM_FRICTION: f32 = 0.25;

                // See if we could jump.
                let jump_grounded = movement.state.is_on_surface()   // but not OnSlope
                    && movement.medium_friction >= MAX_JUMP_MEDIUM_FRICTION;
                let extra_jump_allowed =
                    vel.y >= 0. && input_settings.jump_max_count > 1 && movement.allowed_jumps > 0;

                // Do we want to?
                if up_down > 0. {
                    if (jump_grounded || extra_jump_allowed) && !movement.had_jump_event {
                        movement.had_jump_event = true;
                        movement.allowed_jumps = movement.allowed_jumps.saturating_sub(1);
                        let sluggishness = move_scale.min(1.0);
                        // Jump strictly up.
                        jump_impulse = Vector::new(
                            0.,
                            input_settings.jump_accel as Scalar * sluggishness as Scalar,
                            0.,
                        );
                        movement.state = MovementState::Jumping;
                    }
                    // Consume for jump or failed re-jump.
                    up_down = 0.;
                } else {
                    movement.had_jump_event = false;
                    if jump_grounded {
                        movement.allowed_jumps = input_settings.jump_max_count;
                    }
                }

                if up_down == 0. && vel.y > 0. && movement.state == MovementState::Flying {
                    // HACK: Since we're using physics for the character, we can sometimes "fly"
                    // just by running across a bump. Correct for that with prejudice.
                    vel.y = 0.0;
                    movement.state = movement.state.to_grounded();
                }

                // Apply unconsumed strict up/down movement.
                if up_down != 0. && input_settings.move_up_down_abs {
                    dir_velocity.y = up_down;
                }

                let dir_velocity = dir_velocity * Vec3::new(overall_speed, 1.0, overall_speed);
                if dir_velocity.length_squared() > 0.01 {
                    if movement.state.is_on_surface() {
                        vel.x = (vel.x + dir_velocity.x as Scalar) / 2.0;
                        vel.z = (vel.z + dir_velocity.z as Scalar) / 2.0;
                    } else {
                        let asc = input_settings.air_scale as Scalar;
                        let bs = (input_settings.base_xz_speed as Scalar) * asc;
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
                        let decay = (-0.5 * dt
                            / input_settings.movement_decay_time_secs
                            / move_scale)
                            .exp() as Scalar;

                        vel = Vector::new(vel.x * decay, vel.y, vel.z * decay);
                    }
                }
            }

            PlayerInput::HeadTurn(..)
            | PlayerInput::BodyTurn(..)
            | PlayerInput::TurnAround(..)
            | PlayerInput::Straighten(_)
            | PlayerInput::ToggleCrouch(..)
            | PlayerInput::StartFire(_)
            | PlayerInput::StopFire(_) => {
                // Ignore.
            }
        }

        // Apply any scripted movement.
        movement.apply_turn(dt, Vec3::ZERO, &mut transform);
        look.apply_turn(dt, Vec3::ZERO);

        // Crouch.
        look.crouch_y = look.crouch_y * 0.9
            - if look.crouching {
                input_settings.crouch_depth
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
            cur_vel_xz.clamp_length_max(input_settings.max_xz_speed as Scalar)
        };
        let clamped_y = vel.y.clamp(
            -(input_settings.max_down_speed as Scalar), // i.e. air/fluid resistance
            input_settings.max_up_speed as Scalar,      // i.e. flying/jumping
        );

        // Don't try to interpolate here.
        let clamped_vel = Vector::new(clamped_vel_xz.x, clamped_y, clamped_vel_xz.y);
        *forces.linear_velocity_mut() = clamped_vel;

        // Add this force's contribution afterwards.
        forces.apply_linear_impulse(jump_impulse);

        if movement.state.is_on_surface() {
            let eff_speed: f32 = vel.xz().length();
            if eff_speed > input_settings.base_xz_speed as f32 {
                movement.state = MovementState::Running;
            } else if eff_speed >= input_settings.base_xz_speed as f32 / 2.0 {
                movement.state = MovementState::Walking;
            } else {
                movement.state = MovementState::Grounded;
            }
        }
    }
}

pub fn process_player_input_movement_for_cheats(
    mut player_q: Query<(Forces, &mut PlayerMovement, &PlayerLook, &mut Transform), With<Player>>,
    mut inputs: MessageReader<PlayerInput>,
    time: Res<Time>,
    input_settings: Res<PlayerInputSettings>,
) {
    for input in inputs.read() {
        let res = player_q.get_mut(input.player_entity());

        let Ok((mut forces, mut movement, look, mut transform)) = res else {
            continue;
        };

        let mut vel = forces.linear_velocity();

        let mut instant_thrust = Vec3::ZERO;
        let mut overall_speed = input_settings.base_xz_speed as f32;
        if let PlayerInput::Move(_, input) = input {
            instant_thrust.x = Into::<f32>::into(input.right_left) * input_settings.move_scale.x;
            instant_thrust.y = Into::<f32>::into(input.up_down) * input_settings.move_scale.y;
            instant_thrust.z = Into::<f32>::into(input.forward_back) * input_settings.move_scale.z;

            instant_thrust = instant_thrust.clamp_length_max(2.0);

            let move_speed = if !look.crouching {
                input.speed
            } else {
                input.speed.slower()
            };
            let accel_scale = match move_speed {
                Speed::Fast => input_settings.accelerate_scale,
                Speed::Slow => input_settings.crouch_scale,
                Speed::Crawl => input_settings.crouch_scale * 0.5,
                Speed::Normal => 1.0,
            };
            overall_speed *= accel_scale;

            let dir_velocity = look.rotation * instant_thrust;

            let delta = dir_velocity * overall_speed;
            if delta.length_squared() > 0.01 {
                // Go!
                vel = delta;
            } else {
                // Slow down when not actively moving.
                let decay = (-0.5 * time.delta_secs()
                    / input_settings.movement_decay_time_secs
                    / accel_scale)
                    .exp() as Scalar;
                vel = Vector::new(vel.x * decay, vel.y * decay, vel.z * decay);
            }
        }

        // Clamp speed.
        let cur_len = vel.length();
        let clamped_vel = if cur_len < 0.1 {
            movement.velocity_ramp = 0.0;
            Vector3::splat(0.0)
        } else {
            vel.clamp_length_max(input_settings.max_xz_speed as Scalar)
        };

        if true
        /* !**physics_paused */
        {
            *forces.linear_velocity_mut() = clamped_vel;
        } else {
            transform.translation += clamped_vel * time.delta_secs();
        }
    }
}

impl PlayerInputHandlers {
    pub fn new() -> Self {
        Self {
            common: vec![

            ],
            fps: vec![

            ],
            space: vec![

            ],
            scripted: vec![

            ],
        }
    }
}

pub fn process_player_input_look(
    mut player_q: Query<(&mut PlayerMovement, &mut PlayerLook, &mut Transform), With<Player>>,
    mut inputs: MessageReader<PlayerInput>,
    time: Res<Time>,
    settings: Res<PlayerInputSettings>,
) {
    let dt = time.delta_secs();
    for input in inputs.read() {
        let res = player_q.get_mut(input.player_entity());

        let Ok((mut movement, mut look, mut transform)) = res else {
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
            _ => (),
        }
    }
}

pub fn process_player_input_misc(
    mut player_q: Query<(&mut PlayerMovement, &mut PlayerLook, &Transform), With<Player>>,
    mut inputs: MessageReader<PlayerInput>,
    settings: Res<PlayerInputSettings>,
    mut next_fire_time: Local<Option<Duration>>,
) {
    for input in inputs.read() {
        let res = player_q.get_mut(input.player_entity());

        let Ok((mut movement, mut look, transform)) = res else {
            continue;
        };

        match input {
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
            PlayerInput::HeadTurn(..) => (),
            PlayerInput::BodyTurn(..) => (),
        }
    }
}

/// When physics is disabled, directly apply movement to the player.
fn sync_player_movement(
    mut player_q: Query<
        (
            &mut Transform,
            &mut Position,
            &mut LinearVelocity,
            Option<&GravityScale>,
        ),
        With<Player>,
    >,
    grav: Res<Gravity>,
    time: Res<Time>,
    physics_paused: Res<PhysicsPaused>,
) {
    if !**physics_paused {
        return;
    }

    for (mut xfrm, mut pos, mut vel, grav_opt) in player_q.iter_mut() {
        let orig = xfrm.translation;
        let offs = vel.0;
        let offs = offs + grav_opt.map_or(1.0, |g| **g) * grav.0;
        let xfrm_delta = offs * time.delta_secs();
        xfrm.translation += xfrm_delta;
        pos.x = xfrm.translation.x;
        // pos.y = xfrm.translation.y;
        pos.y = orig.y;
        pos.z = xfrm.translation.z;
        *vel = default();
    }
}
