use std::any::Any;
use std::any::TypeId;
use std::path::PathBuf;
use std::time::Duration;

use bevy::asset::AssetPath;
use bevy::camera::visibility::RenderLayers;
use bevy::color::palettes::tailwind;
use bevy::ecs::system::SystemParam;
use bevy::ecs::system::lifetimeless::Read;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::reflect::Typed;
use bevy::window::CursorGrabMode;
use bevy::window::CursorOptions;
use bevy::window::PrimaryWindow;
use bevy::window::WindowFocused;
use bevy_asset_loader::prelude::*;
use bevy_seedling::prelude::MainBus;

use crate::physics::*;
use crate::prelude::*;

use super::audio::UserVolume;
use super::lifecycle::PauseState;
use super::states_sets::OverlayState;
use super::states_sets::ProgramState;

pub struct GuiPlugin;

impl Plugin for GuiPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy_tweening::TweeningPlugin>() {
            app.add_plugins(bevy_tweening::TweeningPlugin);
        }
        app
        .insert_resource(GuiState::default())
        .insert_resource(UiFont(default()))

        .init_resource::<GrabState>()
        .add_message::<GrabCursor>()
        .init_resource::<PhysicsPaused>()
        .add_systems(
            Update,
            (
                ensure_ui_font.run_if(resource_exists_and_changed::<UiFontPath>),
                update_ui_alpha,
                apply_ui_alpha,
            )
        )
        .add_systems(OnEnter(ProgramState::InGame),
            (
                update_gui_state,    // initialize
                ensure_font_assets,
                grab_cursor_for_game,
                setup_gui_nodes,
            )
            .chain()
        )
        .add_systems(OnTransition { exited: ProgramState::InGame, entered: ProgramState::InGame },
            (
                setup_gui_nodes,
            )
            .chain()
        )
        .add_systems(OnEnter(OverlayState::Hidden),
            grab_cursor_for_game,
        )
        .add_systems(OnExit(OverlayState::Hidden),
            ungrab_cursor_for_overlay,
        )
        .add_systems(OnEnter(ProgramState::Initializing),
            on_loading)
        .add_systems(OnExit(ProgramState::Initializing),
            on_loading_finished)
        .add_systems(OnEnter(OverlayState::Loading),
            on_loading)
        .add_systems(OnExit(OverlayState::Loading),
            on_loading_finished)

        // .add_systems(
        //     OnTransition{ exited: GameplayState::Playing, entered: GameplayState::Setup },
        //     (
        //         hide_instructions,
        //         reset_instructions,
        //     )
        // )
        // .add_systems(OnTransition{ exited: ProgramState::InGame, entered: ProgramState::LaunchMenu },
        //     (
        //         reset_instructions,
        //     )
        //     .
        //     chain()
        // )
        .add_systems(OnExit(OverlayState::Hidden),
            hide_instructions,
        )
        .add_systems(OnEnter(LevelState::Playing),
            show_instructions,
        )
        .add_systems(OnExit(LevelState::Playing),
            (
                hide_instructions,
                reset_instructions,
            )
        )
        .add_systems(
            FixedUpdate,
            check_grab_focus_state.run_if(in_state(ProgramState::InGame))
        )

        .add_systems(
            FixedUpdate,
            update_gui_state.run_if(resource_changed::<GuiState>),
        )
        .add_systems(
            Update,
            (
                update_pause_ui,
                update_mute_ui,
                update_physics_pause_ui,
            )
            // .in_set(InteractionSystems)
            .run_if(in_state(ProgramState::InGame))
        )
        // .add_systems(
        //     Update,
        //     update_status_messages
        //     .in_set(InteractionSystems)
        //     .run_if(in_state(ProgramState::InGame))
        // )
        ;
    }
}

/// Control the UI alpha of the immediate node.
/// The value is multiplied by any others down the tree.
#[derive(Component, Reflect)]
#[component(storage = "SparseSet")]
#[reflect(Component)]
#[type_path = "game"]
pub struct UiNodeAlpha(pub f32);

impl Default for UiNodeAlpha {
    fn default() -> Self {
        Self(1.0)
    }
}

