//! Support user-driven grabbing and movement of items in the world.
//!
//! Items with the [crate::Selected] component can be grabbed.

#[cfg(feature = "input_bei")]
use bevy::color::palettes::tailwind;
use bevy_mod_outline::InheritOutline;
use bevy_mod_outline::OutlineStencil;

use avian3d::prelude::*;
use bevy::prelude::*;

use bevy_mod_outline::{OutlineMode, OutlinePlugin, OutlineVolume};

#[cfg(feature = "input_lim")]
use leafwing_input_manager::prelude::*;
#[cfg(feature = "input_bei")]
use bevy_enhanced_input::prelude::*;

#[cfg(feature = "input_bei")]
use crate::Grabbed;
#[cfg(feature = "input_bei")]
use crate::Highlighted;
#[cfg(feature = "highlighting")]
use crate::HighlightingMode;
#[cfg(feature = "input_bei")]
use crate::ProgramState;
#[cfg(feature = "input_bei")]
use crate::WorldCamera;
#[cfg(feature = "input_bei")]
use crate::actions;
#[cfg(feature = "input_bei")]
use crate::debug_gui_wants_direct_input;
#[cfg(feature = "input_bei")]
use crate::is_in_menu;
#[cfg(feature = "input_bei")]
use crate::is_level_active;
#[cfg(feature = "input_bei")]
use crate::is_paused;

pub struct GrabbingPlugin;

impl Plugin for GrabbingPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<OutlinePlugin>() {
            app.add_plugins(OutlinePlugin);
        }
        app
            .init_resource::<GrabbingForce>()
            .init_resource::<HighlightedIsGrabbable>()
            .init_resource::<GrabbedItemStyle>()
            .add_message::<GrabbingCommand>()
            .add_systems(
            FixedUpdate,
            (
                sync_grabbable_with_highlighted
                    .run_if(highlighted_is_grabbable)
                    .run_if(not(is_grabbing_item))
                    ,
                process_grab_changes,
                move_grabbed_item.run_if(is_grabbing_item),
            )
            .chain()
            .run_if(not(is_in_menu))
            .run_if(is_level_active)
            .run_if(not(is_paused))
            .run_if(not(debug_gui_wants_direct_input))
            .run_if(in_state(ProgramState::InGame))
        );

        if cfg!(feature = "input_bei") {
        // app.add_systems(
        //     FixedUpdate,
        //     check_grab_actions
        //     .run_if(not(is_in_menu))
        //     .run_if(is_level_active)
        //     .run_if(not(is_paused))
        //     .run_if(not(debug_gui_wants_direct_input))
        //     .run_if(in_state(ProgramState::InGame))
        // )
            app
                .add_observer(on_start_grab)
                .add_observer(on_change_grab_distance)
                .add_observer(on_end_grab_drop)
                .add_observer(on_end_grab_fire)
                ;
        }
    }
}

/// This resource defines the default style for highlighted items.
/// The given components are added (and removed) as needed.
#[derive(Resource, Reflect)]
#[reflect(Resource)]
#[type_path = "game"]
pub struct GrabbedItemStyle {
    pub volume: OutlineVolume,
    pub mode: OutlineMode,
    pub stencil: Option<OutlineStencil>,
    pub inherit: Option<InheritOutline>,
}

impl Default for GrabbedItemStyle {
    fn default() -> Self {
        Self {
            volume: OutlineVolume {
                visible: true,
                width: 4.0,
                colour: tailwind::LIME_500.with_alpha(0.75).into(),
            },
            mode: OutlineMode::FloodFlat,
            stencil: None,
            inherit: None,
        }
    }
}

