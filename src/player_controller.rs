use bevy::prelude::*;
use avian3d::prelude::*;
use avian3d::math::*;
use bevy::window::PrimaryWindow;
use bevy::window::WindowFocused;

#[cfg(feature = "input_bei")]
use bevy_enhanced_input::prelude::*;

#[cfg(feature = "input_bei")]
use crate::actions_common_bei::actions::*;

use crate::*;

/// This plugin monitors user input and sends PlayerInput events.
pub struct PlayerControllerPlugin;

impl Plugin for PlayerControllerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerControllerSettings>();

        #[cfg(feature = "input_bei")]
        app.add_systems(
                FixedPreUpdate,
                (
                    collect_player_movement,
                    collect_player_look,
                    collect_player_input,
                )
                .run_if(not(is_paused))
                .run_if(not(debug_gui_wants_direct_input))
            );
    }
}

#[derive(Resource, Debug, Clone, Reflect)]
#[reflect(Resource, Clone, Default)]
#[type_path = "game"]
pub struct PlayerControllerSettings {
    pub center_mouse: bool,
    /// This controls the scaling of the player move speed to units of Euler rotation.
    /// Defaults to Vec3::ONE.
    /// multiplier, +ve
    pub move_scale: Vec3,
    /// This controls the scaling of the player turn speed to degrees of Euler rotation.
    /// Defaults to Vec3::ONE.
    /// Positive values preferred.
    ///
    /// multiplier, +ve
    pub turn_scale: Vec3,
    /// This controls the scaling of the player zoom speed and direction to degrees of Euler rotation.
    /// Defaults to Vec3::ONE.
    /// multiplier, +ve
    pub zoom_scale: Vec3,
    pub invert_turn_x: bool,
    pub invert_turn_y: bool,
    pub invert_zoom_y: bool,
}

impl Default for PlayerControllerSettings {
    fn default() -> Self {
        Self {
            center_mouse: true,
            invert_turn_x: false,
            invert_turn_y: false,
            invert_zoom_y: false,
            move_scale: Vec3::ONE,
            turn_scale: Vec3::ONE,
            zoom_scale: Vec3::ONE,
        }
    }
}

impl PlayerControllerSettings {
    pub fn with_center_mouse(self, center_mouse: bool) -> Self {
        Self {
            center_mouse,
            ..self
        }
    }
}

/// Handles movement from inputs.
///
/// We gather relevant inputs and send events indicating our intent.
#[cfg(feature = "input_bei")]
fn collect_player_movement(
    accel_events: Query<&ActionEvents, (With<Action<Accelerate>>, With<PlayerAction>)>,
    crouch_events: Query<&ActionEvents, (With<Action<Crouch>>, With<PlayerAction>)>,
    jump_events: Query<&ActionEvents, (With<Action<Jump>>, With<PlayerAction>)>,
    move_flycam: Query<&Action<MoveFlycam>, With<PlayerAction>>,
    move_down_up: Query<&Action<MoveDownUp>, With<PlayerAction>>,
    move_left_right: Query<&Action<MoveLeftRight>, With<PlayerAction>>,

    ctrl_settings: Res<PlayerControllerSettings>,
    input_settings: Res<PlayerInputSettings>,
    cam_settings: Res<PlayerCameraSettings>,
    mut writer: MessageWriter<PlayerInput>,
    player_vel_q: Single<(Entity, &LinearVelocity), With<OurPlayer>>,
    mut cam_q: Single<&mut OurCamera, With<WorldCamera>>,
    time: Res<Time>,
    mode: Res<PlayerMode>,
) {
    let mut instant_thrust = Vec3::ZERO;

    let speed = if accel_events.iter().next().unwrap().contains(ActionEvents::START | ActionEvents::FIRE) {
        Speed::Fast
    } else if crouch_events.iter().next().unwrap().contains(ActionEvents::START | ActionEvents::FIRE) {
        Speed::Slow
    } else {
        Speed::Normal
    };

    let move_axis = **move_flycam.iter().next().unwrap();
    let down_up_axis = **move_down_up.iter().next().unwrap();
    let left_right_axis = **move_left_right.iter().next().unwrap();
    instant_thrust.x = (left_right_axis + move_axis.x) * ctrl_settings.move_scale.x;
    instant_thrust.y = down_up_axis * ctrl_settings.move_scale.y;
    instant_thrust.z = move_axis.y * ctrl_settings.move_scale.z;

    if jump_events.iter().next().unwrap().contains(ActionEvents::START | ActionEvents::FIRE) {
        instant_thrust.y += ctrl_settings.move_scale.y;
    }

    let (player, vel) = *player_vel_q;

    // For bob, apply the actual speed, not the intended speed.
    let actual_speed = vel.xz().length() / input_settings.base_xz_speed as Scalar;

    cam_q.adjust_bob_roll_pitch(
        &cam_settings,
        *mode,
        time.delta_secs(),
        instant_thrust.z,
        instant_thrust.x,
        actual_speed as _,
    );

    if crouch_events.iter().next().unwrap().contains(ActionEvents::START) {
        writer.write(PlayerInput::ToggleCrouch(player));
    }
    writer.write(PlayerInput::Move(
        player,
        PlayerMove::new(instant_thrust, speed),
    ));
}