/// Computed alpha from parents and self.
#[derive(Component, Reflect)]
#[component(storage = "SparseSet")]
#[reflect(Component)]
#[type_path = "game"]
pub struct UiNodeComputedAlpha {
    pub(crate) alpha: f32,
    pub(crate) orig_values: HashMap<TypeId, f32>,
}

impl Default for UiNodeComputedAlpha {
    fn default() -> Self {
        Self {
            alpha: 1.0,
            orig_values: default(),
        }
    }
}

fn update_ui_alpha(
    mut commands: Commands,
    alpha_q: Query<(Entity, &UiNodeAlpha)>,
    parent_q: Query<&ChildOf>,
    child_q: Query<&Children>,
    mut comp_alpha_q: Query<&mut UiNodeComputedAlpha>,
    color_q: Query<(
        Option<&ImageNode>,
        Option<&TextColor>,
        Option<&TextShadow>,
        Option<&Sprite>,
    )>,
) {
    for (ent, alpha) in alpha_q.iter() {
        // Figure the alpha for this (child) node.
        let mut full_alpha = alpha.0;
        parent_q.iter_ancestors(ent).for_each(|parent| {
            if let Ok((_, parent_alpha)) = alpha_q.get(parent) {
                full_alpha *= parent_alpha.0;
            }
        });

        // Be sure we have UiNodeAlpha on child nodes.
        child_q.iter_descendants(ent).for_each(|kid| {
            if !alpha_q.contains(kid)
            && let Ok((a, b, c, d)) = color_q.get(kid)
            && (a.is_some() || b.is_some() || c.is_some() || d.is_some()) {
                commands.entity(kid).try_insert(UiNodeAlpha(1.0));
            }
        });

        if let Ok(mut comp) = comp_alpha_q.get_mut(ent) {
            // Only change the alpha.
            #[expect(clippy::float_cmp, reason = "binary diff checking")]
            if comp.alpha != full_alpha {
                comp.alpha = full_alpha;

                commands.entity(ent).try_insert((
                    if full_alpha <= 0.0 {
                        Visibility::Hidden
                    } else {
                        Visibility::Inherited
                    },
                ));
            }
        } else {
            // Remember the baseline values for alphas.
            let mut orig_values = HashMap::default();
            if let Ok((im, text, shadow, sprite)) = color_q.get(ent) {
                if let Some(im) = im {
                    orig_values.insert(im.type_id(), im.color.alpha());
                }
                if let Some(text) = text {
                    orig_values.insert(text.type_id(), text.alpha());
                }
                if let Some(shadow) = shadow {
                    orig_values.insert(shadow.type_id(), shadow.color.alpha());
                }
                if let Some(sprite) = sprite {
                    orig_values.insert(sprite.type_id(), sprite.color.alpha());
                }
            }
            commands.entity(ent).try_insert((
                UiNodeComputedAlpha {
                    alpha: full_alpha,
                    orig_values,
                },
                if full_alpha <= 0.0 {
                    Visibility::Hidden
                } else {
                    Visibility::Inherited
                },
            ));
        }
    }
}

fn apply_ui_alpha(
    mut comp_alpha_q: Query<(
        &UiNodeComputedAlpha,
        Option<&mut ImageNode>,
        Option<&mut TextColor>,
        Option<&mut TextShadow>,
        Option<&mut Sprite>,
    ), Or<(
        Changed<UiNodeComputedAlpha>,
        Changed<UiNodeAlpha>,
    )>>
) {
    for (alpha, im, text, shadow, sprite) in comp_alpha_q.iter_mut() {
        if let Some(mut im) = im {
            let orig_alpha = alpha.orig_values.get(&ImageNode::type_info().type_id()).unwrap_or(&1.0);
            im.color.set_alpha(alpha.alpha * orig_alpha);
        }
        if let Some(mut text) = text {
            let orig_alpha = alpha.orig_values.get(&TextColor::type_info().type_id()).unwrap_or(&1.0);
            text.set_alpha(alpha.alpha * orig_alpha);
        }
        if let Some(mut shadow) = shadow {
            let orig_alpha = alpha.orig_values.get(&TextShadow::type_info().type_id()).unwrap_or(&1.0);
            shadow.color.set_alpha(alpha.alpha * orig_alpha);
        }
        if let Some(mut sprite) = sprite {
            let orig_alpha = alpha.orig_values.get(&Sprite::type_info().type_id()).unwrap_or(&1.0);
            sprite.color.set_alpha(alpha.alpha * orig_alpha);
        }
    }
}