impl GrabbedItemStyle {
    pub fn apply_to<'a>(&self, mut ent_commands: EntityCommands<'a>) {
        ent_commands.insert(self.volume.clone());
        ent_commands.insert(self.mode.clone());
        if let Some(stencil) = &self.stencil {
            ent_commands.insert(stencil.clone());
        }
        if let Some(inherit) = &self.inherit {
            ent_commands.insert(inherit.clone());
        }
    }
    pub fn remove_from<'a>(&self, mut ent_commands: EntityCommands<'a>) {
        ent_commands.try_remove::<(OutlineVolume, OutlineMode)>();
        if self.stencil.is_some() {
            ent_commands.try_remove::<OutlineStencil>();
        }
        if self.inherit.is_some() {
            ent_commands.try_remove::<InheritOutline>();
        }
    }
}

#[derive(Message, Clone, Reflect, Debug)]
#[reflect(Clone)]
pub enum GrabbingCommand {
    GrabItem(Entity),
    ReleaseItems,
    CancelGrabItems,
}

/// When set to `true`, mirror the [crate::Highlighted] state with [Grabbable].
/// Otherwise, it's up to you.
#[derive(Resource, Reflect, Debug, Clone)]
#[reflect(Resource, Clone)]
#[type_path = "game"]
pub struct HighlightedIsGrabbable(pub bool);

impl Default for HighlightedIsGrabbable {
    fn default() -> Self {
        Self(true)
    }
}

pub fn highlighted_is_grabbable(res: Res<HighlightedIsGrabbable>) -> bool {
    res.0
}

fn sync_grabbable_with_highlighted(
    mut commands: Commands,

    hilit_q: Query<Entity, With<Highlighted>>,
    unhilit_q: Query<Entity, (With<Grabbable>, Without<Highlighted>)>,
    grab_q: Query<Entity, With<Grabbable>>,
) {
    // Turn off formerly highlighted items, which cannot be grabbable now.
    for ent in unhilit_q.iter() {
        if grab_q.contains(ent) {
            commands.entity(ent).remove::<Grabbable>();
        }
    }
    // Add newly highlighted items.
    for ent in hilit_q.iter() {
        if !grab_q.contains(ent) {
            commands.entity(ent).insert(Grabbable);
        }
    }
}

/// Mark the entity as being "grabbable".
#[derive(Component, Reflect, Debug)]
#[component(storage = "SparseSet")]
#[reflect(Component)]
#[type_path = "game"]
pub struct Grabbable;

/// Currently grabbed thing and its transform
/// (Resource only defined if so).
#[derive(Resource, Reflect, Debug, Clone)]
#[reflect(Resource, Clone)]
#[type_path = "game"]
pub struct GrabbedItem {
    /// Grabbed entity.
    pub entity: Entity,
    /// Original offset of raycast to the actual origin of the entity.
    pub orig_offset: Vec3,
    /// Distance of the entity to the camera, controlling where it lives
    /// as the camera moves (can change over time).
    pub distance: f32,
    // Original axes of freedom before grabbing.
    orig_axes: LockedAxes,
    #[cfg(feature = "highlighting")]
    orig_mode: HighlightingMode,
    /// Movement from original location to un-stick item.
    movement: f32,
    /// Movement from original location to un-stick item.
    speed: f32,
}

pub fn is_grabbing_item(res: Option<Res<GrabbedItem>>) -> bool {
    res.is_some()
}

/// Force that a grabbed object will be moved.
#[derive(Resource, Reflect, Debug)]
#[reflect(Resource)]
#[type_path = "game"]
pub struct GrabbingForce(pub f32);

impl Default for GrabbingForce {
    fn default() -> Self {
        Self(25.0)
    }
}


/// See if the user is grabbing/dragging/ungrabbing something.
fn on_start_grab(
    _event: On<Start<actions::StartGrab>>,
    inputs: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    grabbable_q: Query<Entity, With<Grabbable>>,
    grabbed_opt: Option<Res<GrabbedItem>>,
) {
    if grabbed_opt.is_some() || (inputs.pressed(KeyCode::AltLeft) || inputs.pressed(KeyCode::AltRight)) {
        // Still holding, release.
        commands.write_message(GrabbingCommand::ReleaseItems);
    } else {
        // Try to grab.
        if let Some(grabbed) = grabbable_q.iter().next() {
            commands.write_message(GrabbingCommand::GrabItem(grabbed));
            commands.entity(grabbed).try_remove::<Grabbable>();
        }
    }
}

