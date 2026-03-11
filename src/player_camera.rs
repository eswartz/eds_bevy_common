// Client-side camera behavior.

use std::any::TypeId;

use avian3d::math::Quaternion;
use avian3d::prelude::*;
use bevy::prelude::*;

#[cfg(feature = "input_lim")]
use leafwing_input_manager::prelude::ActionState;
#[cfg(feature = "input_bei")]
use bevy_enhanced_input::prelude::*;

#[cfg(feature = "input_lim")]
use crate::UserAction;
#[cfg(feature = "input_bei")]
use crate::actions_common_bei::actions::*;

use crate::player_client::OurPlayer;
use crate::*;


pub struct PlayerCameraPlugin;

impl Plugin for PlayerCameraPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<CameraMode>()
            .register_type::<OurCamera>()
            .register_type::<PlayerCameraSettings>()
            .init_resource::<PlayerCameraSettings>()
            .insert_resource(ViewerCameraAlignRate(0.125))
            .add_systems(FixedPreUpdate,
                (
                    handle_player_camera_actions,
                    sync_world_camera_to_player,
                    sync_view_camera_to_player,
                )
                .chain()
                .after(PhysicsSystems::Writeback)
                .before(TransformSystems::Propagate)
                .run_if(not(is_menu_paused))
                .run_if(is_game_active)
                ,
            )
            .add_systems(PostUpdate,
                update_player_ui
                .run_if(|gizmo_config: Res<GizmoConfigStore>| {
                    // Safely access without a panic.
                    if let Some((phys_gizmos, _)) = gizmo_config.get_config_dyn(&TypeId::of::<PhysicsGizmos>()) {
                        phys_gizmos.enabled
                    } else {
                        false
                    }
                })
                .run_if(not(is_menu_paused))
                .run_if(in_state(OverlayState::Hidden).or(in_state(OverlayState::DebugGuiVisible)))
            )
        ;
    }
}

#[derive(Resource, Debug, Clone, Reflect)]
#[reflect(Resource, Clone, Default)]
#[type_path = "game"]
pub struct PlayerCameraSettings {
    /// When set, move in direction you're looking, no matter the angle.
    /// Otherwise, align Y as up if possible.
    pub freecam: bool,
    /// When set, up/down movements are relative to rotation.
    pub move_up_down_abs: bool,
    /// Max roll (side-to-side, strafing) angle in degrees.
    pub roll_degrees: f32,
    /// Time to reach or decay roll target.
    pub roll_angle_time: f32,
    /// Max pitch (forward-back, running) angle in degrees.
    pub pitch_degrees: f32,
    /// Time to reach or decay pitch target.
    pub pitch_angle_time: f32,
    /// Max bob (up-down, moving) in meters.
    pub bob_distance: f32,
    /// Time to reach or decay bob target when walking, divided by speed.
    pub bob_time: f32,
   /// How long it takes for FOV to decay after the user starts zooming.
    pub fov_decay_time_secs: f32,
}

impl Default for PlayerCameraSettings {
    fn default() -> Self {
        Self {
            freecam: false,
            move_up_down_abs: true,
            roll_degrees: 1.0,
            roll_angle_time: 0.25,
            pitch_degrees: 2.0,
            pitch_angle_time: 0.5,
            bob_distance: 0.05,
            bob_time: 0.75,
            fov_decay_time_secs: 0.5,
        }
    }
}

/// How quickly we align the ViewerCamera to the WorldCamera.
#[derive(Resource, Default, Reflect)]
#[reflect(Default)]
#[type_path = "game"]
pub struct ViewerCameraAlignRate(pub f32);

/// This marks the Camera representing the player's point of view.
#[derive(Component, Default, Reflect)]
#[require(Saveable)]
#[reflect(Default)]
#[type_path = "game"]
pub struct OurCamera {
    /// Current dynamic roll, in radians
    pub roll_z_ang: f32,
    pub roll_z_dir: f32,
    pub roll_z_timer: f32,
    /// Current dynamic pitch, in radians
    pub pitch_x_ang: f32,
    pub pitch_x_dir: f32,
    pub pitch_x_timer: f32,
    /// Current dynamic bob, in meters
    pub bob_distance: f32,
    pub bob_movement: f32,
    pub bob_timer: f32,
}

impl OurCamera {