/// Define to define the asset path to the UI font, to be loaded in Startup.
/// Use this OR UiFont.
#[derive(Resource, Reflect)]
#[reflect(Resource)]
#[type_path = "game"]
pub struct UiFontPath(pub PathBuf);

/// Define to define the font for UI. Overrides [UiFontPath]. Defined after Startup.
#[derive(Resource, Reflect)]
#[reflect(Resource)]
#[type_path = "game"]
pub struct UiFont(pub Handle<Font>);

fn ensure_ui_font(
    mut commands: Commands,
    assets: Res<AssetServer>,
    ui_font_path: Option<Res<UiFontPath>>,
) {
    if let Some(path) = ui_font_path {
        commands.insert_resource(UiFont(assets.load(AssetPath::from_path(&path.0))));
    } else {
        commands.insert_resource(UiFont(default()));
    }
}

fn ensure_font_assets(
    world: &mut World,
) {
    world.init_collection::<CommonGuiAssets>();

    // Force font data to load, since it seems to take an indeterminate amount of time (0.18.1)
    let assets = world.resource::<CommonGuiAssets>();
    world.spawn((
        DespawnOnEnter(GameplayState::Playing),
        TextFont {
            font: assets.emoji_icon_font.clone().into(),
            .. default()
        },
        Text::new("🚀\u{1F508}\u{1F6AB}\u{23f1}\u{fe0f}"), // various emoji used below
    ));
}

#[derive(Component)]
pub struct LoadingScreen;

pub fn on_loading(
    mut commands: Commands,
    ui_font: Res<UiFont>,
) {
    let ent_commands = commands.spawn((
        Name::new("Loading..."),
        LoadingScreen,
    ));
    setup_loading_screen(ent_commands, ui_font);
}

pub fn on_loading_finished(
    mut commands: Commands,
    loading_q: Query<Entity, With<LoadingScreen>>,
) {
    for ent in loading_q.iter() {
        commands.entity(ent).try_despawn();
    }
}

pub fn setup_loading_screen(
    mut ent_commands: EntityCommands,
    ui_font: Res<UiFont>,
) -> Entity {
    let icon_size = 32.0f32;

    ent_commands.try_insert((
        Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            .. default()
        },
        BackgroundColor(tailwind::BLUE_950.with_alpha(0.75).into()),
        RenderLayers::from_layers(&[RENDER_LAYER_UI]),
    ))
    .with_children(|builder| {
        builder.spawn((
            Text::new(
                "Loading...",
            ),
            TextFont {
                font: ui_font.0.clone().into(),
                font_size: FontSize::Px(icon_size),
                .. default()
            },
            TextColor(Color::WHITE.with_alpha(0.5)),
        ));
    })
    .id()
}

/// Indicate the desire to change the cursor grab state
/// (false = not grabbed, true = grabbed in the "best way").
///
/// Do not modify the state of [CursorOptions] yourself,
/// to avoid overlapping responsibilities.
#[derive(Message, Debug)]
pub struct GrabCursor(pub bool);

pub const GRABBED_MODE: CursorGrabMode = CursorGrabMode::Locked;

impl Default for GrabState {
    fn default() -> Self {
        Self {
            grabbed: false,
            options: CursorOptions{
                visible: false,
                grab_mode: GRABBED_MODE,
                .. default()
            }
        }
    }
}

/// Flags
#[derive(Resource, Debug, Clone, PartialEq, Reflect)]
#[reflect(Resource)]
#[type_path = "game"]
pub struct GuiState {
    pub enabled: bool,
    pub show_stats: bool,
    pub show_inspector: bool,
    /// Show inspector even if !enabled.
    pub show_inspector_always: bool,
    pub show_physics_gizmos: bool,
    pub show_player_status: bool,
}

