use bevy::camera::visibility::RenderLayers;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;

use crate::assets::CommonGuiAssets;
use crate::RENDER_LAYER_DEFAULT;
use crate::RENDER_LAYER_VIEW;
use crate::WorldCamera;
use crate::debug_gui_wants_direct_input;
use crate::is_in_menu;
use crate::is_level_active;
use crate::is_paused;

use super::states_sets::OverlayState;
use super::states_sets::ProgramState;

pub struct CrosshairPlugin;
impl Plugin for CrosshairPlugin {
    fn build(&self, app: &mut App) {
        app
        .init_resource::<CrosshairTargets>()
        .init_resource::<CrosshairMode>()
        .add_systems(
            OnEnter(ProgramState::InGame),
            init_crosshair
        )
        .add_systems(
            OnExit(ProgramState::InGame),
            term_crosshair
        )
        .add_systems(
            Update,
            check_crosshair_visibility
            .run_if(resource_exists_and_equals(CrosshairMode::AimFromCenter))
            .run_if(resource_changed::<State<OverlayState>>)
        )
        .add_systems(
            FixedUpdate,
            (
                check_crosshair_activity_mouse,
                check_crosshair_activity_gamepad,
                update_crosshair,
                update_crosshair_targets,
            )
            .run_if(resource_exists_and_equals(CrosshairMode::AimFromCenter))
            .run_if(not(is_paused))
            .run_if(not(is_in_menu))
            .run_if(is_level_active)
            .run_if(not(debug_gui_wants_direct_input))
            .run_if(in_state(ProgramState::InGame))
        )
        ;
    }
}

/// Marker for the GUI node representing the crosshair,
/// providing in itself a
///
#[derive(Component, Reflect)]
#[reflect(Component)]
#[type_path = "game"]
pub struct Crosshair {
    /// The current "strength" of  in the range [0..1],
    /// 0 corresponding to no pointer movement and 1 to active motion.
    pub current_strength: f32,
}

impl Crosshair {
    pub fn add_activity(&mut self, activity_delta: f32) {
        self.current_strength = (self.current_strength + activity_delta).clamp(0.0, 1.0);
    }
    pub fn is_active(&self) -> bool {
        self.current_strength >= 0.5
    }
}

/// Client marker for an entity that can be "targeted" by the crosshair
/// and thus appear in [CrosshairTargets].
#[derive(Default, Component, Reflect)]
#[reflect(Component)]
#[type_path = "game"]
pub struct CrosshairTargetable;

/// Client marker for an entity that can be "targeted" by the crosshair
/// and thus appear in [CrosshairTargets].
#[derive(Default, Resource, Reflect, PartialEq, Eq, Hash)]
#[reflect(Resource)]
#[type_path = "game"]
pub enum CrosshairMode {
    /// Don't show it.
    Off,
    /// Show in center, "strengthening" as pointer moves.
    #[default]
    AimFromCenter,
}

fn init_crosshair(
    mut commands: Commands,
    gui_assets: Res<CommonGuiAssets>,
) {
    // TODO: make this simpler. It looks like hacks.
    commands.spawn((
        Name::new("Crosshair"),
        DespawnOnExit(ProgramState::InGame),
        Crosshair { current_strength: 0. },

        RenderLayers::from_layers(&[RENDER_LAYER_DEFAULT, RENDER_LAYER_VIEW]),
        Transform::from_xyz(0., 0., -10.),

        // Hidden until CrosshairMode changes.
        Visibility::Hidden,
        Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            position_type: PositionType::Absolute,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        }
    ))
    .with_children(|builder| {
        builder.spawn((
            ImageNode::new(gui_assets.crosshair.clone()),
            Node {
                width: Val::Px(16.),
                height: Val::Px(16.),
                ..default()
            }
        ));
    });
}

fn term_crosshair(mut commands: Commands, crosshair_q: Query<Entity, With<Crosshair>>) {
    for ent in crosshair_q.iter() {
        commands.entity(ent).try_despawn();
    }
}

fn check_crosshair_visibility(
    crosshair_q: Query<Entity, With<Crosshair>>,
    mode: Res<CrosshairMode>,
    mut vis_q: Query<&mut Visibility>,
    overlay: Res<State<OverlayState>>,
) {
    let visible = !is_in_menu(overlay) && *mode == CrosshairMode::AimFromCenter;
    for crosshair in crosshair_q.iter() {
        if let Ok(mut vis) = vis_q.get_mut(crosshair) {
            *vis = if visible { Visibility::Inherited } else { Visibility::Hidden };
        }
    }
}

