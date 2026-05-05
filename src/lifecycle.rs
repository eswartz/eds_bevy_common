use avian3d::prelude::Physics;
use avian3d::prelude::PhysicsTime as _;
use bevy::ecs::entity_disabling::Disabled;
use bevy::prelude::*;
use bevy::state::state::StateTransitionSystems;
use bevy_tweening::TweenAnim;

use crate::*;

use super::markers::DespawnAfter;

pub struct LifecyclePlugin;

impl Plugin for LifecyclePlugin {
    fn build(&self, app: &mut App) {
        app
            .register_type::<PauseState>()
            .init_resource::<PauseState>()
            .add_systems(Update, (
                check_pause_request,
                reset_pause_on_enter_launch_menu,
                check_despawners.run_if(not(is_paused)),
                check_configure_before_loading,
            ))
            .add_systems(
                StateTransition,
                (
                    despawn_entities_on_state_change::<ProgramState>.in_set(StateTransitionSystems::EnterSchedules),
                    despawn_entities_on_state_change::<LevelState>.in_set(StateTransitionSystems::EnterSchedules),
                    despawn_entities_on_state_change::<GameplayState>.in_set(StateTransitionSystems::EnterSchedules),
                    despawn_entities_on_state_change::<OverlayState>.in_set(StateTransitionSystems::EnterSchedules),
                )
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

/// Despawns entities marked with [`DespawnOnExitOrReenter<S>`] when their state no
/// longer matches the world state.
///
/// If the entity has already been despawned no warning will be emitted.
fn despawn_entities_on_state_change<S: States>(
    mut commands: Commands,
    mut transitions: MessageReader<StateTransitionEvent<S>>,
    query: Query<(Entity, &DespawnOnExitOrReenter<S>), Allow<Disabled>>,
) {
    for transition in transitions.read() {
        let Some(entered) = &transition.entered else {
            continue;
        };
        for (entity, binding) in &query {
            if binding.0 == *entered {
                commands.entity(entity).try_despawn();
            }
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
    if !program_state.is_changed() {
        // Nope
        return
    }
    if !matches!(program_state.get(), ProgramState::LaunchMenu) {
        // Nope
        return
    }

    pause_state.set_menu_paused(false);
    pause_state.set_user_paused(false);
}

fn check_configure_before_loading(
    mut commands: Commands,
    state: Res<State<LevelState>>,
    configure_q: Query<&ConfigureBeforePlaying>,
) {
    if *state.get() == LevelState::Configuring {
        if configure_q.count() == 0 {
            commands.set_state(LevelState::Playing);
        }
    }
}