impl Default for GuiState {
    fn default() -> Self {
        Self {
            enabled: false,
            show_stats: false,
            show_inspector: true,
            show_inspector_always: false,
            show_physics_gizmos: false,
            show_player_status: false,
        }
    }
}

impl GuiState {
    pub fn is_debug_ui_inspector_visible(&self) -> bool {
        self.show_inspector_always ||
        (self.enabled && self.show_inspector)
    }
    pub fn show_cursor(&self) -> bool {
        self.enabled || self.show_inspector_always
    }
    pub fn center_cursor(&self) -> bool {
        !self.enabled
    }
}

pub fn is_debug_ui_enabled(gui_state: Option<Res<GuiState>>) -> bool {
    gui_state.is_some_and(|g| g.enabled)
}

pub fn is_debug_ui_inspector_visible(gui_state: Option<Res<GuiState>>) -> bool {
    gui_state.is_some_and(|g| g.is_debug_ui_inspector_visible())
}

/// Tells whether we're in a mode where the [GameStatusArea] is displayed.
#[derive(Resource, Debug, Clone, PartialEq)]
pub(crate) struct StatusVisible(pub bool);

/// State held while a grab operation is occurring.
#[derive(Resource)]
pub(crate) struct GrabState{ grabbed: bool, options: CursorOptions }

fn update_gui_state(
    state: Res<GuiState>,
    fps_visible: Option<ResMut<StatsOverlayVisible>>,
    status_visible: Option<ResMut<StatusVisible>>,
    mut gizmo_config: ResMut<GizmoConfigStore>,
) {
    if let Some(mut fps_visible) = fps_visible {
        fps_visible.0 = state.show_stats || state.enabled;
    }
    if let Some(mut status_visible) = status_visible {
        status_visible.0 = state.show_player_status;
    }

    if let Some(physics_gizmos) = gizmo_config.get_config_mut::<PhysicsGizmos>() {
        let was_enabled = physics_gizmos.0.enabled;
        if was_enabled != state.show_physics_gizmos {
            // Only trigger on actual change,
            // to avoid avian3d::debug_render::change_mesh_visibility
            // showing everything without recourse.
            physics_gizmos.0.enabled = state.show_physics_gizmos;
        }
    }
}

fn grab_cursor_for_game(
    mut commands: Commands,
    gui_state: Res<GuiState>,
) {
    commands.write_message(GrabCursor(!gui_state.show_cursor()));
}

fn ungrab_cursor_for_overlay(
    mut commands: Commands,
) {
    commands.write_message(GrabCursor(false));
}

/// This is the logic managing [GrabCursor] and [WindowFocused] messages
/// so that [CursorOptions] is updated in a centralized way.
fn check_grab_focus_state(
    mut grab: MessageReader<GrabCursor>,
    mut focused: MessageReader<WindowFocused>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    overlay_state: Res<State<OverlayState>>,
    gui_state: ResMut<GuiState>,
    mut grab_state: ResMut<GrabState>,
    window_cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,

    mut awaiting: Local<Option<bool>>,
) {
    let mut cursor_options = window_cursor_options.into_inner();

    // Helper to determine if we want to grab or ungrab the mouse.
    let grab_on_mode_switch = || {
        if mouse_buttons.get_pressed().next().is_some() {
            // When pressing a button outside UI, take focus.
            overlay_state.is_hidden()
        } else {
            // Else, take focus if there's no need to see the mouse.
            overlay_state.is_hidden() && !gui_state.enabled
        }
    };

    let mut desired_grab: Option<bool> = None;

    // Just check the last [WindowFocused] event (current state).
    if let Some(message) = focused.read().last() {
        if !message.focused {
            desired_grab = Some(false);
        } else {
            desired_grab = Some(grab_on_mode_switch());
        }
    }

    // Obey the last [GrabCursor] message (current desired state)
    if let Some(message) = grab.read().last() {
        desired_grab = Some(message.0);
    }

    // If there's been no request this frame,
    // are we coming back to await a previous grab event completion?
    // (If so, re-request until it takes.)
    if desired_grab.is_none()
    && let Some(awaited_grabbed) = awaiting.take() {
        let current_grabbed = cursor_options.grab_mode != CursorGrabMode::None;
        if awaited_grabbed != current_grabbed {
            // Double-check that we're allowed to before trying again.
            if awaited_grabbed && !grab_on_mode_switch() {
                // Ignore.
            } else {
                // We still want switch modes, so keep poking.
                desired_grab = Some(awaited_grabbed);
            }
        }
    }

    let Some(grab) = desired_grab else { return };

    if grab {
        // Take that cursor!
        cursor_options.grab_mode = GRABBED_MODE;
        cursor_options.visible = false;
        // We need to wait for it to take effect in the OS.
        *awaiting = Some(true);

        grab_state.grabbed = true;
    } else {
        //
        if grab_state.grabbed {
            grab_state.grabbed = false;
            // Restore the [CursorOptions].
            grab_state.options = cursor_options.clone();
        }
        //*awaiting = None;
        *awaiting = Some(false);

        // Release mouse, if captured.
        cursor_options.grab_mode = CursorGrabMode::None;
        cursor_options.visible = true;
    }
}

