// Client-side camera behavior.

use std::any::TypeId;
use std::time::Duration;

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
        app
            .init_resource::<PlayerCameraSettings>()
            .init_resource::<FovZoomState>()
            .add_systems(FixedPreUpdate,
                (
                    // HACK: we "know" zoom and move-while-grabbed use the same actions
                    handle_player_camera_actions.run_if(not(is_grabbing_item)),
                    sync_world_camera_to_player,
                    sync_view_camera_to_player,
                )
                .chain()
                .after(PhysicsSystems::Writeback)
                .before(TransformSystems::Propagate)
                .run_if(not(is_menu_paused))
                .run_if(not(debug_gui_wants_input))
                .run_if(is_game_active)
                ,
            )
            .add_systems(FixedUpdate,
                decay_camera_zoom
                .run_if(not(is_grabbing_item))
                .run_if(not(is_menu_paused))
                .run_if(not(debug_gui_wants_input))
                .run_if(is_game_active)
                ,
            )
            .add_systems(PostUpdate,
                update_player_ui
                .run_if(|gizmo_config: Res<GizmoConfigStore>| {
                    // Safely access without a panic if no debug UI logged.
                    if let Some((phys_gizmos, _)) = gizmo_config.get_config_dyn(&TypeId::of::<PhysicsGizmos>()) {
                        phys_gizmos.enabled
                    } else {
                        false
                    }
                })
                .run_if(not(is_menu_paused))
                .run_if(in_state(OverlayState::Hidden)) //.or(in_state(OverlayState::DebugGuiVisible)))
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
    pub roll_angle_time: Duration,
    /// Max pitch (forward-back, running) angle in degrees.
    pub pitch_degrees: f32,
    /// Time to reach or decay pitch target.
    pub pitch_angle_time: Duration,
    /// Max bob (up-down, moving) in meters.
    pub bob_distance: f32,
    /// Time to reach or decay bob target when walking, divided by speed.
    pub bob_time: Duration,
    /// How quickly we align the ViewerCamera to the WorldCamera.
    /// This is used when there is an alternate camera view (3rd person)
    pub viewer_camera_align_time: Duration,
    /// How long the FOV delta sticks after the user stops zooming.
    pub fov_delta_hold_time: Duration,
    /// How long it takes for FOV delta to decay after the user stops zooming.
    pub fov_delta_decay_time: Duration,
}

impl Default for PlayerCameraSettings {
    fn default() -> Self {
        Self {
            freecam: false,
            move_up_down_abs: true,
            roll_degrees: 1.0,
            roll_angle_time: Duration::from_secs_f32(0.25),
            pitch_degrees: 2.0,
            pitch_angle_time: Duration::from_secs_f32(0.5),
            bob_distance: 0.05,
            bob_time: Duration::from_secs_f32(0.75),
            viewer_camera_align_time: Duration::from_secs_f32(0.125),
            fov_delta_hold_time: Duration::from_secs_f32(5.0),
            fov_delta_decay_time: Duration::from_secs_f32(0.5),
        }
    }
}

/// Current zooming state.
#[derive(Resource, Debug, Default, Reflect, PartialEq)]
#[reflect(Resource, Default)]
#[type_path = "game"]
pub enum FovZoomState {
    /// Not zooming at all.
    #[default]
    Inactive,
    /// Actions or scripting is changing zoom.
    Zooming,
    /// User stopped zooming.
    /// The duration is initialized from [PlayerCameraSettings::fov_delta_hold_time]
    /// and counts down.
    AtZoom(Duration),
    /// Currently restoring zoom back to 0.
    Unzooming,
}