/// User drops the item.
fn on_end_grab_drop(
    _event: On<bevy_enhanced_input::prelude::Cancel<actions::ReleaseGrab>>,
    mut commands: Commands,
    grabbed_opt: Option<Res<GrabbedItem>>,
) {
    // Let go.
    if grabbed_opt.is_some() {
        commands.write_message(GrabbingCommand::ReleaseItems);
    } else {
        commands.write_message(GrabbingCommand::CancelGrabItems);
    }
}

/// User long-drops/fires the item.
fn on_end_grab_fire(
    _event: On<Complete<actions::ReleaseGrab>>,
    mut commands: Commands,
    // grabbable_q: Query<Entity, With<Grabbable>>,
    grabbed_opt: Option<Res<GrabbedItem>>,
) {
    // Let go.
    if grabbed_opt.is_some() {
        commands.write_message(GrabbingCommand::ReleaseItems);
    } else {
        commands.write_message(GrabbingCommand::CancelGrabItems);
    }
}

// Extend (e.g. mouse wheel) moves the ideal distance in or out
// from its original position.
//
// Only see these after the hold delay.
fn on_change_grab_distance(
    event: On<Fire<actions::CycleHighlightedItem>>,
    mut grabbed_opt: Option<ResMut<GrabbedItem>>,
) {
    // Still grabbing?
    if let Some(grabbed) = &mut grabbed_opt
    {
        let new_dist = (grabbed.distance + event.value).clamp(0.1, 100.0);
        grabbed.distance = new_dist;
    }

}

fn move_grabbed_item(
    mut commands: Commands,
    mut grabbed: ResMut<GrabbedItem>,
    grabbing_force: Res<GrabbingForce>,

    camera_q: Query<&GlobalTransform, (With<Camera3d>, With<WorldCamera>)>,

    mut gizmos: Gizmos,
    mut phys_info_q: Query<(Forces, &GlobalTransform, &Transform, Option<&LockedAxes>)>,
) {
    let Ok((mut forces, item_global_xfrm, xfrm, _)) = phys_info_q.get_mut(grabbed.entity) else {
        commands.write_message(GrabbingCommand::CancelGrabItems);
        return
    };

    let Ok(cam_global_xfrm) = camera_q.single() else {
        log::warn!("no camera for grabbing {}", grabbed.entity);
        commands.write_message(GrabbingCommand::CancelGrabItems);
        return
    };

    // Compute the desired new location, i.e. the current
    // position plus the camera's position + original distance,
    // then apply an impulse to move to that location.
    let cur_pos = item_global_xfrm.translation() + grabbed.orig_offset;

    let cam_pos = cam_global_xfrm.translation();
    let new_pos = cam_pos + cam_global_xfrm.rotation() * Vec3::NEG_Z * grabbed.distance;

    let offset = new_pos - cur_pos;

    let movement = offset.length();
    if movement > 0.01 {
        grabbed.speed = grabbed.speed.max(0.05) * 1.01;
        *forces.linear_velocity_mut() = offset * grabbed.speed * grabbing_force.0;
        *forces.angular_velocity_mut() = default();
        grabbed.movement += movement;
    } else {
        grabbed.speed *= 0.99;
    }

    // Draw axes from all edges.
    gizmos.axes(*xfrm, grabbing_force.0);

    let mut inv_xfrm = xfrm.clone();
    inv_xfrm.rotate_local_x(std::f32::consts::PI);
    gizmos.axes(inv_xfrm, grabbing_force.0);

    let mut inv_xfrm = xfrm.clone();
    inv_xfrm.rotate_local_y(std::f32::consts::PI);
    gizmos.axes(inv_xfrm, grabbing_force.0);

    let mut inv_xfrm = xfrm.clone();
    inv_xfrm.rotate_local_z(std::f32::consts::PI);
    gizmos.axes(inv_xfrm, grabbing_force.0);
}

