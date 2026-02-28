use bevy::camera::visibility::RenderLayers;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;

use crate::assets::CommonAssets;
use crate::RENDER_LAYER_DEFAULT;
use crate::RENDER_LAYER_VIEW;
use crate::WorldCamera;
use crate::is_in_menu;

use super::states_sets::OverlayState;
use super::states_sets::ProgramState;

pub struct CrosshairPlugin;
impl Plugin for CrosshairPlugin {
    fn build(&self, app: &mut App) {
        app
        .init_resource::<CrosshairTarget>()
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
            .run_if(resource_changed::<State<OverlayState>>)
        )
        .add_systems(
            Update,
            (
                check_crosshair_activity,
                update_crosshair,
                check_crosshair_target,
            )
            .run_if(in_state(ProgramState::InGame))
        )
        ;
    }
}

/// Marker for the GUI node representing the crosshair,
/// providing in itself a
///
#[derive(Component)]
pub struct Crosshair {
    /// The current "strength" of  in the range [0..1],
    /// 0 corresponding to no pointer movement and 1 to active motion.
    pub current_strength: f32,
}

impl Crosshair {
    pub fn is_active(&self) -> bool {
        self.current_strength >= 0.5
    }
}

/// Client marker for an entity that can be targeted by the crosshair.
#[derive(Default, Component, Reflect)]
#[reflect(Component)]
#[type_path = "game"]
pub struct CrosshairTargetable;

fn init_crosshair(
    mut commands: Commands,
    gui_assets: Res<CommonAssets>,
    mut crosshair_q: Query<&mut Visibility, With<Crosshair>>,
) {
    if let Ok(mut vis) = crosshair_q.single_mut() {
        *vis = Visibility::Visible;
    } else {
        commands.spawn((
            Name::new("Crosshair"),
            DespawnOnExit(ProgramState::InGame),
            Crosshair { current_strength: 0. },
            RenderLayers::from_layers(&[RENDER_LAYER_DEFAULT, RENDER_LAYER_VIEW]),
            Transform::from_xyz(0., 0., -10.),
            Visibility::Visible,
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
}

fn term_crosshair(mut vis: Single<&mut Visibility, With<Crosshair>>) {
    **vis = Visibility::Hidden;
}

fn check_crosshair_visibility(
    crosshair_q: Single<Entity, With<Crosshair>>,
    mut vis_q: Query<&mut Visibility>,
    overlay: Res<State<OverlayState>>,
) {
    let visible = !is_in_menu(overlay);
    if let Ok(mut vis) = vis_q.get_mut(*crosshair_q) {
        *vis = if visible { Visibility::Inherited } else { Visibility::Hidden };
    }
}

fn check_crosshair_activity(
    mut crosshair_q: Single<&mut Crosshair>,
    movement: Res<AccumulatedMouseMotion>,
    time: Res<Time>,
) {
    let dt = time.delta_secs() * 4.0;
    let activity_delta = if movement.delta.length() < 1.0 {
        -dt
    } else {
        dt
    };
    crosshair_q.current_strength = (crosshair_q.current_strength + activity_delta).clamp(0.0, 1.0);
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
    image.color = image.color.with_alpha(strength.clamp(0.0, 1.0));
}


/// This tracks the [CrosshairTargetable]s in view of a [WorldCamera]-oriented raycast.
/// The module's systems perform a periodic scan in [FixedUpdate]
/// which changes this value as the raycast hits change.
#[derive(Resource, Reflect, Debug, PartialEq, Default)]
#[reflect(Resource, Default)]
#[type_path = "game"]
pub struct CrosshairTarget { pub targets: Vec<Entity> }

/// See if we're looking at something clickable.
fn check_crosshair_target(
    crosshair_q: Single<&Crosshair>,
    camera_q: Single<&GlobalTransform, (With<Camera3d>, With<WorldCamera>)>,
    // level: Res<LevelMetadata>,
    targetable_q: Query<&CrosshairTargetable>,
    parent_q: Query<&ChildOf>,
    // scene_q: Query<&SceneRoot>,
    // func_q: Query<(Option<&FuncButton>, Option<&FuncTile>)>,
    // level_state: Res<State<LevelState>>,
    mut raycast: MeshRayCast,
    mut cur_target_mut: ResMut<CrosshairTarget>,
) {
    let crosshair = *crosshair_q;
    if !crosshair.is_active() {
        return;
    }

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

    // let ctr = std::cell::RefCell::new(2u32);
    // let gather_and_limit_seen = move |_: Entity| { *ctr.borrow_mut() -= 1; *ctr.borrow() == 0 };
    let settings = MeshRayCastSettings::default()
        .with_visibility(RayCastVisibility::Any)    // allow for hidden controller ents
        .always_early_exit()
        .with_filter(&filter)
        ;
    let hits = raycast.cast_ray(ray, &settings);
    // // Ignore if same targets.
    // if cur_target.is_none_or(|cur_target| cur_target.targets != targets) {
    //     return;
    // }

    // // Clean up previous target before we overwrite the resource.
    // if let Some(cur) = cur_target.take() {
    //     commands.insert_resource(CrosshairDetarget(cur.0));
    // }

    let targets = hits.iter().map(|(target, _)| *target).collect::<Vec<_>>();
    let crosshair_target = CrosshairTarget{ targets };

    // dbg!(&crosshair_target);
    // cur_target_mut.set_if_neq(crosshair_target);
    *cur_target_mut = crosshair_target;
}
