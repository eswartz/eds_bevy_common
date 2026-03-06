use crate::*;

use avian3d::prelude::{CollidingEntities, PhysicsSystems};
use bevy::prelude::*;

use bevy_tweening::lens::TransformPositionLens;
use bevy_tweening::{EaseMethod, Tween, TweenAnim};

use std::time::Duration;

/// Add this to process collisions with any [DeathboxCollider].
/// It will move [Player] entities back to some [PlayerStart]
/// and despawn [Spawned] items that hit it.
#[derive(Default)]
pub struct DeathboxPlugin {
    flags: DeathboxFlags,
}

#[derive(Resource, Debug, Reflect, Default, Clone)]
#[reflect(Resource, Default)]
pub struct DeathboxFlags {
    move_players: bool,
    remove_spawns: bool,
}

impl DeathboxPlugin {
    pub fn with_move_player_to_start(self, flag: bool) -> Self {
        DeathboxPlugin {
            flags: DeathboxFlags {
                move_players: flag,
                .. self.flags
            },
            .. self
        }
    }
    pub fn with_despawn_items(self, flag: bool) -> Self {
        DeathboxPlugin {
            flags: DeathboxFlags {
                remove_spawns: flag,
                .. self.flags
            },
            .. self
        }
    }
}

impl Plugin for DeathboxPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_message::<HitDeathboxMessage>()
            .insert_resource(self.flags.clone())
            .add_systems(
                FixedUpdate,
                (
                    check_out_of_bounds,
                )
                .before(TransformSystems::Propagate)
                .after(PhysicsSystems::Writeback)
                .run_if(not(is_user_paused))
                .run_if(in_state(LevelState::Playing))
                .run_if(in_state(ProgramState::InGame)),
            )
            .add_systems(
                Update,
                (
                    handle_out_of_bounds,
                )
                .run_if(not(is_user_paused))
                .run_if(in_state(LevelState::Playing))
                .run_if(in_state(ProgramState::InGame)),
            )
        ;
    }
}

/// This message is sent when a given [Spawned] or [Player] has hit the deathbox.
#[derive(Message)]
pub enum HitDeathboxMessage {
    Player(Entity),
    Spawned(Entity),
}

fn check_out_of_bounds(
    parent_q: Query<&ChildOf>,
    sensor_q: Query<&CollidingEntities, With<DeathboxCollider>>,
    player_q: Query<&Player>,
    spawned_q: Query<&Spawned, Without<DespawnAfter>>,
    mut writer: MessageWriter<HitDeathboxMessage>,
) {
    for coll in sensor_q.iter() {
        for ent in coll.iter() {
            if player_q.contains(*ent) {
                writer.write(HitDeathboxMessage::Player(*ent));
                continue;
            }
            if spawned_q.contains(*ent) {
                writer.write(HitDeathboxMessage::Spawned(*ent));
                continue;
            }

            for parent in parent_q.iter_ancestors(*ent) {
                if player_q.contains(parent) {
                    writer.write(HitDeathboxMessage::Player(parent));
                    break;
                }
                if spawned_q.contains(parent) {
                    writer.write(HitDeathboxMessage::Spawned(parent));
                    break;
                }
            }
        }
    }
}

fn handle_out_of_bounds(
    flags: Res<DeathboxFlags>,
    mut commands: Commands,
    mut reader: MessageReader<HitDeathboxMessage>,
    player_q: Query<&Transform, With<Player>>,
    player_start_q: Query<&Transform, With<PlayerStart>>,
) {
    for hit in reader.read() {
        match hit {
            HitDeathboxMessage::Player(entity) => {
                if flags.move_players
                && let Ok(xfrm) = player_q.get(*entity)
                && let Some(dest_xfrm) = player_start_q.iter().next() {
                    let xfrm_tween = Tween::new(
                        EaseMethod::EaseFunction(EaseFunction::BackOut),
                        Duration::from_secs_f32(0.5),
                        TransformPositionLens {
                            start: xfrm.translation,
                            end: dest_xfrm.translation,
                        }
                    );
                    commands.entity(*entity).try_insert((
                        TweenAnim::new(xfrm_tween).with_destroy_on_completed(true),
                    ));
                }
            }
            HitDeathboxMessage::Spawned(entity) => {
                if flags.remove_spawns {
                    commands.entity(*entity).try_insert(DespawnAfter(Duration::from_secs(1)));
                }
            }
        }
    }
}
