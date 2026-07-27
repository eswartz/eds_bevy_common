//! This is a demo program showing the integration of various "ebc" features,
//! focusing on custom Firewheel nodes.
use std::time::Duration;

use bevy::prelude::*;
use bevy_seedling::prelude::*;
use eds_bevy_common::prelude::{
    AppPlugin, AudioCommonPlugin, CommonAssetsPlugin, CommonFxAssets, DEFAULT_POOL_VOLUME,
    DebugPlugin, GameplayState, GuiState, LifecyclePlugin, ProgramState, StatsOverlayPlugin,
    TimeStretchNode, UserVolume, dev_tools_enabled,
};

fn main() {
    let mut app = App::new();
    app
        // Needs to precede DefaultPlugins.AssetPlugin
        .add_plugins(CommonAssetsPlugin)
        .add_plugins((DefaultPlugins,))
        .add_plugins((
            AppPlugin, // for asset loading
            LifecyclePlugin,
            AudioCommonPlugin,
            // GuiPlugin,
        ))
        .add_systems(Startup, startup)
        .add_systems(OnEnter(ProgramState::LaunchMenu), start_loop)
        .add_systems(Update, spawner.run_if(in_state(ProgramState::InGame)));

    if dev_tools_enabled() {
        app.add_plugins(DebugPlugin)
            .add_systems(
                First,
                (
                    bevy::dev_tools::states::log_transitions::<ProgramState>,
                    bevy::dev_tools::states::log_transitions::<GameplayState>,
                ),
            )
            .add_plugins(StatsOverlayPlugin);
    }

    app.run();
}

fn start_loop(mut commands: Commands, gui_state: Option<ResMut<GuiState>>) {
    commands.set_state(ProgramState::InGame);
    commands.set_state(GameplayState::Playing);
    if let Some(mut gui_state) = gui_state {
        gui_state.enabled = true;
        gui_state.show_inspector = true;
        gui_state.show_inspector_always = true;
    }
}

#[derive(PoolLabel, Reflect, PartialEq, Eq, Debug, Hash, Clone)]
#[reflect(Component)]
pub(crate) struct SfxPool;

#[derive(NodeLabel, PartialEq, Eq, Debug, Hash, Clone)]
pub(crate) struct SfxBus;

fn startup(mut commands: Commands) {
    commands.spawn((
        UserVolume {
            volume: DEFAULT_POOL_VOLUME,
            muted: false,
        },
        // Marker for the bus.
        SfxBus,
    ));

    // Let's create a new sample player pool and route it to our effects bus.
    //
    // All the desired effects must be placed in the pool.
    // Then a given SamplePlayer targeting this pool can override the effects.
    commands
        .spawn((
            SamplerPool(SfxPool),
            PoolSize(256..=1024),
            sample_effects![
                // Order matters! Put SpatialBasicNode first so effects happen after
                // the sound is localized.
                SpatialBasicNode::default(),
                TimeStretchNode {
                    stretch_factor: 1.0
                },
                SvfNode::<2>::from_allpass(f32::MAX, 0.0), // effectively disabled
            ],
        ))
        // Send everything in the pool into the effects bus (with the master VolumeNode).
        .connect(SfxBus);
}

const SPAWN_DELAY_SECS: f32 = 0.5;

fn spawner(
    mut commands: Commands,
    // server: Res<AssetServer>,
    time: Res<Time>,
    mut timer: Local<Timer>,
    fx: Res<CommonFxAssets>,
) {
    if timer.duration().is_zero() {
        // Initialize.
        *timer = Timer::from_seconds(SPAWN_DELAY_SECS, TimerMode::Repeating);
    }

    let dt = time.delta();
    if !timer.tick(dt).just_finished() {
        return;
    }

    let offset = Vec3::new(time.elapsed_secs() % 2.0 - 1.0, 0.0, 0.0);
    // let offset = Vec3::ZERO;

    use std::f32::consts as f32c;
    let stretch_factor: f32 = ((time.elapsed_secs() * f32c::FRAC_PI_2 / 7.0).cos() * 2.0 + 1.5).max(0.01001);
    info!("stretch: {stretch_factor:.3}");
    commands.spawn((
        SfxPool,
        SamplePlayer::new(fx.action.clone()),
        // SamplePlayer::new(fx.bump3.clone()),
        Transform::from_translation(offset),
        sample_effects![
            // SpatialBasicNode {
            //     // prepopulate to avoid click at start
            //     offset: offset.into(),
            //     ..default()
            // },
            TimeStretchNode {
                stretch_factor,
            },
        ],
    ));

    timer.reset();
    timer.set_duration(Duration::from_secs(1));
}