#[derive(Component, Clone, Copy, PartialEq, Hash, Reflect)]
#[reflect(Component, Clone)]
#[type_path = "game"]
#[non_exhaustive]
pub enum GuiAreaMarker {
    /// The information area of the GUI (smaller font, bottom left corner)
    InfoArea,

    /// The information area of the GUI (smaller font, bottom)
    InstructionsArea,

    /// Where the status of the held item is.
    HandStatusArea,

    /// The game status area of the GUI (large)
    GameStatusArea,

    /// Where the score (if any) is presented (small, upper-right)
    ScoreArea,

    /// Mark the Mute state icon.
    MuteArea,

    /// Mark the User Pause state icon.
    UserPausedArea,

    /// Mark the Pause Scripts state icon.
    ScriptsRunningArea,
    /// Mark the crossed-out icon (on top of [ScriptsRunningArea] and visible only when paused).
    ScriptsRunningCrossArea,

    /// Mark the Physics Running state icon.
    PhysicsRunningArea,
    /// Mark the crossed-out icon (on top of [PhysicsRunningArea] and visible only when physics paused).
    PhysicsRunningCrossArea,

}

#[derive(SystemParam)]
pub struct GuiAreaMarkerLocator<'w, 's> {
    marker_q: Query<'w, 's, (Entity, Read<GuiAreaMarker>)>,
}

impl<'w, 's> GuiAreaMarkerLocator<'w, 's> {
    pub fn find_first_marker(&self, marker: GuiAreaMarker) -> Option<Entity> {
        for (entity, &a_marker) in self.marker_q.iter() {
            if a_marker == marker {
                return Some(entity)
            }
        }
        None
    }
    pub fn with_first<R>(&self, marker: GuiAreaMarker, mut cb: impl FnMut(Entity) -> R) -> Option<R> {
        for (entity, &a_marker) in self.marker_q.iter() {
            if a_marker == marker {
                return Some(cb(entity));
            }
        }
        None
    }
    pub fn with_marker<R>(&self, marker: GuiAreaMarker, mut cb: impl FnMut(Entity) -> R) -> Option<R> {
        let mut ret = None::<R>;
        for (entity, &a_marker) in self.marker_q.iter() {
            if a_marker == marker {
                ret.replace(cb(entity));
            }
        }
        ret
    }
}

