use bevy::asset::uuid::Uuid;
use bevy::prelude::*;

use crate::Saveable;

/// This plugin monitors user input and sends PlayerInput events.
pub struct PlayerClientPlugin;

impl Plugin for PlayerClientPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<OurPlayer>()
            .register_type::<OurUser>()
            .add_systems(PreUpdate, sync_player);
    }
}

/// This represents some player.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
#[type_path = "game"]
pub struct Player(pub Uuid);

/// This represents our account.
#[derive(Resource, Reflect, Default)]
#[type_path = "game"]
pub struct OurUser(pub Uuid);

/// This represents our player, attached to a Player(Uuid) matching ours.
#[derive(Component, Reflect, Default)]
#[require(Saveable)]
#[type_path = "game"]
pub struct OurPlayer;

fn sync_player(
    user: Res<OurUser>,
    mut params: ParamSet<(
        Query<Entity, (Without<Player>, With<OurPlayer>)>,
        Query<(Entity, &Player), (Added<Player>, Without<OurPlayer>)>,
    )>,
    mut removed: RemovedComponents<Player>,
    mut commands: Commands,
) {
    for ent in removed.read() {
        if params.p0().contains(ent) {
            log::info!("Despawned our player");
            commands.entity(ent).remove::<OurPlayer>();
        }
    }
    for (ent, player) in params.p1().iter() {
        if player.0 == user.0 {
            commands.entity(ent).insert(OurPlayer);
            log::info!("Claiming entity {ent} for player {}", user.0);
            break;
        }
    }
}
