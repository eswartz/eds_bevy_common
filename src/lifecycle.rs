use avian3d::prelude::Physics;
use avian3d::prelude::PhysicsTime as _;
use bevy::prelude::*;
use bevy_tweening::TweenAnim;

use crate::*;

use super::markers::DespawnAfter;

pub struct LifecyclePlugin;

impl Plugin for LifecyclePlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<PauseState>()
            .add_systems(Update, (
                check_pause_request,
                reset_pause_on_enter_launch_menu
                    .run_if(resource_changed::<State<ProgramState>>),
                check_despawners.run_if(not(is_paused)),
                check_configure_before_playing,
            ))

            .add_systems(
                OnEnter(ProgramState::InGame),
                |mut time: ResMut<Time<Physics>>| {
                    info!("resume");
                    time.unpause();
                }
            )
            .add_systems(
                OnEnter(GameplayState::Setup),
                |mut time: ResMut<Time<Physics>>| {
                    info!("pause");
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
    mut time: ResMut<Time<Physics>>,
    mut animator_transform_q: Query<&mut TweenAnim>,
) {
    if !paused.is_changed() {
        return
    }
    // Get the current, changed value, read as "our action: pause".
    let pause = paused.is_paused();
    // refactor?
    if pause {
        time.pause();
        for mut animator in animator_transform_q.iter_mut() {
            // By our convention,
            animator.playback_state = bevy_tweening::PlaybackState::Paused;
        }
        // for mut runner in time_runner_q.iter_mut() {
        //     runner.set_paused(true);
        // }
    } else /* !pause ==> resume */ {
        time.unpause();
        for mut animator in animator_transform_q.iter_mut() {
            animator.playback_state = bevy_tweening::PlaybackState::Playing;
        }
        // for mut runner in time_runner_q.iter_mut() {
        //     runner.set_paused(false);
        // }
    }
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
        if *frames >= 60 {
            error!("Removing stuck ConfigureBeforePlaying on: {ents:?}");
            // Remove them all.
            for ent in ents {
                commands.entity(ent).remove::<ConfigureBeforePlaying>();
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