fn setup_gui_nodes(
    mut commands: Commands,
    assets: Res<CommonGuiAssets>,
    ui_font: Res<UiFont>,
) {
    // let font = ui_font.map_or(default(), |f| f.0.clone());
    let font: FontSource = ui_font.0.clone().into();
    let icon_size = 32.0;

    let despawn = DespawnOnReset(ProgramState::InGame);

    // Info
    commands.spawn((
        despawn.clone(),
        GuiAreaMarker::InfoArea,
        Text::new(""),
        TextFont {
            font: font.clone(),
            font_size: FontSize::Px(10.0),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(160.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));

    // Instructions
    commands.spawn((
        despawn.clone(),
        GuiAreaMarker::InstructionsArea,
        Visibility::Hidden,
        Text::new(
            "",
        ),
        TextFont {
            font: font.clone(),
            font_size: FontSize::Px(16.0),
            .. default()
        },
        Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            .. default()
        },
    ));

    // Score
    commands.spawn((
        despawn.clone(),
        GuiAreaMarker::ScoreArea,
        Text::default(),
        TextFont {
            font: font.clone(),
            font_size: FontSize::Px(icon_size),
            ..default()
        },
        TextColor(Color::Srgba(tailwind::YELLOW_300)),
        TextShadow {
            offset: Vec2::splat(2.),
            color: Color::linear_rgba(0., 0., 0., 1.0),
        },
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            right: Val::Px(12.0),
            ..default()
        },
    ));

    // Game Status (win/lose)
    commands.spawn((
        despawn.clone(),
        Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            .. default()
        },
        BackgroundColor(Color::NONE),
        RenderLayers::from_layers(&[RENDER_LAYER_UI]),

        children![
            GuiAreaMarker::GameStatusArea,
            Text::new(
                "", // e.g. "You win!"
            ),
            TextFont {
                font: font.clone(),
                font_size: FontSize::Px(64.0),
                .. default()
            },
            TextColor( Color::linear_rgba(0., 0., 0., 1.0)),
            TextShadow {
                offset: Vec2::splat(4.),
                color: Color::linear_rgba(0., 0., 0., 0.5),
            },
            TextLayout::new(Justify::Center, LineBreak::WordBoundary),
        ],
    ));

    // In-hand status
    commands.spawn((
        despawn.clone(),
        GuiAreaMarker::HandStatusArea,
        UiNodeAlpha(0.0),
        Name::new("InHandStatus"),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Percent(5.0),
            right: Val::Percent(50.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::End,
            ..default()
        },
        Visibility::Hidden,
    ))
    ;

    // We place these in from right to left,
    // but in some cases stack them.

    let pos_y = Val::Px(4.0);
    let mut right_x = 4.0 - icon_size;    // can't do add/sub on Val, keep  as f32

    // Mute icon
    right_x += icon_size;

    commands.spawn((
        DespawnOnReset(ProgramState::InGame),
        GuiAreaMarker::MuteArea,
        TextFont {
            font: assets.emoji_icon_font.clone().into(),
            font_size: FontSize::Px(icon_size),
            .. default()
        },
        TextColor(Color::Srgba(tailwind::YELLOW_300)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(4.0),
            right: Val::Px(right_x),
            .. default()
        },
        Text::new("\u{1F508}"), // speaker
        Visibility::Hidden,
    ));

    // User Pause icon, to the left
    right_x += icon_size;
    commands.spawn((
        DespawnOnReset(ProgramState::InGame),
        GuiAreaMarker::UserPausedArea,
        TextFont {
            font: assets.emoji_icon_font.clone().into(),
            font_size: FontSize::Px(icon_size),
            .. default()
        },
        TextColor(Color::Srgba(tailwind::YELLOW_300)),
        Node {
            position_type: PositionType::Absolute,
            top: pos_y,
            right: Val::Px(right_x),
            .. default()
        },
        Text::new("\u{1F6AB}"),  // circle with slash
        Visibility::Inherited,
    ));

    // Script Pause icon, to the left
    right_x += icon_size;
    commands.spawn((
        Name::new("RunningArea"),
        DespawnOnReset(ProgramState::InGame),
        GuiAreaMarker::ScriptsRunningArea,
        TextFont {
            font: assets.emoji_icon_font.clone().into(),
            // font: assets.hack_font.clone().into(),
            font_size: FontSize::Px(icon_size),
            .. default()
        },
        TextColor(Color::Srgba(tailwind::GRAY_100)),
        Node {
            position_type: PositionType::Absolute,
            top: pos_y,
            right: Val::Px(right_x),
            .. default()
        },
        // Text::new("\u{23f1}\u{fe0f}"), // stopwatch
        // ⚡⚠⚒⛭
        // missing ⛯⚗⛕⛈⏻⛹
        Text::new("⛭"),
        Visibility::Hidden, // by default
        ZIndex(-1), // under
    ));
    commands.spawn((
        Name::new("RunningCrossArea"),
        DespawnOnReset(ProgramState::InGame),
        GuiAreaMarker::ScriptsRunningCrossArea,
        TextFont {
            font: assets.emoji_icon_font.clone().into(),
            font_size: FontSize::Px(icon_size),
            .. default()
        },
        TextColor(Color::Srgba(tailwind::RED_700)),
        Node {
            position_type: PositionType::Absolute,
            top: pos_y,
            right: Val::Px(right_x), // on top
            .. default()
        },
        Text::new("\u{1F5D9}"), // cross X
        Visibility::Hidden, // by default
        ZIndex(1),
    ));

    // "Movement Pause" or "Freeze" icon, to the left
    right_x += icon_size;
    commands.spawn((
        DespawnOnReset(ProgramState::InGame),
        GuiAreaMarker::PhysicsRunningArea,
        TextFont {
            font: assets.emoji_icon_font.clone().into(),
            font_size: FontSize::Px(icon_size),
            .. default()
        },
        TextColor(Color::Srgba(tailwind::GRAY_300)),
        Node {
            position_type: PositionType::Absolute,
            top: pos_y,
            right: Val::Px(right_x),
            .. default()
        },
        Text::new("\u{1f680}"), // rocket 🚀
        Visibility::Inherited,
        ZIndex(-1), // under
    ));
    commands.spawn((
        DespawnOnReset(ProgramState::InGame),
        GuiAreaMarker::PhysicsRunningCrossArea,
        TextFont {
            font: assets.emoji_icon_font.clone().into(),
            font_size: FontSize::Px(icon_size),
            .. default()
        },
        TextColor(Color::Srgba(tailwind::RED_700)),
        Node {
            position_type: PositionType::Absolute,
            top: pos_y,
            right: Val::Px(right_x),
            .. default()
        },
        Text::new("\u{1F5D9}"), // cross X
        Visibility::Hidden,
        ZIndex(0),
    ));

}

