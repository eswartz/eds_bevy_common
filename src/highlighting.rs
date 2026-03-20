//! 3D highlighting support.
//!
//! This builds on [CrosshairTargets] to allow cycling through
//! items visible along a line of sight.
//!
//! The `CycleHighlightedItem` action switches between these.
//!
//! Highlighting is represented with the [Highlighted] component.
//!
#[cfg(feature = "input_lim")]
use bevy_enhanced_input::action::ActionState;
use bevy_mod_outline::{InheritOutline, OutlineStencil};
use bevy_mod_outline::{OutlineMode, OutlinePlugin, OutlineVolume};
use bevy_seedling::sample::PlaybackSettings;
use bevy_seedling::prelude::*;

use bevy::prelude::*;
use rand::RngExt as _;
use rand::seq::IndexedRandom as _;

#[cfg(feature = "input_lim")]
use leafwing_input_manager::prelude::*;
#[cfg(feature = "input_bei")]
use bevy_enhanced_input::prelude::*;

use crate::*;

pub struct HighlightingPlugin;

impl Plugin for HighlightingPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<OutlinePlugin>() {
            app.add_plugins(OutlinePlugin);
        }
        app
            .add_message::<ChangeHighlightedItem>()

            .init_resource::<HighlightingMode>()
            .init_resource::<HighlightedItemStyle>()
            .init_resource::<CountAccumulator<HighlightedItemCycle>>()

            .add_systems(
                FixedUpdate,
                (
                    check_actions,
                    cycle_targetables,
                    update_highlight_ui.run_if(resource_changed::<CrosshairTargets>),
                ).chain()
                    .run_if(is_highlighting_enabled)
                    .run_if(not(is_paused))
                    .run_if(not(is_in_menu))
                    .run_if(is_level_active)
                    .run_if(not(debug_gui_wants_direct_input))
                    .run_if(in_state(ProgramState::InGame))
                ,
            )
        ;
    }
}

/// This resource defines the default style for highlighted items.
/// The given components are added (and removed) as needed.
#[derive(Resource, Reflect)]
#[reflect(Resource)]
#[type_path = "game"]
pub struct HighlightedItemStyle {
    pub volume: OutlineVolume,
    pub mode: OutlineMode,
    pub stencil: Option<OutlineStencil>,
    pub inherit: Option<InheritOutline>,
}

impl Default for HighlightedItemStyle {
    fn default() -> Self {
        Self {
            volume: OutlineVolume {
                visible: true,
                width: 2.0,
                colour: Color::WHITE.with_alpha(0.5),
            },
            mode: OutlineMode::FloodFlat,
            stencil: None,
            inherit: None,
        }
    }
}

impl HighlightedItemStyle {
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
        ent_commands.remove::<OutlineVolume>();
        if self.stencil.is_some() {
            ent_commands.remove::<OutlineStencil>();
        }
        if self.inherit.is_some() {
            ent_commands.remove::<InheritOutline>();
        }
    }
}

/// When this resource exists and is Enabled, tells whether highlighting systems should function.
#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Resource)]
#[type_path = "game"]
pub enum HighlightingMode {
    #[default]
    Disabled,
    Enabled,
    /// This means that e.g. the grabbing system is active.
    Busy,
}
impl HighlightingMode {
    pub(crate) fn original_or_enabled(&self) -> HighlightingMode {
        if *self == HighlightingMode::Busy {
            HighlightingMode::Enabled
         } else {
            *self
         }
    }
}

pub fn is_highlighting_enabled(res: Res<HighlightingMode>) -> bool {
    *res == HighlightingMode::Enabled
}

/// Marker for CountAccumulator.
struct HighlightedItemCycle;

#[cfg(feature = "input_lim")]
fn check_actions(
    mut commands: Commands,
    actions: Res<ActionState<UserAction>>,
    crosshair_targets: Res<CrosshairTargets>,
    cycle_action_q: Query<(&ActionEvents, &Action<actions::CycleHighlightedItem>), With<PlayerAction>>,
    mut cycle_ctr: ResMut<CountAccumulator<HighlightedItemCycle>>,
) {
    if actions.just_pressed(&UserAction::CycleHighlightedItem) {
        cycle_ctr.reset();
    }

    if let Some(dir) = cycle_ctr.add_and_test(actions.value(&UserAction::CycleHighlightedItem))
    && !crosshair_targets.targets.is_empty() {
        commands.write_message(ChangeHighlightedItem(dir as isize));
    }

    if actions.just_pressed(&UserAction::CycleHighlightedItem) {
        cycle_ctr.reset();
    }

}

