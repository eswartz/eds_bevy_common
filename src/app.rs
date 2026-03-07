use std::time::Duration;

use avian3d::prelude::Physics;
use avian3d::prelude::PhysicsTime as _;
use bevy::camera::visibility::RenderLayers;
use bevy::color::palettes::tailwind;
use bevy::ecs::message::MessageUpdateSystems;
use bevy::prelude::*;
use bevy_asset_loader::loading_state::LoadingState;
use bevy_asset_loader::loading_state::LoadingStateAppExt as _;

use crate::*;

/// This provides common app-level handling.
/// It registers the main states used in this crate
/// (`ProgramState`, `GameplayState`, `OverlayState`)
/// and handles app exit.
pub struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        app
        .insert_state(ProgramState::default())
        .insert_state(GameplayState::default())
        .insert_state(OverlayState::default())

        .init_state::<ProgramState>()
        .init_state::<GameplayState>()
        .init_state::<LevelState>()
        //////

        // This needs to be added at the app level, though
        // you can configure it later.
        .add_loading_state(
            LoadingState::new(ProgramState::Initializing)
                .continue_to_state(ProgramState::New)
                .on_failure_continue_to_state(ProgramState::Error)
            ,
        )

        // Custom exit handling.
        .add_systems(
            First,
            (
                check_app_exit.in_set(MessageUpdateSystems),
                check_windows_closed.in_set(MessageUpdateSystems),
            )
                .chain(),
        )

        .add_systems(OnEnter(ProgramState::Initializing), on_enter_initializing)
        .add_systems(
            OnEnter(ProgramState::New),
            (on_enter_loading, init_perf_ui.run_if(show_dev_tools)).chain(),
        )
        .add_systems(
            OnEnter(ProgramState::LaunchMenu),
            (on_enter_launch_menu,).chain(),
        )
        .add_systems(
            OnEnter(ProgramState::InGame),
            (on_exit_launch_menu.run_if(is_in_menu),
                on_enter_in_game).chain(),
        )
        .add_systems(
            OnEnter(GameplayState::Playing),
            show_3d_camera,
        )
        .add_systems(
            OnExit(GameplayState::Playing),
            hide_3d_camera,
        )

        .add_systems(OnEnter(ProgramState::Error),
            on_enter_error)
        .add_systems(OnEnter(OverlayState::ErrorScreen),
            on_error_screen)
        .add_systems(OnExit(OverlayState::ErrorScreen),
            on_error_screen_finished)

        .insert_resource(VideoSettings::default())

        .insert_resource(ProductName("My Game".to_string()))
        .insert_resource(PauseState::new(false))

        ;

    }
}

/// This is registered to initiate a shutdown. It is added by
/// stock menu actions and may otherwise be added
/// The process may take a few frames (e.g. waiting on network).
#[derive(Debug, Resource)]
pub struct ExitRequest;


pub fn check_app_exit(
    mut commands: Commands,
    exit: Option<Res<ExitRequest>>,
    mut app_exit: MessageWriter<AppExit>,
) {
    if exit.is_none() {
        return;
    }

    commands.remove_resource::<ExitRequest>();
    app_exit.write(AppExit::Success);
}

// It seems WindowClosed, WindowClosing, WindowDestroyed events don't make it for the primary window...?
pub fn check_windows_closed(windows: Query<&Window>, mut commands: Commands) {
    if windows.is_empty() {
        commands.insert_resource(ExitRequest);
    }
}

pub fn on_enter_initializing(mut commands: Commands, camera_q: Query<&Camera, With<Camera2d>>) {
    if camera_q.single().is_err() {
        commands.spawn((
            Camera2d,
            Camera {
                // Render before 3D.
                order: -1,
                clear_color: ClearColorConfig::Default,
                ..default()
            },
            RenderLayers::from_layers(&[RENDER_LAYER_UI]),
        ));
    }
}

pub fn on_enter_loading(mut commands: Commands) {
    commands.set_state(ProgramState::LaunchMenu);
}

pub fn on_enter_launch_menu(mut commands: Commands) {
    commands.set_state(OverlayState::MainMenu);
}

pub fn on_exit_launch_menu(mut commands: Commands) {
    commands.set_state(OverlayState::Hidden);
}

pub fn on_enter_in_game(mut time: ResMut<Time<Physics>>) {
    time.unpause();
}

pub fn init_perf_ui(mut commands: Commands) {
}


#[derive(Component)]
pub struct ErrorScreen;

pub fn on_enter_error(
    mut commands: Commands,
) {
    commands.set_state(OverlayState::ErrorScreen);
}

pub fn on_error_screen(
    mut commands: Commands,
    ui_font: Option<Res<UiFont>>,
) {
    let ent_commands = commands.spawn((
        Name::new("Loading..."),
        ErrorScreen,
    ));
    setup_error_screen(ent_commands, ui_font);
}

pub fn on_error_screen_finished(
    mut commands: Commands,
    gui_q: Query<Entity, With<ErrorScreen>>,
) {
    for ent in gui_q.iter() {
        commands.entity(ent).try_despawn();
    }
}

pub fn setup_error_screen(
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
        BackgroundColor(tailwind::RED_800.with_alpha(0.75).into()),
        RenderLayers::from_layers(&[RENDER_LAYER_UI]),
    ))
    .with_children(|builder| {
        builder.spawn((
            Text::new(
                "There is an installation error (assets are missing).\nPlease gather stdout and stderr and report.",
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