fn process_grab_changes(
    mut reader: MessageReader<GrabbingCommand>,

    mut commands: Commands,
    grabbed_opt: Option<Res<GrabbedItem>>,
    styler: Res<GrabbedItemStyle>,
    // grabbing_force: Res<GrabbingForce>,

    camera_q: Query<&GlobalTransform, (With<Camera3d>, With<WorldCamera>)>,

    #[cfg(feature = "highlighting")]
    mut mode: ResMut<HighlightingMode>,

    mut raycast: MeshRayCast,
    // mut gizmos: Gizmos,
    phys_info_q: Query<(Forces, &GlobalTransform, &Transform, Option<&LockedAxes>)>,
) {
    for command in reader.read() {
        match command {
            GrabbingCommand::GrabItem(entity) => {
                let entity = *entity;

                let Ok(cam_global_xfrm) = camera_q.single() else {
                    log::warn!("no camera for grabbing {entity}");
                    continue
                };

                let Ok((_, item_global_xfrm, _, axes)) = phys_info_q.get(entity) else {
                    log::warn!("no physical item {entity}");
                    continue
                };

                // We can have clicked anywhere on the grabbed object,
                // but later compute grab distance based on the center.
                // Account for that here.
                let cam_pos = cam_global_xfrm.translation();
                let cam_dir = cam_global_xfrm.rotation() * Dir3::NEG_Z;
                let cur_pos = item_global_xfrm.translation();
                let hits = raycast.cast_ray(
                    Ray3d::new(cam_pos, cam_dir),
                    &MeshRayCastSettings::default()
                        .with_filter(&|ent| ent == entity)
                        .never_early_exit()
                );
                let new_pos = if let Some(hit) = hits.get(0) {
                    hit.1.point
                } else {
                    cur_pos
                };
                let distance = cam_pos.distance(new_pos);

                commands.insert_resource(GrabbedItem{
                    entity,
                    orig_offset: new_pos - cur_pos,
                    #[cfg(feature = "highlighting")]
                    orig_mode: mode.original_or_enabled(),
                    distance,
                    orig_axes: axes.map_or(default(), |a| *a),
                    movement: 0.,
                    speed: 0.,
                });

                if cfg!(feature = "highlighting") {
                    *mode = HighlightingMode::Busy;
                }

                // Mark as grabbed and make physics user-controllable.
                commands.entity(entity).try_insert((
                    Grabbed,
                    LockedAxes::ROTATION_LOCKED,
                ));
                commands.queue(SleepBody(entity));

                // Insert the outline bundle, whatever it is.
                styler.apply_to(commands.entity(entity));
            }
            GrabbingCommand::ReleaseItems => {
                commands.remove_resource::<GrabbedItem>();
                if let Some(grabbed) = &grabbed_opt {

                    // The entity may have been deleted, so, be extra-careful here.
                    let Ok(mut ent_commands) = commands.get_spawned_entity(grabbed.entity) else {
                        log::warn!("grabbed is gone"); continue
                    };

                    ent_commands.try_remove::<Grabbed>();

                    if grabbed.orig_axes.to_bits() != 0 {
                        ent_commands.try_insert(grabbed.orig_axes);
                    } else {
                        ent_commands.try_remove::<LockedAxes>();
                    }

                    styler.remove_from(ent_commands);

                    // Make sure Avian knows it's awake but without affecting it.
                    commands.queue(WakeBody(grabbed.entity));

                    //
                    if cfg!(feature = "highlighting") {
                        if *mode == HighlightingMode::Busy {
                            *mode = grabbed.orig_mode.original_or_enabled();
                        }
                    }
                }
            }
            GrabbingCommand::CancelGrabItems => {
                commands.remove_resource::<GrabbedItem>();
                *mode = HighlightingMode::Enabled;
            }
        }
    }
}
