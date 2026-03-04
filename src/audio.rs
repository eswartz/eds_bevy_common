use bevy::prelude::*;
use bevy_seedling::prelude::*;
use bevy_tweening::Lens;

pub struct AudioCommonPlugin;

impl Plugin for AudioCommonPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(SeedlingPlugin::default())
            .add_systems(Startup, initialize_audio)

            .add_systems(PostUpdate,
                (
                    apply_spatial_fixes,
                    apply_volumes,
                )
            )
        ;
    }
}

/// This drives the volume from the user config point of view.
///
/// Our [apply_volumes] system ensures that a corresponding VolumeNode matches
/// the volume and muted state.
#[derive(Component)]
#[require(VolumeNode{ volume: Volume::SILENT, ..default() })]
pub struct UserVolume {
    pub volume: Volume,
    pub muted: bool,
}

/// Pool for in-game diegetic sound effects with spatial listening.
#[derive(PoolLabel, Reflect, PartialEq, Eq, Debug, Hash, Clone)]
#[reflect(Component)]
pub struct Sfx;

/// Pool for UI sound effects (menus, etc), not spatial.
#[derive(PoolLabel, Reflect, PartialEq, Eq, Debug, Hash, Clone)]
#[reflect(Component)]
pub struct UiSfx;

/// Pool for the music, not spatial.
#[derive(PoolLabel, Reflect, PartialEq, Eq, Debug, Hash, Clone)]
#[reflect(Component)]
pub struct Music;

/// Node label for the music.
#[derive(NodeLabel, Reflect, PartialEq, Eq, Debug, Hash, Clone)]
#[reflect(Component)]
pub struct MusicBus;


pub fn initialize_audio(master: Single<Entity, With<MainBus>>, mut commands: Commands) {
    commands.entity(*master).insert(UserVolume {
        volume: Volume::Linear(0.5),
        muted: false,
    });

    const DEFAULT_POOL_VOLUME: Volume = Volume::Linear(1.0);

    // For each new pool, we can provide non-default initial values for the volume.
    commands.spawn((
        Name::new("Music"),
        SamplerPool(Music),
        UserVolume {
            volume: DEFAULT_POOL_VOLUME,
            muted: false,
        },
        PoolSize(2 ..= 4),

        MusicBus,

        // So we can apply fading.
        sample_effects![
            VolumeNode::default(),
        ],

    ))
    ;
    commands.spawn((
        Name::new("SFX"),
        SamplerPool(Sfx),
        sample_effects![(
            SpatialBasicNode {
                panning_threshold: 0.9,
                ..default()
            },
            SpatialScale(Vec3::splat(50.0))
        )],
        UserVolume {
            volume: DEFAULT_POOL_VOLUME,
            muted: false,
        },
        PoolSize(8 ..= 256),
    ));
    commands.spawn((
        Name::new("UI"),
        SamplerPool(UiSfx),
        UserVolume {
            volume: DEFAULT_POOL_VOLUME,
            muted: false,
        },
        PoolSize(2 ..= 8),
    ));
}

/// Apply the correct offset to new SpatialBasicNodes.
///
/// Workaround for https://github.com/CorvusPrudens/bevy_seedling/issues/87
pub fn apply_spatial_fixes(
    listener_q: Query<&Transform, With<SpatialListener3D>>,
    mut spatial_q: Query<(&Transform, &mut SpatialBasicNode), Added<SpatialBasicNode>>,
) {
    // Fetch the spatializer location.
    let Some(spat_xfrm) = listener_q.iter().next() else { return };

    for (xfrm, mut node) in spatial_q.iter_mut() {
        node.offset = (Into::<Vec3>::into(spat_xfrm.translation) - xfrm.translation).into();
    }
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
#[allow(unused)]
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