    pub fn adjust_bob_roll_pitch(
        &mut self,
        settings: &PlayerCameraSettings,
        mode: PlayerMode,
        dt: f32, fwd: f32, strafe: f32, speed: f32
    ) {

        let sign_or_zero = |v: f32| -> f32 {
            if v.abs() < 0.0001 { 0.0 } else { v.signum() }
        };
        let move_toward = |ease: EaseFunction, src: f32, dst: f32, time: f32, timer: &mut f32| -> f32 {
            *timer = (*timer + dt).min(time);
            let t = *timer / time;
            EasingCurve::new(src, dst, ease).sample_clamped(t)
        };
        if sign_or_zero(self.roll_z_dir) != sign_or_zero(-strafe) {
            self.roll_z_timer = 0.;
            self.roll_z_dir = -strafe;
        }

        let roll_angle_max = settings.roll_degrees.to_radians().abs();
        self.roll_z_ang = (if strafe != 0.0 {
            move_toward(
                EaseFunction::QuadraticOut,
                0.0,
                roll_angle_max * self.roll_z_dir,
                settings.roll_angle_time,
                &mut self.roll_z_timer)
        } else {
            move_toward(
                EaseFunction::ExponentialOut,
                roll_angle_max * self.roll_z_ang.signum(),
                0.0,
                settings.roll_angle_time,
                &mut self.roll_z_timer)

        }).clamp(-roll_angle_max, roll_angle_max);

        if self.roll_z_dir == 0. && self.roll_z_ang.abs() < 0.0001 {
            self.roll_z_ang = 0.;
        }

        if sign_or_zero(self.pitch_x_dir) != sign_or_zero(fwd) {
            self.pitch_x_timer = 0.;
            self.pitch_x_dir = fwd;
        }

        let pitch_max = settings.pitch_degrees.to_radians().abs();
        self.pitch_x_ang = (if fwd != 0.0 {
            move_toward(
                EaseFunction::QuadraticOut,
                0.0,
                pitch_max * self.pitch_x_dir,
                settings.pitch_angle_time,
                &mut self.pitch_x_timer)
        } else {
            move_toward(
                EaseFunction::ExponentialOut,
                pitch_max * self.pitch_x_ang.signum(),
                0.0,
                settings.pitch_angle_time,
                &mut self.pitch_x_timer)
        }).clamp(-pitch_max, pitch_max);

        if self.pitch_x_dir == 0. && self.pitch_x_ang.abs() < 0.0001 {
            self.pitch_x_ang = 0.;
        }

        if mode == PlayerMode::Fps {
            // let movement = Vec2::new(fwd, strafe).length() * speed.mul();
            if sign_or_zero(self.bob_movement) != sign_or_zero(speed) {
                self.bob_timer = 0.;
                self.bob_movement = speed;
            }

            if speed >= 0.25 {
                let bob_max = speed * settings.bob_distance.abs();
                self.bob_distance = ops::sin(self.bob_timer * std::f32::consts::TAU / settings.bob_time) * bob_max;
                self.bob_timer += dt;
            } else {
                self.bob_distance *= 0.5;
                if self.bob_distance.abs() < 0.0001 {
                    self.bob_distance = 0.;
                }
            }
        } else {
            self.bob_timer = 0.;
            self.bob_movement = 0.;
            self.bob_distance = 0.;
        }
    }
}