#[cfg(feature = "input_bei")]
fn check_actions(
    mut commands: Commands,

    crosshair_targets: Res<CrosshairTargets>,
    cycle_action_q: Query<(&ActionEvents, &Action<actions::CycleHighlightedItem>), With<PlayerAction>>,
    mut cycle_ctr: ResMut<CountAccumulator<HighlightedItemCycle>>,
) {
    if let Some((cycle_events, cycle_action)) = cycle_action_q.iter().next() {
        if cycle_events.contains(ActionEvents::START) {
            cycle_ctr.reset();
        }

        if let Some(dir) = cycle_ctr.add_and_test(**cycle_action)
        && !crosshair_targets.targets.is_empty() {
            commands.write_message(ChangeHighlightedItem(dir as isize));
        }

        if cycle_events.contains(ActionEvents::COMPLETE) {
            cycle_ctr.reset();
        }
    }
}

/// When sent, advance the current item by the given increment.
#[derive(Message, Debug)]
struct ChangeHighlightedItem(isize);

/// Update the [Highlighted] item if [CrosshairTargets] changes or the
/// [CycleHighlightedItem] event changes the item.
fn cycle_targetables(
    mut commands: Commands,

    mut reader: MessageReader<ChangeHighlightedItem>,

    hilit_q: Query<Entity, (With<Spawned>, With<Highlighted>)>,
    // highlighted_opt: Option<Res<HighlightedItem>>,
    mut crosshair_targets: ResMut<CrosshairTargets>,
) {
    // What was marked Highlighted last frame?
    let old_items = hilit_q.iter().collect::<Vec<_>>();

    let mut first_exist = None;

    // Remove any that are no longer in the crosshair
    // and remember the first candidate that still is.
    for ent in &old_items {
        if crosshair_targets.targets.contains(ent) {
            if first_exist.is_none() {
                first_exist = Some(*ent)
            }
        } else {
            commands.entity(*ent).try_remove::<Highlighted>();
        }
    }

    // See where we index now into the crosshair list.
    let mut new_index = if let Some(first) = &first_exist {
        crosshair_targets.targets.iter().position(|e| *e == *first).expect("we found it above") as isize
    } else {
        0
    };

    // Apply cycle actions.
    for event in reader.read() {
        new_index = new_index.wrapping_add(event.0);
    }

    let new_index = if !crosshair_targets.targets.is_empty() {
        new_index.rem_euclid(crosshair_targets.targets.len() as isize) as usize
    } else {
        0
    };

    // Update resource only if changed.
    if crosshair_targets.index != new_index {
        crosshair_targets.index = new_index;
        if let Some(first) = &first_exist {
            commands.entity(*first).try_remove::<Highlighted>();
        }
    }

    // Highlight the new item, if new.
    if let Some(new_item) = crosshair_targets.targets.get(new_index) {
        if !hilit_q.contains(*new_item) {
            commands.entity(*new_item).try_insert(Highlighted);
        }
    }
}

/// When [CrosshairTargets] changes (see registration), remove/add [Highlighted].
fn update_highlight_ui(
    mut commands: Commands,
    fx: Res<CommonFxAssets>,
    style: Res<HighlightedItemStyle>,
    now_hovered_q: Query<Entity, (With<Spawned>, Added<Highlighted>)>,
    was_hovered_q: Query<Entity, (With<OutlineVolume>, With<Spawned>, Without<Highlighted>)>,
) {
    for ent in was_hovered_q.iter() {
        style.remove_from(commands.entity(ent));
    }

    let mut any = false;
    for ent in now_hovered_q.iter() {
        let mut ent_commands = commands.entity(ent);
        ent_commands.try_insert(Highlighted);
        style.apply_to(ent_commands);
        any = true;
    }

    if any && cfg!(feature = "firewheel") {
        let mut rng = rand::rng();
        commands.spawn((
            UiSfx,
            SamplePlayer::new(
                (*[&fx.select]
                    .choose(&mut rng)
                    .unwrap())
                .clone(),
            ),
            PlaybackSettings {
                speed: rng.random_range(0.9..1.1),
                ..default()
            },
            VolumeNode::from_linear(rng.random_range(0.85..1.0)),
        ));
    }
}