fn update_pause_ui(
    paused: Res<PauseState>,
    gui_area: GuiAreaMarkerLocator,
    mut vis_q: Query<&mut Visibility>,
) {
    gui_area.with_marker(GuiAreaMarker::UserPausedArea, |ent| {
        if let Ok(mut vis) = vis_q.get_mut(ent) {
            *vis = if paused.is_paused() { Visibility::Inherited } else { Visibility::Hidden };
        }
    });
}

fn update_mute_ui(
    vol_q: Single<&UserVolume, With<MainBus>>,
    gui_area: GuiAreaMarkerLocator,
    mut vis_q: Query<&mut Visibility>,
) {
    gui_area.with_marker(GuiAreaMarker::MuteArea, |ent| {
        if let Ok(mut vis) = vis_q.get_mut(ent) {
            *vis = if vol_q.muted { Visibility::Inherited } else { Visibility::Hidden };
        }
    });
}

#[derive(Resource, Debug, Default, Deref, DerefMut, Reflect)]
#[reflect(Resource, Default)]
pub struct PhysicsPaused(pub bool);

fn update_physics_pause_ui(
    paused: Res<PhysicsPaused>,

    gui_area: GuiAreaMarkerLocator,
    mut vis_q: Query<&mut Visibility>,

    mut time: If<ResMut<Time<Physics>>>,
) {
    gui_area.with_marker(GuiAreaMarker::PhysicsRunningArea, |ent| {
        if let Ok(mut vis) = vis_q.get_mut(ent) {
            // If we're here, we've got physics.
            vis.set_if_neq(Visibility::Inherited);
        }
    });
    if !paused.is_changed() {
        return
    }
    if **paused {
        time.pause();
    } else {
        time.unpause();
    }
    gui_area.with_marker(GuiAreaMarker::PhysicsRunningCrossArea, |ent| {
        if let Ok(mut vis) = vis_q.get_mut(ent) {
            // If we're here, it's paused.
            vis.set_if_neq(if **paused { Visibility::Inherited } else { Visibility::Hidden });
        }
    });
}

