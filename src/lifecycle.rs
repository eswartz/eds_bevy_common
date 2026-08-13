use std::time::Duration;

use bevy::prelude::*;
use bevy::winit::UpdateMode;
use bevy::winit::WinitSettings;
use bevy_tweening::TweenAnim;

use crate::physics::*;
use crate::prelude::*;

use super::markers::DespawnAfter;

pub struct LifecyclePlugin;

impl Plugin for LifecyclePlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<PauseState>()
            .init_resource::<NonPauseTime>()
            .add_systems(Startup,
                init_phased_winit_settings,
            )
            .add_systems(Update, (
                check_pause_request,
                reset_pause_on_enter_launch_menu
                    .run_if(resource_changed::<State<ProgramState>>),
                check_despawners
                    .run_if(not(is_paused)),
                check_configure_before_playing,
                update_frame_rate_on_pause
                    .run_if(resource_changed::<PauseState>),
                count_nonpaused_time
                    .run_if(not(is_paused)),
            ))

            .add_systems(
                OnEnter(ProgramState::InGame),
                |mut time: If<ResMut<Time<Physics>>>| {
                    time.unpause();
                }
            )
            .add_systems(
                OnEnter(GameplayState::Setup),
                |mut time: If<ResMut<Time<Physics>>>| {
                    time.pause();
                }
            )
        ;
    }
}

fn check_despawners(
    mut commands: Commands,
    mut despawn_q: Query<(Entity, &mut DespawnAfter)>,
    time: Res<Time>,
) {
    let dt = time.delta();
    for (ent, mut despawn) in despawn_q.iter_mut() {
        if despawn.0.is_zero() {
            // Ignore these as in a default component.
            continue
        }
        if despawn.0 <= dt {
            commands.entity(ent).try_despawn();
        } else {
            despawn.0 = despawn.0.saturating_sub(dt);
        }
    }
}

/// This resource reflects and drives the state of Pause across the process.
#[derive(Resource, Debug, Clone, Reflect, Default)]
#[reflect(Resource)]
#[type_path = "game"]
pub struct PauseState {
    /// User state (e.g. from pressing Pause key)
    user: bool,
    /// Menu state (a menu is up)
    menu: bool,
}

impl PauseState {
    pub fn new(user: bool) -> Self {
        Self{ user, menu: false }
    }

    pub fn is_paused(&self) -> bool { self.user | self.menu }
    pub fn is_user_paused(&self) -> bool { self.user }
    pub fn is_menu_paused(&self) -> bool { self.menu }

    pub fn set_user_paused(&mut self, paused: bool) {
        self.user = paused
    }
    pub fn set_menu_paused(&mut self, paused: bool) {
        self.menu = paused
    }
}

/// This processes PauseState changes as the source of truth for
/// pausing-related components that come from types we can't extend
/// to apply their own logic based on `resource_changed::<PauseState>`.
///
fn check_pause_request(
    paused: Res<PauseState>,
    time: Option<ResMut<Time<Physics>>>,
    mut animator_transform_q: Query<&mut TweenAnim>,
) {
    if !paused.is_changed() {
        return
    }
    // Get the current, changed value, read as "our action: pause".
    let pause = paused.is_paused();
    // refactor?
    if pause {
        time.map(|mut time| { time.pause(); });
        for mut animator in animator_transform_q.iter_mut() {
            // By our convention,
            animator.playback_state = bevy_tweening::PlaybackState::Paused;
        }
        // for mut runner in time_runner_q.iter_mut() {
        //     runner.set_paused(true);
        // }
    } else /* !pause ==> resume */ {
        time.map(|mut time| { time.unpause(); });
        for mut animator in animator_transform_q.iter_mut() {
            animator.playback_state = bevy_tweening::PlaybackState::Playing;
        }
        // for mut runner in time_runner_q.iter_mut() {
        //     runner.set_paused(false);
        // }
    }
}

/// This stores the winit settings to apply when the main program is running vs. paused.
/// These are orthogonal from the focused/unfocused state.
#[derive(Resource)]
pub struct PhasedWinitSettings{
    pub running: WinitSettings,
    pub paused: WinitSettings,
}

fn init_phased_winit_settings(
    mut commands: Commands,
    winit_settings: Res<WinitSettings>,
    phased_settings: Option<Res<PhasedWinitSettings>>,
) {
    if phased_settings.is_some() { return };

    commands.insert_resource(PhasedWinitSettings{
        running: winit_settings.clone(),
        paused: WinitSettings {
            focused_mode: UpdateMode::reactive_low_power(
                Duration::from_secs_f32(1.0 / 1.0)
            ),
            unfocused_mode: winit_settings.unfocused_mode,
        },
    });
}

// App-specific handling on top of ebc systems.
// Reduce the frame rate when paused.
fn update_frame_rate_on_pause(
    paused: ResMut<PauseState>,
    phased_settings: If<Res<PhasedWinitSettings>>,
    mut winit_settings: ResMut<WinitSettings>,
) {
    let pause = paused.is_menu_paused();
    *winit_settings = if !pause {
        phased_settings.running.clone()
    } else {
        phased_settings.paused.clone()
    };
}

/// If we see a big state change, clear the pause state.
fn reset_pause_on_enter_launch_menu(
    program_state: Res<State<ProgramState>>,
    mut pause_state: ResMut<PauseState>,
) {
    if **program_state != ProgramState::LaunchMenu {
        // Nope
        return
    }

    pause_state.set_menu_paused(false);
    pause_state.set_user_paused(false);
}


fn check_configure_before_playing(
    mut commands: Commands,
    state: Res<State<LevelState>>,
    configure_q: Query<Entity, With<ConfigureBeforePlaying>>,
    mut frames: Local<u8>,
) {
    // Monitor things during this state.
    if *state.get() == LevelState::Configuring {
        // We expect this to go to zero after a few frames.
        let ents: Vec<_> = configure_q.iter().collect();
        if ents.is_empty() {
            *frames = 0;
            commands.set_state(LevelState::Playing);
            return;
        }

        // Wait for a given number of frames.
        if *frames >= 15 {
            error!("Removing stuck ConfigureBeforePlaying on: {ents:?}");
            // Remove them all.
            for ent in ents {
                commands.entity(ent).try_remove::<ConfigureBeforePlaying>();
            }
            // Let the next frame handle their removal and re-querying,
            // or not, in case something is e.g. adding this component
            // every frame.
            *frames = 0;
        } else {
            *frames += 1;
        }
    } else if state.is_changed() {
        // Reset whenever we are (now) in some other LevelState.
        *frames = 0;
    }
}

#[derive(Resource, Default, Clone, Reflect, PartialEq)]
#[reflect(Resource, Default)]
#[type_path = "game"]
pub struct NonPauseTime(pub Duration);

fn count_nonpaused_time(
    time: Res<Time>,
    pause_state: Res<PauseState>,
    mut pause_time: ResMut<NonPauseTime>,
) {
    if !pause_state.is_paused() {
        pause_time.0 += time.delta();
    } else {
        pause_time.set_if_neq(NonPauseTime(Duration::ZERO));
    }
}
