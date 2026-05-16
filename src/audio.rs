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

/// Remember to schedule [initialize_audio] or a local copy
/// (can be as early as [Startup])
pub struct AudioCommonPlugin;

impl Plugin for AudioCommonPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(SeedlingPlugins)
            .insert_resource(AudioContextConfig(FirewheelConfig {
                initial_node_capacity: 1024,
                ..default()
            }))

            .configure_loading_state(
                LoadingStateConfig::new(ProgramState::Initializing)
                    .load_collection::<CommonFxAssets>()
            )
            .configure_loading_state(
                LoadingStateConfig::new(ProgramState::LoadingSave)
                    .load_collection::<CommonFxAssets>()
            )

            .add_systems(PreUpdate,
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

/// Pool for UI sound effects (menus, etc), not spatial.
#[derive(PoolLabel, Reflect, PartialEq, Eq, Debug, Hash, Clone)]
#[reflect(Component)]
#[type_path = "game"]
pub struct UiSfx;

/// Pool for the music, not spatial.
#[derive(PoolLabel, Reflect, PartialEq, Eq, Debug, Hash, Clone)]
#[reflect(Component)]
#[type_path = "game"]
pub struct Music;

/// Node label for the music.
#[derive(NodeLabel, Reflect, PartialEq, Eq, Debug, Hash, Clone)]
#[reflect(Component)]
#[type_path = "game"]
pub struct MusicBus;

/// Marker for the background audio, if any.
#[derive(Component, Reflect)]
#[reflect(Component)]
#[type_path = "game"]
struct BackgroundAudio;

/// Default means for initializing the Seedling [PoolLabel]s provided here.
///
/// It is not scheduled by default!
///
/// Either use directly or copy and freely adapt per client.
pub fn initialize_audio(master: Single<Entity, With<MainBus>>, mut commands: Commands) {
    commands.entity(*master).insert(UserVolume {
        volume: Volume::Linear(0.5),
        muted: false,
    });

    const DEFAULT_POOL_VOLUME: Volume = Volume::Linear(0.5);

    // For each new pool, we can provide non-default initial values for the volume.

    // Also: The lower bound of 0 on the pools works around seedling bug #87.

    commands.spawn((
        Name::new("Music"),
        SamplerPool(Music),

        // This ensures a sibling VolumeNode.
        UserVolume {
            volume: DEFAULT_POOL_VOLUME,
            muted: false,
        },

        // This accounts, in theory, for two crossfading songs.
        // Otherwise use the dynamic pool...?
        PoolSize(0 ..= 2),

        MusicBus,

        // Use for e.g. fading *on top of* the [VolumeNode] (fade-out, fade-in) on this node.
        // The [UserVolume] above is for the sound channel volume.
        sample_effects![
            VolumeNode::default(),
        ],
    ))
    ;

    commands.spawn((
        Name::new("SFX"),
        SamplerPool(Sfx),
        UserVolume {
            volume: DEFAULT_POOL_VOLUME,
            muted: false,
        },

        // This pool is for spatial samples.
        PoolSize(0 ..= 256),

        sample_effects![(
            SpatialBasicNode {
                panning_threshold: 0.9,
                ..default()
            },
        )],
    ));

    commands.spawn((
        Name::new("UI"),
        SamplerPool(UiSfx),
        UserVolume {
            volume: DEFAULT_POOL_VOLUME,
            muted: false,
        },
        PoolSize(0 ..= 8),
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
                commands.entity(ent).insert(PlaybackPaused);
            }
        }
    } else /* !pause ==> resume */ {
        for (ent, mut settings, paused) in settings_q.iter_mut() {
            if paused.is_some() {
                settings.play();
                commands.entity(ent).remove::<PlaybackPaused>();
            }
        }
    }
}
