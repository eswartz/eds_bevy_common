use std::any::Any;
use std::any::TypeId;
use std::path::PathBuf;

use avian3d::prelude::PhysicsGizmos;
use bevy::asset::AssetPath;
use bevy::camera::visibility::RenderLayers;
use bevy::color::palettes::tailwind;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::reflect::Typed;
use bevy::window::CursorGrabMode;
use bevy::window::CursorOptions;
use bevy::window::PrimaryWindow;
use bevy::window::WindowFocused;
use bevy_asset_loader::prelude::*;
use bevy_seedling::prelude::MainBus;

use crate::DespawnOnExitOrReenter;
use crate::StatsOverlayVisible;
use crate::assets::CommonGuiAssets;
use crate::RENDER_LAYER_UI;

use super::audio::UserVolume;
use super::lifecycle::PauseState;
use super::states_sets::OverlayState;
use super::states_sets::ProgramState;

pub struct GuiPlugin;

impl Plugin for GuiPlugin {
    fn build(&self, app: &mut App) {
        app
        .insert_resource(GuiState::default())
        .insert_resource(StatusVisible(false))
        .init_resource::<GrabState>()
        .add_message::<GrabCursor>()
        .add_systems(Startup,
            load_ui_font,
        )
        .add_systems(
            Update,
            (
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

        .add_systems(
            Update,
            check_grab_focus_state.run_if(in_state(ProgramState::InGame))
        )

        .add_systems(
            Update,
            update_gui_state.run_if(resource_changed::<GuiState>),
        )
        .add_systems(
            Update,
            (
                update_pause_ui,
                update_mute_ui,
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
pub struct UiNodeAlpha(pub f32);

impl Default for UiNodeAlpha {
    fn default() -> Self {
        Self(1.0)
    }
}

/// Computed alpha from parents and self.
#[derive(Component, Reflect)]
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
                commands.entity(kid).insert(UiNodeAlpha(1.0));
            }
        });

        if let Ok(mut comp) = comp_alpha_q.get_mut(ent) {
            // Only change the alpha.
            if comp.alpha != full_alpha {
                comp.alpha = full_alpha;

                commands.entity(ent).insert((
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
            commands.entity(ent).insert((
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

fn load_ui_font(
    mut commands: Commands,
    assets: Res<AssetServer>,
    ui_font_path: Option<Res<UiFontPath>>,
    ui_font: Option<Res<UiFont>>,
) {
    if let Some(path) = ui_font_path && ui_font.is_none() {
        commands.insert_resource(UiFont(assets.load(AssetPath::from_path(&path.0))));
    }
}

fn ensure_font_assets(
    world: &mut World,
) {
    world.init_collection::<CommonGuiAssets>();
    // if world.get_resource::<UiFont>().is_none() {
    //     let default_ui_font = world.get_resource::<CommonAssets>().unwrap().recursive_bold_font.clone();
    //     world.insert_resource(UiFont(default_ui_font));
    // }
}

#[derive(Component)]
pub struct LoadingScreen;

pub fn on_loading(
    mut commands: Commands,
    ui_font: Option<Res<UiFont>>,
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
    ui_font: Option<Res<UiFont>>,
) -> Entity {
    ent_commands.insert((
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
                font: ui_font.map_or(default(), |f| f.0.clone()),
                font_size: 32.0,
                .. default()
            },
            TextColor(Color::WHITE.with_alpha(0.5)),
        ));
    })
    .id()
}

impl Default for GrabState {
    fn default() -> Self {
        Self {
            was_grabbed: false,
            options: CursorOptions{
                visible: false,
                grab_mode: GRABBED_MODE,
                .. default()
            }
        }
    }
}

const GRABBED_MODE: CursorGrabMode = CursorGrabMode::Locked;

/// Indicate the desire to change the cursor grab state (false = not grabbed).
#[derive(Message, Debug)]
pub struct GrabCursor(pub bool);

/// Tells whether we're in a mode where the [GameStatusArea] is displayed.
#[derive(Resource, Debug, Clone, PartialEq)]
pub(crate) struct StatusVisible(pub bool);

/// Flags
#[derive(Resource, Debug, Clone, PartialEq, Reflect)]
#[reflect(Resource)]
#[type_path = "game"]
pub struct GuiState {
    pub enabled: bool,
    pub show_status: bool,
    pub show_fps: bool,
    pub show_inspector: bool,
    pub show_inspector_always: bool,
    pub show_physics_gizmos: bool,
}

impl Default for GuiState {
    fn default() -> Self {
        Self {
            enabled: false,
            show_status: false,
            show_fps: false,
            show_inspector: true,
            show_inspector_always: false,
            show_physics_gizmos: false,
        }
    }
}

pub fn is_debug_ui_enabled(gui_state: Option<Res<GuiState>>) -> bool {
    gui_state.is_some_and(|g| g.enabled)
}

pub fn is_debug_ui_inspector_active(
    gui_state: Option<Res<GuiState>>,
    ovl_state: Option<Res<State<OverlayState>>>,
) -> bool {
    gui_state.is_some_and(|gui_state| {
        gui_state.enabled && (
            gui_state.show_inspector_always ||
            (gui_state.show_inspector && ovl_state.is_none_or(|ovl_state|
                !ovl_state.is_menu() || *ovl_state == OverlayState::ControlsMenu // allow testing
            ))
        )
    })
}

#[derive(Resource)]
pub struct GrabState{ was_grabbed: bool, options: CursorOptions }

fn update_gui_state(
    state: Res<GuiState>,
    fps_visible: Option<ResMut<StatsOverlayVisible>>,
    mut status_visible: ResMut<StatusVisible>,
    mut gizmo_config: ResMut<GizmoConfigStore>,
) {
    if let Some(mut fps_visible) = fps_visible {
        fps_visible.0 = state.show_fps || state.enabled;
    }
    status_visible.0 = state.show_status;

    gizmo_config.config_mut::<PhysicsGizmos>().0.enabled = state.show_physics_gizmos;
}

fn grab_cursor_for_game(
    mut commands: Commands,
    gui_state: Res<GuiState>,
) {
    commands.write_message(GrabCursor(!gui_state.enabled));
}

fn ungrab_cursor_for_overlay(
    mut commands: Commands,
    gui_state: Res<GuiState>,
) {
    commands.write_message(GrabCursor(gui_state.enabled));
}

fn check_grab_focus_state(
    mut grab: MessageReader<GrabCursor>,
    mut focused: MessageReader<WindowFocused>,
    overlay_state: Res<State<OverlayState>>,
    gui_state: ResMut<GuiState>,
    mut grab_state: ResMut<GrabState>,
    mut cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,

    mut awaiting: Local<Option<bool>>,
) {
    let mut desired_grab: Option<bool> = None;

    if let Some(event) = focused.read().last() {
        if !event.focused {
            desired_grab = Some(false);
        } else {
            desired_grab = Some((**overlay_state).is_hidden() && !gui_state.enabled);
        }
    }

    if let Some(event) = grab.read().last() {
        desired_grab = Some(event.0);
    }

    if desired_grab.is_none() && awaiting.is_some() {
        if cursor_options.grab_mode == CursorGrabMode::None {
            desired_grab = Some(true);
        } else {
            *awaiting = None;
        }
    }

    if let Some(grab) = desired_grab {
        if grab {
            *awaiting = Some(true);
            cursor_options.grab_mode = GRABBED_MODE;
            cursor_options.visible = false;

            grab_state.was_grabbed = true;
        } else {
            *awaiting = None;
            if grab_state.was_grabbed {
                grab_state.was_grabbed = false;
                grab_state.options = cursor_options.clone();
            }

            // Release mouse, if captured
            cursor_options.grab_mode = CursorGrabMode::None;
            cursor_options.visible = true;
        }
    }
}

/// The information area of the GUI (smaller font, bottom left corner)
#[derive(Component)]
pub struct InfoArea;

/// The information area of the GUI (smaller font, bottom)
#[derive(Component)]
pub struct InstructionsArea;

/// Where the status of the held item is.
#[derive(Component)]
pub struct HandStatusArea;

/// The game status area of the GUI (large)
#[derive(Component)]
pub struct GameStatusArea;

/// Where the score (if any) is presented (small, upper-right)
#[derive(Component)]
pub struct ScoreArea;

/// Mark the Pause state icon.
#[derive(Component)]
struct PauseArea;

/// Mark the Mute state icon.
#[derive(Component)]
struct MuteArea;

fn setup_gui_nodes(
    mut commands: Commands,
    assets: Res<CommonGuiAssets>,
    ui_font: Option<Res<UiFont>>,
) {
    let font = ui_font.map_or(default(), |f| f.0.clone());

    // Info
    commands.spawn((
        DespawnOnExitOrReenter(ProgramState::InGame),
        InfoArea,
        Text::new(""),
        TextFont {
            font: font.clone(),
            font_size: 10.0,
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
        DespawnOnExitOrReenter(ProgramState::InGame),
        InstructionsArea,
        Visibility::Hidden,
        Text::new(
            "",
        ),
        TextFont {
            font: font.clone(),
            font_size: 16.0,
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
        DespawnOnExitOrReenter(ProgramState::InGame),
        ScoreArea,
        Text::default(),
        TextFont {
            font: font.clone(),
            font_size: 32.0,
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
        DespawnOnExitOrReenter(ProgramState::InGame),
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
    ))
    .with_children(|builder| {
        builder.spawn((
            GameStatusArea,
            Text::new(
                "",
            ),
            TextFont {
                font: font.clone(),
                font_size: 64.0,
                .. default()
            },
            TextColor( Color::linear_rgba(0., 0., 0., 1.0)),
            TextShadow {
                offset: Vec2::splat(4.),
                color: Color::linear_rgba(0., 0., 0., 0.5),
            },
            TextLayout::new(Justify::Center, LineBreak::WordBoundary),
        ));
    })
    ;

    // In-hand status
    commands.spawn((
        DespawnOnExitOrReenter(ProgramState::InGame),
        HandStatusArea,
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

    // Pause icon in upper right
    commands.spawn((
        DespawnOnExitOrReenter(ProgramState::InGame),
        PauseArea,
        Visibility::Visible,
        TextFont {
            font: assets.emoji_icon_font.clone(),
            font_size: 32.0,
            .. default()
        },
        TextColor(Color::Srgba(tailwind::YELLOW_300)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(4.0),
            right: Val::Px(4.0),
            .. default()
        },
        Text::new(""),
    ));

    // Mute icon in upper right
    commands.spawn((
        DespawnOnExitOrReenter(ProgramState::InGame),
        MuteArea,
        Visibility::Visible,
        TextFont {
            font: assets.emoji_icon_font.clone(),
            font_size: 32.0,
            .. default()
        },
        TextColor(Color::Srgba(tailwind::YELLOW_300)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(4.0),
            right: Val::Px(36.0),
            .. default()
        },
        Text::new(""),
    ));

}

fn update_pause_ui(
    paused: Res<PauseState>,
    mut text_q: Query<&mut Text, With<PauseArea>>,
) {
    if let Ok(mut text) = text_q.single_mut() {
        // One icon for any pause reason.
        let new_text = if paused.is_paused() { "\u{1F6AB}" } else { " " };
        if new_text != text.0 {
            text.0 = new_text.to_string();
        }
    }
}

fn update_mute_ui(
    vol_q: Single<&UserVolume, With<MainBus>>,
    mut text_q: Query<&mut Text, With<MuteArea>>,
) {
    if let Ok(mut text) = text_q.single_mut() {
        let new_text = if vol_q.muted { "\u{1F508}" } else { " " };
        if new_text != text.0 {
            text.0 = new_text.to_string();
        }
    }
}

pub fn hide_instructions(
    mut inst_q: Query<&mut Visibility, With<InstructionsArea>>,
) {
    for mut vis in inst_q.iter_mut() {
        *vis = Visibility::Hidden;
    }
}
