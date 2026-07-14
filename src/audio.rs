use bevy::prelude::*;
use bevy_seedling::context::AudioContextConfig;
use bevy_seedling::prelude::*;
use bevy_asset_loader::prelude::*;
use bevy_tweening::Lens;

use bevy_seedling::prelude::PlaybackSettings;
use bevy_seedling::sample::SamplePlayer;

use crate::CommonFxAssets;
use crate::PauseState;
use crate::ProgramState;
use crate::TimeStretchNode as TimeStretchNode;

/// Remember to schedule [initialize_audio] or a local copy
/// (can be as early as [Startup])
pub struct AudioCommonPlugin;

impl Plugin for AudioCommonPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(SeedlingPlugins)
            .register_node::<TimeStretchNode>()

            .insert_resource(AudioContextConfig(FirewheelConfig {
                ..default()
            }))

            .configure_loading_state(
                LoadingStateConfig::new(ProgramState::Initializing)
                    .load_collection::<CommonFxAssets>()
            )

            .add_systems(FixedPreUpdate,
                (
                    check_pause_request,
                )
            )
            .add_systems(PostUpdate,
                (
                    apply_volumes,
                )
            )
        ;
    }
}

/// This drives the volume from the user config point of view.
///
/// Our [apply_volumes] system manages a corresponding [VolumeNode] that
/// tracks the `volume` and `muted` state.
#[derive(Component, Reflect)]
#[require(VolumeNode{ volume: Volume::SILENT, ..default() })]
#[reflect(Component)]
#[type_path = "game"]
pub struct UserVolume {
    pub volume: Volume,
    pub muted: bool,
}

/// Pool for in-game diegetic sound effects with spatial listening.
#[derive(PoolLabel, Reflect, PartialEq, Eq, Debug, Hash, Clone)]
#[reflect(Component)]
#[type_path = "game"]
pub struct Sfx;

/// Node label for the in-game sound effects.
#[derive(NodeLabel, Reflect, PartialEq, Eq, Debug, Hash, Clone)]
#[reflect(Component)]
#[type_path = "game"]
pub struct SfxNode;

/// Node label for the FreeverbNode for the Sfx bus.
#[derive(Component, Reflect, Debug, Default)]
#[reflect(Component)]
#[type_path = "game"]
pub struct SfxReverbNode;

/// Pool for UI sound effects (menus, etc), not spatial.
#[derive(PoolLabel, Reflect, PartialEq, Eq, Debug, Hash, Clone)]
#[reflect(Component)]
#[type_path = "game"]
pub struct UiSfx;

/// Node label for the UI sound effects.
#[derive(NodeLabel, Reflect, PartialEq, Eq, Debug, Hash, Clone)]
#[reflect(Component)]
#[type_path = "game"]
pub struct UiSfxNode;

/// Pool for the music, not spatial.
#[derive(PoolLabel, Reflect, PartialEq, Eq, Debug, Hash, Clone)]
#[reflect(Component)]
#[type_path = "game"]
pub struct Music;

/// Node label for the music.
#[derive(NodeLabel, Reflect, PartialEq, Eq, Debug, Hash, Clone)]
#[reflect(Component)]
#[type_path = "game"]
pub struct MusicNode;

/// Marker for the background audio, if any.
#[derive(Component, Reflect)]
#[reflect(Component)]
#[type_path = "game"]
struct BackgroundAudio;

pub const DEFAULT_POOL_VOLUME: Volume = Volume::Linear(0.75);

