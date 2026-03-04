use avian3d::prelude::Physics;
use avian3d::prelude::PhysicsTime as _;
use bevy_seedling::prelude::PlaybackSettings;
use bevy::prelude::*;
use bevy_seedling::sample::SamplePlayer;
use bevy_tweening::TweenAnim;

use crate::is_paused;

use super::markers::DespawnAfter;

pub struct LifecyclePlugin;

impl Plugin for LifecyclePlugin {
    fn build(&self, app: &mut App) {
        app
            .register_type::<PauseState>()
            .init_resource::<PauseState>()
            // .add_message::<PostStatusMessage>()
            .add_systems(Update, (
                check_pause_request,
                check_despawners.run_if(not(is_paused))
            ))
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

#[derive(Component)]
pub struct PlaybackPaused;

/// This owns the management of play/pause toggling.
pub fn check_pause_request(
    mut commands: Commands,
    paused: ResMut<PauseState>,
    mut time: ResMut<Time<Physics>>,
    mut settings_q: Query<(Entity, &mut PlaybackSettings, Option<&PlaybackPaused>), With<SamplePlayer>>,
    mut animator_transform_q: Query<&mut TweenAnim>,
    // mut time_runner_q: Query<&mut TimeRunner>,
) {
    if !paused.is_changed() {
        return
    }
    // Get the current, changed value, read as "our action: pause".
    let pause = paused.is_paused();
    // refactor?
    if pause {
        time.pause();
        for (ent, mut settings, _) in settings_q.iter_mut() {
            if *settings.play {
                settings.pause();
                commands.entity(ent).insert(PlaybackPaused);
            }
        }
        for mut animator in animator_transform_q.iter_mut() {
            // By our convention,
            animator.playback_state = bevy_tweening::PlaybackState::Paused;
        }
        // for mut runner in time_runner_q.iter_mut() {
        //     runner.set_paused(true);
        // }
    } else /* !pause ==> resume */ {
        time.unpause();
        for (ent, mut settings, paused) in settings_q.iter_mut() {
            if paused.is_some() {
                settings.play();
                commands.entity(ent).remove::<PlaybackPaused>();
            }
        }
        for mut animator in animator_transform_q.iter_mut() {
            animator.playback_state = bevy_tweening::PlaybackState::Playing;
        }
        // for mut runner in time_runner_q.iter_mut() {
        //     runner.set_paused(false);
        // }
    }
}
