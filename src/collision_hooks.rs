use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use avian3d::prelude::*;

use crate::*;

/// Define a `SystemParam` for modifying collision hooks.
#[derive(SystemParam)]
pub struct GeometryCollisionHooks<'w, 's> {
    player_q: Query<'w, 's, &'static PlayerMovement, With<Player>>,
    projectile_q: Query<'w, 's, (), With<Projectile>>,
    parent_q: Query<'w, 's, &'static ChildOf>,
    cb_opt: Option<Res<'w, GeometryCollisionHooksCallbacks>>,
}

/// Register this to tell if a given entity (collider) is a liquid or not.
/// The collision hook queries this to ignore cases where the player
/// is above but not sufficiently inside the area.
/// (This is most useful with [player_spawning::add_fps_foot_gameplay_collider].)
#[derive(Resource)]
pub struct GeometryCollisionHooksCallbacks {
    pub is_liquid: Box<dyn Fn(Entity) -> bool + Sync + Send>,
}

///

impl GeometryCollisionHooks<'_, '_> {
    fn is_player(&self, body: Option<Entity>) -> bool {
        if let Some(body) = body {
            if self.player_q.contains(body) {
                // I.e. for the player full body
                true
            } else if let Ok(parent) = self.parent_q.get(body) {
                // i.e. for the GamePlay layer subcollider of the player
                self.player_q.contains(parent.0)
            } else {
                false
            }
        } else {
            false
        }
    }

    fn is_projectile(&self, body_opt: Option<Entity>, body: Entity) -> bool {
        self.projectile_q.contains(
            if let Some(the_body) = body_opt {
                the_body
            } else {
                body
            }
        )
    }
}

impl CollisionHooks for GeometryCollisionHooks<'_, '_> {
    fn modify_contacts(&self, contacts: &mut ContactPair, _commands: &mut Commands) -> bool {
        let (is_flipped, _proj, world, deepest) = {
            // Ignore if not touching.
            let Some(deepest) = contacts.find_deepest_contact() else {
                return false;
            };

            // See what things are colliding.
            let ((player1, proj1), (player2, proj2)) = (
                (self.is_player(contacts.body1), self.is_projectile(contacts.body1, contacts.collider1)),
                (self.is_player(contacts.body2), self.is_projectile(contacts.body2, contacts.collider2)),
            );
            if player1 && player2 {
                // Player V. Player, ignore
                return true
            }
            if !player1 && !proj1 && !player2 && !proj2 {
                // Shouldn't be here unless we added an ActiveCollisionHooks and forgot to handle it
                warn!("not handling hook for {:?} and {:?}", contacts.body1, contacts.body2);
                return true
            }

            // Make player or projectile first.
            let body1 = contacts.body1.unwrap_or(contacts.collider1);
            let body2 = contacts.body2.unwrap_or(contacts.collider2);
            if player1 || proj1 {
                (false, body1, body2, *deepest)
            } else {
                (true, body2, body1, deepest.flipped())
            }
        };

        // Enter areas when we're for sure _in_ them, not grazing.
        if deepest.penetration < 0.125
            && let Some(cb) = &self.cb_opt
            && (cb.is_liquid)(world) {
                debug!("ignoring grazing with liquid {world} @ {deepest:?}");
                return false;
            }

        // Ignore cases where we hit an embedded and invisible collider face in the ground.
        if deepest.penetration < 0.1 /* && deepest.local_point2.y < -0.05 */ && deepest.normal_impulse.abs() < 0.01 {
            // let mut any_edges = false;
            for man in &contacts.manifolds {
                for pt in &man.points {
                    if if is_flipped {
                        pt.feature_id2.is_edge()
                    } else {
                        pt.feature_id1.is_edge()
                    } {
                        return false
                    }
                }
            }
        }

        true
    }
}