/// Set the instruction text for level.
/// It is consumed and displayed in [LevelState::LevelLoaded].
#[derive(Resource, Reflect, Deref)]
#[reflect(Resource)]
#[type_path = "game"]
pub struct InstructionText(pub String);

/// Set when we showed the instruction text for what level.
#[derive(Resource, Reflect)]
#[reflect(Resource)]
#[type_path = "game"]
pub struct ShowedInstructions{ level_id: String }

fn show_instructions(
    mut commands: Commands,
    text: If<Res<InstructionText>>,
    level: Option<Res<CurrentLevel>>,
    showed: Option<Res<ShowedInstructions>>,
    fonts: Res<CommonGuiAssets>,
    gui_area: GuiAreaMarkerLocator,
) {
    use bevy_tweening::AnimTarget;
    use bevy_tweening::EaseMethod;
    use bevy_tweening::Tween;
    use bevy_tweening::TweenAnim;
    use bevy_tweening::lens::TextColorLens;

    // Ignore blank instructions.
    if text.0.is_empty() {
        return
    }

    // Show only if we don't remember showing the instructions for this level.
    let level_id = if let Some(level) = level {
        level.id.clone()
    } else {
        String::new()
    };

    if showed.is_some_and(|s| s.level_id == level_id) {
        return;
    }

    commands.insert_resource(ShowedInstructions{
        level_id,
    });

    let mut text_ent = Entity::PLACEHOLDER;

    gui_area.with_first(GuiAreaMarker::InstructionsArea, |ent| {
        commands.entity(ent)
        .try_insert(Visibility::Inherited)  // show
        .with_children(|builder| {
            text_ent = builder.spawn((
                DespawnOnReset(LevelState::Playing),
                Text::new(text.0.clone()),
                TextLayout::new(Justify::Center, LineBreak::WordBoundary),
                TextFont {
                    font: fonts.std_ui.clone().into(),
                    font_size: FontSize::Px(32.0),
                    .. default()
                },
                TextColor(Color::WHITE.with_alpha(0.5)),
                TextShadow {
                    offset: Vec2::splat(2.),
                    color: Color::linear_rgba(0., 0., 0., 0.0),
                },
            )).id();
        });
    });

    // Fade in and out.

    const TIME_SECS: f32 = 2.0;

    let color_tween = Tween::new(
        EaseMethod::EaseFunction(EaseFunction::CubicOut),
        Duration::from_secs_f32(TIME_SECS),
        TextColorLens {
            start: Color::WHITE.with_alpha(0.0),
            end: Color::WHITE.with_alpha(1.0),
        }
    )
    .with_repeat(2, bevy_tweening::RepeatStrategy::MirroredRepeat);

    let shadow_tween = Tween::new(
        EaseMethod::EaseFunction(EaseFunction::CubicOut),
        Duration::from_secs_f32(TIME_SECS),
        TextShadowColorLens {
            start: Color::linear_rgba(0., 0., 0., 0.0),
            end: Color::linear_rgba(0., 0., 0., 1.0),
        }
    )
    .with_repeat(2, bevy_tweening::RepeatStrategy::MirroredRepeat);

    commands.entity(text_ent).try_insert((
        DespawnOnReset(GameplayState::Playing),
        TweenAnim::new(color_tween).with_destroy_on_completed(true),

        // Add another TweenAnim.
        children![(
            TweenAnim::new(shadow_tween).with_destroy_on_completed(true),
            AnimTarget::component::<TextShadow>(text_ent),
        )]
    ));
}

pub fn hide_instructions(
    gui_area: GuiAreaMarkerLocator,
    mut vis_q: Query<&mut Visibility>,
) {
    gui_area.with_marker(GuiAreaMarker::InstructionsArea, |ent| {
        if let Ok(mut vis) = vis_q.get_mut(ent) {
            *vis = Visibility::Hidden;
        }
    });
}

pub fn reset_instructions(
    mut commands: Commands,
) {
    commands.remove_resource::<InstructionText>();
}