fn check_crosshair_activity_mouse(
    mut crosshair_q: Query<&mut Crosshair>,
    movement: Res<AccumulatedMouseMotion>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    let activity_delta = if movement.delta.length() < 1.0 {
        -dt
    } else {
        dt * 4.0
    };
    for mut crosshair in crosshair_q.iter_mut() {
        crosshair.add_activity(activity_delta);
    }
}

fn check_crosshair_activity_gamepad(
    mut crosshair_q: Query<&mut Crosshair>,
    gamepad_q: Query<&Gamepad>,
    time: Res<Time>,
) {
    let mut delta = 0.;
    let mut count = 0;
    for gamepad in gamepad_q.iter() {
        for (_, value) in gamepad.analog().all_axes_and_values() {
            delta += value;
            count += 1;
        }
    }
    let dt = time.delta_secs();
    let activity_delta = if delta * (count as f32) < 1.0 {
        -dt
    } else {
        dt * 4.0
    };
    for mut crosshair in crosshair_q.iter_mut() {
        crosshair.add_activity(activity_delta);
    }
}

fn update_crosshair(
    crosshair_q: Single<(Entity, &Crosshair)>,
    child_q: Query<&Children>,
    mut image_q: Query<&mut ImageNode>,
) {
    let Some(image_ent) = child_q.iter_descendants(crosshair_q.0).find(|ent| image_q.contains(*ent)) else {
        error!("no crosshair image");
        return;
    };
    let strength = crosshair_q.1.current_strength;
    let mut image = image_q.get_mut(image_ent).unwrap();
    let new_color = image.color.with_alpha(strength.clamp(0.0, 1.0));
    if image.color != new_color {
        image.color = new_color;
    }
}


/// This tracks the [CrosshairTargetable]s in view of a [WorldCamera]-oriented raycast.
/// The module's systems perform a periodic scan in [FixedUpdate]
/// which changes this value as the raycast hits change.
///
/// The index is owned by the game, though it retains the same Entity reference if possible.
#[derive(Resource, Reflect, Debug, PartialEq, Default)]
#[reflect(Resource, Default)]
#[type_path = "game"]
pub struct CrosshairTargets {
    pub targets: Vec<Entity>,
    pub index: usize,
}

/// This always runs, regardless of [CrosshairMode],
/// to populate [CrosshairTargets].
pub fn update_crosshair_targets(
    camera_q: Single<&GlobalTransform, (With<Camera3d>, With<WorldCamera>)>,
    targetable_q: Query<&CrosshairTargetable>,
    parent_q: Query<&ChildOf>,
    mut raycast: MeshRayCast,
    mut crosshair_targets: ResMut<CrosshairTargets>,
) {
    let gxfrm = *camera_q;
    let ray = Ray3d::new(gxfrm.translation(), gxfrm.rotation() * Dir3::NEG_Z);
    let filter = |ent: Entity| {
        let mut step = ent;
        loop {
            if targetable_q.contains(step) {
                return true
            }
            if let Ok(parent) = parent_q.get(step) {
                step = parent.0
            } else {
                break
            }
        }
        false
    };
    let settings = MeshRayCastSettings::default()
        .with_visibility(RayCastVisibility::Any)    // allow for hidden controller ents
        .never_early_exit()
        .with_filter(&filter)
        ;
    let hits = raycast.cast_ray(ray, &settings);
    let targets = hits.iter().map(|(target, _)| *target).collect::<Vec<_>>();
    let target_opt = crosshair_targets.targets.get(crosshair_targets.index);
    let index = if let Some(old_target) = target_opt.cloned() {
        targets.iter().position(|t| *t == old_target)
    } else {
        None
    };
    let new_crosshair_targets = CrosshairTargets{ targets, index: index.unwrap_or(0) };
    crosshair_targets.set_if_neq(new_crosshair_targets);
}

/// Format a string reporting which items are currently visible
/// in the crosshair, indicating the selected index.
pub fn report_crosshair_targets(
    crosshair_target: &CrosshairTargets,
    targets_q: &Query<Option<&Name>>,
) -> Option<String> {
    if crosshair_target.targets.is_empty() {
        return None
    }

    let mut message = "[".to_string();
    let mut started = false;
    let current = crosshair_target.targets.get(crosshair_target.index).cloned();
    for ent in &crosshair_target.targets {
        let Ok(name_opt) = targets_q.get(*ent) else { continue };
        if !started {
            started = true
        } else {
            message += ", "
        }
        if current == Some(*ent) {
            message += "*";
        }
        let segment = if let Some(name) = name_opt {
            format!("{ent}: \"{name}\"")
        } else {
            ent.to_string()
        };
        message += &segment;
    }
    message += "]";
    Some(message)
}