pub fn sync_world_camera_to_player(
    mut player_q: Single<(&Transform, &PlayerLook, &ColliderAabb, &mut Visibility), (With<OurPlayer>, Without<Camera3d>)>,
    mut world_camera_q: Single<(&PlayerCamera, &mut Transform, &OurCamera), (With<Camera3d>, With<WorldCamera>)>,
    time: Res<Time>,
) {
    let (player_xfrm, look, player_aabb, ref mut model_visibility) = *player_q;
    let (PlayerCamera(mode), ref mut camera_xfrm, cam) = *world_camera_q;

    // let q = (time.delta_secs() * 10.0).min(1.0);
    let q = (-0.5 * time.delta_secs() * 100.0).exp();
    // let q = 0.5;

    let eyes_pos = player_eyes(player_xfrm, player_aabb, look);
    match mode {
        CameraMode::FirstPerson => {
            model_visibility.set_if_neq(Visibility::Hidden);

            camera_xfrm.rotation = look.rotation
                * Quat::from_rotation_z(cam.roll_z_ang)
                * Quat::from_rotation_x(cam.pitch_x_ang)
                ;

            camera_xfrm.translation = eyes_pos
                + camera_xfrm.rotation * Vec3::Y * cam.bob_distance;

        }
        CameraMode::ThirdPerson => {
            model_visibility.set_if_neq(Visibility::Inherited);
            let cam_pos = eyes_pos
                + look.rotation * Vec3::new(0.0, 1.0, 5.0);
            let new_xfrm = camera_xfrm.looking_at(eyes_pos, Vec3::Y);
            camera_xfrm.translation = camera_xfrm.translation.lerp(cam_pos, q);
            camera_xfrm.rotation = camera_xfrm.rotation.slerp(new_xfrm.rotation, q);
        }
        CameraMode::LookingAt => {
            model_visibility.set_if_neq(Visibility::Inherited);
            let new_xfrm = camera_xfrm.looking_at(eyes_pos, Vec3::Y);
            camera_xfrm.rotation = camera_xfrm.rotation.slerp(new_xfrm.rotation, q);
        }
        CameraMode::Stationary => {
            model_visibility.set_if_neq(Visibility::Inherited);
        }
    };
}

pub fn sync_view_camera_to_player(
    // WorldCamera + ViewerCamera can be on the same.
    mut params: ParamSet<(
        Single<&Transform, (With<Camera3d>, With<WorldCamera>, With<OurCamera>)>,
        Single<&mut Transform, (With<Camera3d>, With<ViewerCamera>)>
    )>,
    align_rate: Res<ViewerCameraAlignRate>,
) {
    let camera_xfrm = params.p0().clone();
    let mut view_camera_q = params.p1();

    // View camera is always aligned to world camera.
    // **view_camera_q = **camera_xfrm;
    view_camera_q.translation = camera_xfrm.translation;

    // Slowly align but don't tilt.
    let target_rot = view_camera_q.rotation.lerp(camera_xfrm.rotation, align_rate.0);
    let (ex, ey, _ez) = target_rot.to_euler(default());
    let target_rot = Quaternion::from_euler(default(), ex, ey, 0.0);
    view_camera_q.rotation = target_rot;
}

#[cfg(feature = "input_lim")]
pub fn handle_player_camera_actions(
    action_state: Res<ActionState<UserAction>>,
    mut camera_q: Single<&mut PlayerCamera, (With<WorldCamera>, With<OurCamera>)>,
) {
    if action_state.just_pressed(&UserAction::ChangeCamera) {
        camera_q.0 = camera_q.0.next();
    }
}

#[cfg(feature = "input_bei")]
pub fn handle_player_camera_actions(
    change_camera: Single<&ActionEvents, With<Action<ChangeCamera>>>,
    mut camera_q: Single<&mut PlayerCamera, (With<WorldCamera>, With<OurCamera>)>,
) {
    if change_camera.contains(ActionEvents::START) {
        camera_q.0 = camera_q.0.next();
    }
}

pub fn show_3d_camera(mut camera_q: Query<&mut Camera, (With<Camera3d>, With<OurCamera>)>) {
    for mut camera in camera_q.iter_mut() {
        camera.is_active = true;
    }
}

pub fn hide_3d_camera(mut camera_q: Query<&mut Camera, (With<Camera3d>, With<OurCamera>)>) {
    for mut camera in camera_q.iter_mut() {
        camera.is_active = false;
    }
}

pub fn update_player_ui(
    mut player_q: Query<(&PlayerMovement, &PlayerLook, &Transform, &ColliderAabb), With<OurPlayer>>,
    mut gizmos: Gizmos,
) {
    for (movement, look, transform, aabb) in player_q.iter_mut() {
        // Show where we're looking.
        let head = player_eyes(transform, aabb, look);
        let normal = look.rotation * Vec3::NEG_Z;
        gizmos.arrow(
            head,
            head + normal * 5.0,
            Color::WHITE,
        );

        if movement.state == MovementState::Grounded {
            let feet = player_feet(transform, aabb);
            gizmos.circle(
                Isometry3d::new(feet, Quat::from_rotation_x(std::f32::consts::PI / 2.0)),
                1.0,
                Color::WHITE,
            );
        }
    }
}
