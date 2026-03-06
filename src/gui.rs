use std::path::PathBuf;

use bevy::asset::AssetPath;
use bevy::camera::visibility::RenderLayers;
use bevy::color::palettes::tailwind;
use bevy::prelude::*;
use bevy::window::CursorGrabMode;
use bevy::window::CursorOptions;
use bevy::window::PrimaryWindow;
use bevy::window::WindowFocused;
use bevy_asset_loader::prelude::*;
use bevy_seedling::prelude::MainBus;

use crate::assets::CommonAssets;
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
        .configure_loading_state(
            LoadingStateConfig::new(ProgramState::Initializing)
                .load_collection::<CommonAssets>()
        )
        .configure_loading_state(
            LoadingStateConfig::new(ProgramState::LoadingSave)
                .load_collection::<CommonAssets>()
        )
        .add_systems(Startup,
            load_ui_font,
        )
        .add_systems(OnEnter(ProgramState::InGame),
            (
                check_gui_state,    // initialize
                ensure_font_assets,
                grab_cursor_for_game,
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
            check_gui_state.run_if(resource_changed::<GuiState>.or(resource_changed::<State<OverlayState>>)),
        )
        .add_systems(
            Update,
            (
                check_grab_focus_state,
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

/// Define to define the asset path to the UI font, to be loaded in Startup.
/// Use this OR UiFont.
#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct UiFontPath(pub PathBuf);

/// Define to define the font for UI. Overrides [UiFontPath]. Defined after Startup.
#[derive(Resource, Reflect)]
#[reflect(Resource)]
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
    world.init_collection::<CommonAssets>();
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
#[derive(Message)]
pub struct GrabCursor(pub bool);

/// Tells whether we're in a mode where the status area is displayed.
#[derive(Resource, Clone, PartialEq)]
pub struct StatusVisible(pub bool);

#[derive(Resource, Clone, PartialEq, Reflect)]
#[reflect(Resource)]
#[type_path = "game"]
pub struct GuiState {
    pub show_status: bool,
    pub show_fps: bool,
    pub show_inspector: bool,
    pub show_inspector_always: bool,
}

impl Default for GuiState {
    fn default() -> Self {
        Self {
            show_status: false,
            show_fps: false,
            show_inspector: true,
            show_inspector_always: false,
        }
    }
}

#[derive(Resource)]
pub struct GrabState{ was_grabbed: bool, options: CursorOptions }

fn check_gui_state(
    state: Res<GuiState>,
    fps: Option<ResMut<bevy::dev_tools::fps_overlay::FpsOverlayConfig>>,
    mut status_visible: ResMut<StatusVisible>,
    overlay: Res<State<OverlayState>>,
) {
    if let Some(mut fps) = fps {
        fps.enabled = state.show_fps || overlay.is_debug();
    }
    status_visible.0 = state.show_status;
}

fn grab_cursor_for_game(
    mut commands: Commands,
) {
    commands.write_message(GrabCursor(true));
}

fn ungrab_cursor_for_overlay(
    mut commands: Commands,
) {
    commands.write_message(GrabCursor(false));
}

fn check_grab_focus_state(
    mut grab: MessageReader<GrabCursor>,
    mut focused: MessageReader<WindowFocused>,
    overlay_state: Res<State<OverlayState>>,
    mut grab_state: ResMut<GrabState>,
    mut cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let mut desired_grab: Option<bool> = None;

    for event in focused.read() {
        if !event.focused {
            desired_grab = Some(false);
        } else {
            desired_grab = Some(*overlay_state.get() == OverlayState::Hidden);
        }
    }

    for event in grab.read() {
        desired_grab = Some(event.0);
    }

    if let Some(grab) = desired_grab {
        if grab {
            cursor_options.grab_mode = GRABBED_MODE;
            cursor_options.visible = false;

            grab_state.was_grabbed = true;
        } else {
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
    assets: Res<CommonAssets>,
    ui_font: Option<Res<UiFont>>,
) {
    let font = ui_font.map_or(default(), |f| f.0.clone());

    // Info
    commands.spawn((
        DespawnOnExit(ProgramState::InGame),
        InfoArea,
        Text::new(""),
        TextFont {
            font: font.clone(),
            font_size: 10.0,
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(128.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));

    // Instructions
    commands.spawn((
        DespawnOnExit(ProgramState::InGame),
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
        DespawnOnExit(ProgramState::InGame),
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
        DespawnOnExit(ProgramState::InGame),
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

    // Pause icon in upper right
    commands.spawn((
        DespawnOnExit(ProgramState::InGame),
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
        DespawnOnExit(ProgramState::InGame),
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