/// This marks the Camera representing the player's point of view.
///
/// These values are aesthetic adjustments to the "true"
/// rotation and position of the camera used to simulate
/// an e.g. human head moving on a body.
#[derive(Component, Default, Reflect)]
#[require(Saveable)]
#[reflect(Component, Default)]
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
                settings.roll_angle_time.as_secs_f32(),
                &mut self.roll_z_timer)
        } else {
            move_toward(
                EaseFunction::ExponentialOut,
                roll_angle_max * self.roll_z_ang.signum(),
                0.0,
                settings.roll_angle_time.as_secs_f32(),
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
                settings.pitch_angle_time.as_secs_f32(),
                &mut self.pitch_x_timer)
        } else {
            move_toward(
                EaseFunction::ExponentialOut,
                pitch_max * self.pitch_x_ang.signum(),
                0.0,
                settings.pitch_angle_time.as_secs_f32(),
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
                self.bob_distance = ops::sin(self.bob_timer * std::f32::consts::TAU / settings.bob_time.as_secs_f32()) * bob_max;
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
    settings: Res<PlayerCameraSettings>,
) {
    let camera_xfrm = params.p0().clone();
    let mut view_camera_q = params.p1();

    // View camera is always aligned to world camera.
    // **view_camera_q = **camera_xfrm;
    view_camera_q.translation = camera_xfrm.translation;

    // Slowly align but don't tilt.
    let target_rot = view_camera_q.rotation.lerp(camera_xfrm.rotation, settings.viewer_camera_align_time.as_secs_f32());
    let (ex, ey, _ez) = target_rot.to_euler(default());
    let target_rot = Quat::from_euler(default(), ex, ey, 0.0);
    view_camera_q.rotation = target_rot;
}

pub fn handle_player_camera_actions(
    #[cfg(feature = "input_lim")]
    action_state: Res<ActionState<UserAction>>,
    #[cfg(feature = "input_bei")]
    change_camera: Query<&ActionEvents, (With<Action<ChangeCamera>>, With<PlayerAction>)>,
    mut camera_q: Single<&mut PlayerCamera, (With<WorldCamera>, With<OurCamera>)>,
    #[cfg(feature = "input_bei")]
    zoom_camera: Query<&Action<Zoom>, (With<PlayerAction>,)>,
    mut fov_delta: ResMut<FovDelta>,
    mut zoom_state: ResMut<FovZoomState>,
    settings: Res<PlayerCameraSettings>,
) {
    #[cfg(feature = "input_lim")]
    {
        if action_state.just_pressed(&UserAction::ChangeCamera) {
            camera_q.0 = camera_q.0.next();
        }
    }
    #[cfg(feature = "input_bei")]
    {
        if let Some(change_camera) = change_camera.iter().next() {
            if change_camera.contains(ActionEvents::START) {
                camera_q.0 = camera_q.0.next();
            }
        }
        if let Some(zoom_camera) = zoom_camera.iter().next() {
            if zoom_camera.length() > 0. {
                **fov_delta = (**fov_delta + zoom_camera.y).clamp(-90.0, 90.0);
                *zoom_state = FovZoomState::Zooming;
            } else {
                if *zoom_state == FovZoomState::Zooming {
                    // No longer zooming, reset eventually.
                    *zoom_state = FovZoomState::AtZoom(settings.fov_delta_hold_time)
                }
            }
        }
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

fn decay_camera_zoom(
    settings: Res<PlayerCameraSettings>,
    time: Res<Time>,
    mut zoom_state: ResMut<FovZoomState>,
    mut fov_delta: ResMut<FovDelta>,

    player_q: Query<&PlayerMovement, With<OurPlayer>,>,
) {
    // In a zoom?
    if let FovZoomState::AtZoom(decay) = *zoom_state {
        // Did the user move?
        if let Some(movement) = player_q.iter().next()
        && movement.state.is_moving() {
            *zoom_state = FovZoomState::Unzooming;
        } else {
            // Time to decay?
            let decay = decay.saturating_sub(time.delta());
            if decay.is_zero() {
                *zoom_state = FovZoomState::Unzooming;
            } else {
                *zoom_state = FovZoomState::AtZoom(decay);
            }
        }
    }
    if *zoom_state != FovZoomState::Unzooming {
        return
    }

    // Do we reset immediately?
    if settings.fov_delta_decay_time.is_zero() {
        **fov_delta = 0.;
        *zoom_state = FovZoomState::Inactive;
        return
    }

    // Else, slowly move back towards 0.
    let q = time.delta().div_duration_f32(settings.fov_delta_decay_time);
    **fov_delta = **fov_delta * ops::exp(-q);

    // Back at start?
    if fov_delta.abs() < 0.01 {
        *zoom_state = FovZoomState::Inactive;
    }
}
