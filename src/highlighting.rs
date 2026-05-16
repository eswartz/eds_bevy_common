//! 3D highlighting support.
//!
//! This builds on [CrosshairTargets] to allow cycling through
//! items visible along a line of sight.
//!
//! The `CycleHighlightedItem` action switches between these.
//!
//! Highlighting is represented with the [Highlighted] component.
//!
use bevy_mod_outline::{InheritOutline, OutlineStencil};
use bevy_mod_outline::{OutlineMode, OutlinePlugin, OutlineVolume};
use bevy_seedling::sample::PlaybackSettings;
use bevy_seedling::prelude::*;

use bevy::prelude::*;
use rand::RngExt as _;
use rand::seq::IndexedRandom as _;

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
                OnEnter(LevelState::Configuring),
                clear_highlighted
                    .run_if(not(is_paused))
                    .run_if(not(is_in_menu))
                    .run_if(is_level_active)
                    .run_if(not(debug_gui_wants_direct_input))
                    .run_if(in_state(ProgramState::InGame))
            )
            .add_systems(
                FixedUpdate,
                (
                    check_actions,
                    cycle_targetables,
                    update_highlightable,
                    update_highlighting_mode,
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
    #[must_use]
    pub fn original_or_enabled(&self) -> HighlightingMode {
        if *self == HighlightingMode::Busy {
            HighlightingMode::Enabled
        } else {
            *self
        }
    }

    #[must_use]
    pub fn toggle_enabled(&self) -> HighlightingMode {
        match self {
            HighlightingMode::Disabled => HighlightingMode::Enabled,
            HighlightingMode::Enabled => HighlightingMode::Disabled,
            HighlightingMode::Busy => HighlightingMode::Busy,
        }
    }

    pub fn is_enabled(&self) -> bool {
        *self == HighlightingMode::Enabled
    }
    pub fn is_disabled(&self) -> bool {
        *self == HighlightingMode::Disabled
    }
}

pub fn is_highlighting_enabled(res: Res<HighlightingMode>) -> bool {
    res.is_enabled()
}

/// Marker for CountAccumulator.
struct HighlightedItemCycle;

fn clear_highlighted(
    mut commands: Commands,
    style: Res<HighlightedItemStyle>,
    hilit_q: Query<Entity, With<Highlighted>>,
) {
    for ent in hilit_q.iter() {
        let mut ent_commands = commands.entity(ent);
        ent_commands.remove::<Highlighted>();
        style.remove_from(ent_commands);
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

    hilit_q: Query<Entity, (With<CrosshairTargetable>, With<Highlighted>)>,
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
    if let Some(new_item) = crosshair_targets.targets.get(new_index)
    && !hilit_q.contains(*new_item) {
        commands.entity(*new_item).try_insert(Highlighted);
    }
}

/// When [HighlightingMode] changes, remove [Highlighted].
fn update_highlighting_mode(
    mut commands: Commands,
    fx: Res<CommonFxAssets>,
    mode: Res<HighlightingMode>,
    mut crosshair_q: Single<&mut Crosshair>,
    hilit_q: Query<Entity, (With<CrosshairTargetable>, With<Highlighted>)>,
    style: Res<HighlightedItemStyle>,
    mut targets: ResMut<CrosshairTargets>,
) {
    if !mode.is_changed() {
        return
    }

    if mode.is_enabled() {
        // Turn on, and re-scan.
        crosshair_q.current_strength = 1.0;
    } else if mode.is_disabled() {
        // Turn off.
        crosshair_q.current_strength = 0.0;

        let mut any = !targets.targets.is_empty();
        if any {
            targets.targets.clear();
            targets.index = 0;
        }

        for ent in hilit_q.iter() {
            let mut ent_commands = commands.entity(ent);
            ent_commands.try_remove::<Highlighted>();
            style.remove_from(ent_commands);
            any = true;
        }

        if any && cfg!(feature = "firewheel") {
            let mut rng = rand::rng();
            commands.spawn((
                UiSfx,
                SamplePlayer::new(
                    (*[&fx.deselect]
                        .choose(&mut rng)
                        .expect("we have one"))
                    .clone(),
                ),
                PlaybackSettings {
                    speed: rng.random_range(0.9..1.1),
                    ..default()
                },
                VolumeNode::from_linear(rng.random_range(0.25..0.5)),
            ));
        }
    }
}

/// When [CrosshairTargets] changes, remove/add [Highlighted].
fn update_highlightable(
    mut commands: Commands,
    fx: Res<CommonFxAssets>,
    targets: Res<CrosshairTargets>,
    style: Res<HighlightedItemStyle>,
    now_hovered_q: Query<Entity, (With<CrosshairTargetable>, Added<Highlighted>)>,
    was_hovered_q: Query<Entity, (With<OutlineVolume>, With<CrosshairTargetable>, Without<Highlighted>)>,
) {
    if !targets.is_changed() {
        return
    }

    for ent in was_hovered_q.iter() {
        style.remove_from(commands.entity(ent));
    }

    let mut any = false;
    for ent in now_hovered_q.iter() {
        style.apply_to(commands.entity(ent));
        any = true;
    }

    if any && cfg!(feature = "firewheel") {
        let mut rng = rand::rng();
        commands.spawn((
            UiSfx,
            SamplePlayer::new(
                (*[&fx.select]
                    .choose(&mut rng)
                    .expect("we have one"))
                .clone(),
            ),
            PlaybackSettings {
                speed: rng.random_range(0.9..1.1),
                ..default()
            },
            VolumeNode::from_linear(rng.random_range(0.25..0.5)),
        ));
    }
}