/// Default means for initializing the Seedling [PoolLabel]s provided here.
///
/// It is not scheduled by default!
///
/// Either use directly or copy and freely adapt per client.
pub fn initialize_audio(master: Single<Entity, With<MainBus>>, mut commands: Commands) {
    commands.entity(*master).try_insert(UserVolume {
        volume: Volume::Linear(0.5),
        muted: false,
    });


    // For each new pool, we can provide non-default initial values for the volume.

    commands.spawn((
        MusicNode,

        // This ensures a sibling VolumeNode.
        UserVolume {
            volume: DEFAULT_POOL_VOLUME,
            muted: false,
        },
    ))
    ;

    commands.spawn((
        Name::new("Music"),
        SamplerPool(Music),

        // This accounts, in theory, for two crossfading songs.
        // Otherwise use the dynamic pool...?
        PoolSize(1 ..= 2),

        // Use for e.g. fading *on top of* the [VolumeNode] (fade-out, fade-in) on this node.
        // The [UserVolume] above is the base sound channel volume.
        sample_effects![
            VolumeNode::default(),
        ],
    ))
    .connect(MusicNode)
    ;

    let sfx_bus = commands.spawn((
        // Marker for the bus.
        SfxNode,

        UserVolume {
            volume: DEFAULT_POOL_VOLUME,
            muted: false,
        },
    ))
    .id();

    let send = commands.spawn((
        SfxReverbNode,

        FreeverbNode {
            room_size: 0.25,
            width: 0.5,
            damping: 0.5,
            ..default()
        },
    ))
    .connect(sfx_bus)
    .head();

    // All the desired effects must be placed in the pool.
    // Then a given SamplePlayer targeting this pool can override the effects.
    commands
        .spawn((
            SamplerPool(Sfx),

            PoolSize(0 ..= 256),

            sample_effects![
                // Order matters! Put SpatialBasicNode first so effects happen after
                // the sound is localized.
                SpatialBasicNode::default(),
                FastLowpassNode::<2>::from_cutoff_hz(48000.0),
                // TimeStretchNode { stretch_factor: 1.0 },
                // SvfNode::<2>::from_highpass(20.0, 0.5), // effectively disabled
                SendNode::new(Volume::Linear(0.0), send), // effectively disabled
            ],
        ))
        .connect(SfxNode);

    commands.spawn((
        Name::new("UI"),
        SamplerPool(UiSfx),
        UserVolume {
            volume: DEFAULT_POOL_VOLUME,
            muted: false,
        },
        PoolSize(0 ..= 8),

        // Marker for the node for UI effects.
        UiSfxNode,

    ));
}


/// Apply mute-able UserVolume to VolumeNodes.
pub fn apply_volumes(
    mut vol_q: Query<(Entity, &UserVolume, &mut VolumeNode), Changed<UserVolume>>,
) {
    for (_ent, user, mut vol) in vol_q.iter_mut() {
        vol.volume = if user.muted { Volume::SILENT } else { user.volume };
    }
}


/// Fixme, VolumeNode/VolumeFade should work...
#[derive(Debug)]
pub struct VolumeNodeLens {
    pub start: VolumeNode,
    pub end: VolumeNode,
}

impl Lens<VolumeNode> for VolumeNodeLens {
    fn lerp(&mut self, mut target: Mut<VolumeNode>, ratio: f32) {
        let new_linear = self.start.volume.linear().lerp(self.end.volume.linear(), ratio);
        target.set_linear(new_linear);
    }
}

#[derive(Component, Reflect)]
#[reflect(Component)]
#[type_path = "game"]
struct PlaybackPaused;

/// This owns the management of play/pause toggling.
fn check_pause_request(
    mut commands: Commands,
    paused: ResMut<PauseState>,
    mut settings_q: Query<(Entity, &mut PlaybackSettings, Option<&PlaybackPaused>), With<SamplePlayer>>,
) {
    if !paused.is_changed() {
        return
    }
    let pause = paused.is_paused();
    if pause {
        for (ent, mut settings, _) in settings_q.iter_mut() {
            if *settings.play {
                settings.pause();
                commands.entity(ent).try_insert(PlaybackPaused);
            }
        }
    } else /* !pause ==> resume */ {
        for (ent, mut settings, paused) in settings_q.iter_mut() {
            if paused.is_some() {
                settings.play();
                commands.entity(ent).try_remove::<PlaybackPaused>();
            }
        }
    }
}