/// Handles looking around.
///
/// We gather relevant inputs and send events indicating our intent.
#[cfg(feature = "input_bei")]
fn collect_player_look(
    mut primary_window: Query<&mut Window, With<PrimaryWindow>>,

    look: Query<&Action<Look>, (With<Action<Look>>, With<PlayerAction>)>,
    turn_around_events: Query<&ActionEvents, (With<Action<TurnAround>>, With<PlayerAction>)>,
    reset_events: Query<&ActionEvents, (With<Action<Reset>>, With<PlayerAction>)>,
    // alt_fire_events: Query<&ActionEvents, (With<Action<Firing>>, With<PlayerAction>)>,
    mouse_button_events: Res<ButtonInput<MouseButton>>,

    settings: Res<PlayerControllerSettings>,
    player_q: Single<Entity, With<OurPlayer>>,
    gui_state: Res<GuiState>,
    overlay_state: Res<State<OverlayState>>,
    mut writer: MessageWriter<PlayerInput>,
) {
    let Ok(mut window) = primary_window.single_mut() else {
        return;
    };
    if !window.focused {
        return;
    }

    // Only accept player-look movement in debug mode if right MB held.
    let alt_fire = mouse_button_events.pressed(MouseButton::Right);
    let ignore_mouse = gui_state.is_debug_ui_inspector_visible() && !alt_fire;

    let look_axis = **look.single().unwrap();

    let mut instant_body_turn = Vec3::ZERO;
    let mut instant_head_turn = Vec3::ZERO;

    if !ignore_mouse {
        // Note: swap axes here.  From mouse, "Y" is up/down in userland, "X" is left/right.
        instant_body_turn.y = (if settings.invert_turn_x { 1.0 } else { -1.0 })
            * (settings.turn_scale.x * look_axis.x).to_radians();
        instant_head_turn.y = instant_body_turn.y;

        instant_head_turn.x = (if settings.invert_turn_y { 1.0 } else { -1.0 })
            * (settings.turn_scale.y * look_axis.y).to_radians();

        if settings.center_mouse && !gui_state.show_cursor() && !overlay_state.is_menu() {
            // Keep mouse cursor set window center when while invisible,
            // so look movements will not go outside the window.
            let center = Vec2::new(window.width() / 2.0, window.height() / 2.0);
            window.set_cursor_position(Some(center));
        }
    }

    // let mut tilt = action_state.value(&UserAction::Tilt);
    // // Avoid having touchpad generate this as a side effect of a zoom.
    // if mouse_scroll.delta != Vec2::ZERO && mouse_scroll.delta.y.abs() >= 8.0 {
    //     tilt = 0.0;
    // }
    // let mut tilt = 0.0;
    // instant_head_turn.z = tilt * settings.turn_scale.z;

    // Don't repeat, else it's just a 360 on the slightest lingering touch.
    if turn_around_events.iter().next().unwrap().contains(ActionEvents::START) {
        writer.write(PlayerInput::TurnAround(*player_q));
        return;
    } else if reset_events.iter().next().unwrap().contains(ActionEvents::START) {
        writer.write(PlayerInput::Straighten(*player_q));
        return;
    }

    writer.write(PlayerInput::BodyTurn(
        *player_q,
        PlayerBodyTurn::new(instant_body_turn),
    ));
    writer.write(PlayerInput::HeadTurn(
        *player_q,
        PlayerHeadTurn::new(instant_head_turn),
    ));
}

#[cfg(feature = "input_bei")]
fn collect_player_input(
    // mut commands: Commands,
    // // fire_events: Query<&ActionEvents, (With<Action<Firing>>, With<PlayerAction>)>,
    // player_q: Single<Entity, With<OurPlayer>>,

    mut focused: MessageReader<WindowFocused>,
    mut ignore_mouse: Local<bool>,
) {
    // Avoid hitch when mouse moves after gaining/losing focus.
    if !focused.is_empty() {
        let focused = focused.read().any(|e| e.focused);
        *ignore_mouse = true;
        debug!("focus change: {focused}");
        return;
    }

    if *ignore_mouse {
        debug!("ignoring mouse this frame");
        *ignore_mouse = false;
        return;
    }

    if fire_events.iter().next().unwrap().contains(ActionEvents::START) {
        debug!("press Fire");
        commands.write_message(PlayerInput::StartFire(*player_q));
    }
    if fire_events.iter().next().unwrap().contains(ActionEvents::COMPLETE) {
        debug!("release Fire");
        commands.write_message(PlayerInput::StopFire(*player_q));
    }
}
