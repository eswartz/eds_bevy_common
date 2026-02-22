
use avian3d::PhysicsPlugins;
use avian3d::prelude::{CollidingEntities, Physics, PhysicsSystems};
use bevy::asset::uuid::Uuid;
use bevy::color::palettes::tailwind;
use bevy::dev_tools::fps_overlay::FpsOverlayPlugin;
use bevy::prelude::*;
use bevy::camera::visibility::RenderLayers;
use bevy::ecs::world::CommandQueue;
use bevy::scene::SceneInstanceReady;
use bevy::sprite::Text2dShadow;
use bevy_seedling::spatial::SpatialListener3D;
use bevy_skein::SkeinPlugin;
use bevy_tweening::lens::{TextColorLens, TransformPositionLens};
use bevy_tweening::{AnimTarget, EaseMethod, Tween, TweenAnim};
use eds_bevy_common::*;
use leafwing_input_manager::prelude::{ActionState, InputMap};
use strum::VariantArray;

use std::time::Duration;

use bevy::winit::WinitSettings;

fn main() -> AppExit {
    let mut app = App::new();
    app
        .insert_resource(WinitSettings {
            focused_mode: bevy::winit::UpdateMode::reactive_low_power(Duration::from_secs_f32(
                1.0 / 120.0,
            )),
            unfocused_mode: bevy::winit::UpdateMode::reactive_low_power(Duration::from_secs_f32(
                1.0 / 24.0,
            )),
        })

        .add_plugins((
            DefaultPlugins,
            PhysicsPlugins::default(),
        ))
        // .add_plugins(avian3d::debug_render::PhysicsDebugPlugin::default())

        .add_plugins(SkeinPlugin::default())

        .add_plugins(AppPlugin)

        .add_plugins(ActionPlugin)
        .insert_resource(create_input_map())

        .add_plugins(LifecyclePlugin)
        .add_plugins(GuiPlugin)
        .add_plugins(WorldUiPlugin)
        .add_plugins(WorldStatePlugin)
        .add_plugins(AudioCommonPlugin)
        .add_plugins(EffectsPlugin)
        .add_plugins(LevelsPlugin)

        .add_plugins(PlayerCameraPlugin)
        .add_plugins(PlayerInputPlugin)
        .add_plugins(PlayerClientPlugin)
        .add_plugins(PlayerMovementPlugin)
        .add_plugins(PlayerControllerPlugin)

        // req'd by GuiPlugin
        .insert_resource(UiFontPath(std::path::Path::new("fonts/Hack-Regular.ttf").to_path_buf()))

        // req'd by player plugins
        .insert_resource(OurUser(default()))
        .insert_resource(PlayerMode::Fps)
        .insert_resource(PlayerInputSettings::for_fps())

        .add_plugins(MyMenuPlugin)
        .init_resource::<LevelDifficulty>()

        .add_plugins(MyGamePlugin)

        .add_systems(Startup, (
            register_dummy_level,
        ))
        .add_systems(
            OnEnter(GameplayState::Playing),
            ensure_3d_camera,
        )

        .add_systems(
            FixedUpdate,
            (
                check_actions,
            )
                .run_if(not(is_in_menu))
                .run_if(is_level_active)
                .run_if(not(is_paused))
                .run_if(not(debug_gui_wants_input))
                .run_if(in_state(ProgramState::InGame))
            ,
        )
        .add_systems(
            FixedUpdate,
            (
                check_player_out_of_bounds,
            )
            .before(TransformSystems::Propagate)
            .after(PhysicsSystems::Writeback)
            .run_if(not(is_user_paused))
            .run_if(in_state(LevelState::Playing))
            .run_if(in_state(ProgramState::InGame)),
        )
    ;

    if show_dev_tools() {
        app
            .add_plugins(DebugPlugin)

            .add_systems(
                First,
                (
                    bevy::dev_tools::states::log_transitions::<ProgramState>,
                    bevy::dev_tools::states::log_transitions::<GameplayState>,
                    bevy::dev_tools::states::log_transitions::<OverlayState>,
                    bevy::dev_tools::states::log_transitions::<LevelState>,
                ),
            )
            .add_plugins(FpsOverlayPlugin::default())
        ;
    }

    app.run()
}

fn create_input_map() -> InputMap::<UserAction> {
    let mut map = InputMap::default();
    map.merge(&stock_input_maps::default_gui_input_map());
    map.merge(&stock_input_maps::default_wasd_input_map());
    map
}


fn check_actions(
    actions: Res<ActionState<UserAction>>,
    mut commands: Commands,
) {

    if actions.just_released(&UserAction::ForceLose) {
        commands.set_state(LevelState::Lost);
    }
    if actions.just_released(&UserAction::ForceWin) {
        commands.set_state(LevelState::Won);
    }
}

fn register_dummy_level(
    assets: Res<AssetServer>,
    mut list: ResMut<LevelList>) {
    list.0.push(LevelInfo {
        id: "test0".to_string(),
        label: "Level 0 (Intro)".to_string(),
        scene: assets.load("maps/empty.glb#Scene0"),
    });
    list.0.push(LevelInfo {
        id: "test1".to_string(),
        label: "Level 1 (Etc)".to_string(),
        scene: assets.load("maps/empty.glb#Scene0"),
    });
}

///////////////////////////

/// Make sure Entities with Camera3d + WorldCamera and ViewCamera exist,
/// reusing but reconfiguring any existing entities.
pub(crate) fn ensure_3d_camera(
    mut commands: Commands,
    camera_q: Query<Entity, (With<Camera3d>, Without<WorldCamera>, Without<ViewerCamera>)>,
    world_camera_q: Query<Entity, (With<Camera3d>, With<WorldCamera>)>,
) {
    let ent = if let Ok(ent) = world_camera_q.single() {
        // Got one.
        ent
    } else if let Some(ent) = camera_q.iter().next() {
        // Reuse.
        ent
    } else {
        info!("Creating 3D camera");
        commands.spawn_empty().id()
    };

    configure_world_camera(commands.get_entity(ent).unwrap());

    // Force init.
    commands.insert_resource(VideoCameraSettingsChanged);
    commands.insert_resource(VideoEffectSettingsChanged);
}

fn configure_world_camera(mut ent_commands: EntityCommands) {
    ent_commands.insert((
        DespawnOnExit(GameplayState::Playing),
        (
            Name::new("WorldCamera"),
            WorldCamera,
            ViewerCamera,
            Camera3d::default(),
            RenderLayers::layer(RENDER_LAYER_DEFAULT),
            Camera {
                order: 1,
                clear_color: Color::BLACK.into(),
                ..default()
            },
            PlayerCamera(CameraMode::FirstPerson),
            OurCamera::default(),
        ),

        // Audio is from the perspective of the camera.
        SpatialListener3D::default(),
    ));
}

///////////////////////////

pub struct MyMenuPlugin;
impl Plugin for MyMenuPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(MenuCommonPlugin)
            .add_systems(OnEnter(OverlayState::MainMenu), on_enter_main_menu)
            .add_systems(OnEnter(OverlayState::EscapeMenu), on_enter_escape_menu)
            .add_systems(OnExit(OverlayState::EscapeMenu), on_exit_escape_menu)
            .add_systems(OnEnter(OverlayState::GameMenu), on_enter_game_menu)
            .add_systems(OnEnter(OverlayState::OptionsMenu), on_enter_options_menu)
            .add_systems(OnEnter(OverlayState::AudioMenu), on_enter_audio_menu)
            .add_systems(OnEnter(OverlayState::VideoMenu), on_enter_video_menu)
            .add_systems(OnEnter(OverlayState::ControlsMenu), on_enter_controls_menu);
    }
}

#[derive(Debug)]
pub(crate) enum SimpleMenuActions {
    PlayGame,
    GameMenu,
    OptionsMenu,
    AudioMenu,
    VideoMenu,
    ControlsMenu,
    Quit,
    Back,
    ResumeGame,
    StopGame,
}

impl MenuItemHandler for SimpleMenuActions {
    fn handle(&mut self, world: &mut World, message: &MenuActionMessage) {
        // Fetch the paused resource into a local copy to avoid double mutable borrows.
        let mut paused_copy = world
            .get_resource::<PauseState>()
            .cloned()
            .unwrap_or_default();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, world);

        match message {
            MenuActionMessage::Navigate(_) => (),
            MenuActionMessage::Activate(_) | MenuActionMessage::Next(_) => match self {
                SimpleMenuActions::Back => {
                    commands.insert_resource(GoBackInMenuRequest);
                }
                SimpleMenuActions::PlayGame => {
                    // Do not modify current_level LevelIndex, etc. here, but in client.
                    start_game(commands.reborrow());
                }
                SimpleMenuActions::GameMenu => {
                    commands.insert_resource(GoIntoMenuRequest(OverlayState::GameMenu));
                }
                SimpleMenuActions::OptionsMenu => {
                    commands.insert_resource(GoIntoMenuRequest(OverlayState::OptionsMenu));
                }
                SimpleMenuActions::AudioMenu => {
                    commands.insert_resource(GoIntoMenuRequest(OverlayState::AudioMenu));
                }
                SimpleMenuActions::VideoMenu => {
                    commands.insert_resource(GoIntoMenuRequest(OverlayState::VideoMenu));
                }
                SimpleMenuActions::ControlsMenu => {
                    commands.insert_resource(GoIntoMenuRequest(OverlayState::ControlsMenu));
                }
                SimpleMenuActions::Quit => {
                    commands.insert_resource(ExitRequest);
                }
                SimpleMenuActions::ResumeGame => {
                    paused_copy.set_menu_paused(false);
                    commands.insert_resource(paused_copy);

                    commands.set_state(OverlayState::Hidden);
                }
                SimpleMenuActions::StopGame => {
                    paused_copy.set_menu_paused(false);
                    paused_copy.set_user_paused(false);
                    commands.insert_resource(paused_copy);

                    commands.set_state(ProgramState::LaunchMenu);
                    commands.set_state(GameplayState::New);
                }
            },
            MenuActionMessage::Reset(_) => (),
            MenuActionMessage::Previous(_) => (),
            MenuActionMessage::Slide(..) => (),
        }
        queue.apply(world);
    }
}

fn on_enter_main_menu(
    mut commands: Commands,
    font: Res<UiFont>,
    mut history: ResMut<MenuItemSelectionHistory>,
    // mut glyph_mats: ResMut<Assets<TitleShader>>,
    product_name: Res<ProductName>,
    current_level: Option<Res<CurrentLevel>>,
) {
    // Re-initialize state (on entry and on game exit).

    // Do not clear CurrentLevel. `Play` goes there and acts as Reset...

    commands.spawn((
        DespawnOnExit(OverlayState::MainMenu),
        Text2d::new(&product_name.0),
        TextFont {
            font_size: 128.0,
            font: font.0.clone(),
            ..default()
        },
        Text2dShadow {
            offset: Vec2::new(8.0, -8.0),
            color: Color::BLACK,
            ..default()
        },
        // bevy_pretty_text::prelude::Typewriter::new(30.),
        // bevy_pretty_text::prelude::Breathe {
        //     min: 0.975,
        //     max: 1.025,
        //     ..default()
        // },
        // PrettyTextMaterial(glyph_mats.add(TitleShader::default())),
        RenderLayers::layer(RENDER_LAYER_UI),
        Transform::from_xyz(0., 300.0, 0.),
    ));

    MenuItemBuilder::new(
        commands,
        OverlayState::MainMenu,
        ProgramState::LaunchMenu,
        font.0.clone(),
        1.0,
        &history,
    )
    .add_item(
        if let Some(level) = current_level {
            format!("Reset ({})", level.label)
        } else {
            "Play".to_string()
        },
        (), SimpleMenuActions::PlayGame)
    .add_item("Game", (), SimpleMenuActions::GameMenu)
    .add_item("Options", (), SimpleMenuActions::OptionsMenu)
    .add_item("Quit", (), SimpleMenuActions::Quit)
    .finish(&mut history);
}

fn on_enter_game_menu(
    font: Res<UiFont>,
    program_state: Res<State<ProgramState>>,
    mut commands: Commands,
    mut history: ResMut<MenuItemSelectionHistory>,
    level_list: Res<LevelList>,
) {
    macro_rules! make_res_enum_getter_setter {
        ($getter:ident $setter:ident => $enum:ident $res:ident $field:tt) => {
            let $getter = commands.register_system(IntoSystem::into_system(
                |In(entity): In<Entity>, mut enum_q: Query<&mut MenuEnum>, res: Res<$res>| {
                    enum_q.get_mut(entity).unwrap().current = Some(
                        $enum::VARIANTS
                            .iter()
                            .position(|e| *e == res.$field)
                            .unwrap(),
                    );
                },
            ));
            let $setter = commands.register_system(IntoSystem::into_system(
                |In(v): In<usize>, mut res: ResMut<$res>| {
                    res.$field = $enum::VARIANTS[v];
                },
            ));
        };
    }

    make_res_enum_getter_setter!(get_difficulty set_difficulty => Difficulty LevelDifficulty 0);

    fn get_level(In(entity): In<Entity>, mut enum_q: Query<&mut MenuEnum>, next_level_index: Option<Res<LevelIndex>>) {
        let index = next_level_index.map_or(0, |nli| nli.0);
        enum_q.get_mut(entity).unwrap().current = Some(index);
    }
    fn set_level(In(v): In<usize>, mut commands: Commands) {
        commands.insert_resource(LevelIndex(v));
    }
    let get_level = commands.register_system(IntoSystem::into_system(get_level));
    let set_level = commands.register_system(IntoSystem::into_system(set_level));

    let level_infos = &level_list.0;
    let level_count = level_infos.len();
    let level_names = level_infos.iter().map(|info| info.label.clone()).collect::<Vec<_>>();

    MenuItemBuilder::new(
        commands,
        OverlayState::GameMenu,
        *program_state.get(),
        font.0.clone(),
        1.0,
        &history,
    )
    .add_item(
        "Level",
        MenuEnum::new(
            get_level,
            set_level,
            move || level_count,
            move |index| level_names[index].clone(),
        ),
        EnumMenuActions::SelectStartLevelEnum,
    )
    .add_item(
        "Difficulty",
        MenuEnum::new(
            get_difficulty,
            set_difficulty,
            || Difficulty::VARIANTS.len(),
            |index| Difficulty::VARIANTS[index].to_string(),
        ),
        EnumMenuActions::DifficultyEnum,
    )
    .add_item("Back", (), SimpleMenuActions::Back)
    .finish(&mut history);
}

fn on_enter_options_menu(
    font: Res<UiFont>,
    commands: Commands,
    program_state: Res<State<ProgramState>>,
    mut history: ResMut<MenuItemSelectionHistory>,
) {
    MenuItemBuilder::new(
        commands,
        OverlayState::OptionsMenu,
        *program_state.get(),
        font.0.clone(),
        1.0,
        &history,
    )
    .add_item("Audio", (), SimpleMenuActions::AudioMenu)
    .add_item("Video", (), SimpleMenuActions::VideoMenu)
    .add_item("Controls", (), SimpleMenuActions::ControlsMenu)
    .add_item("Back", (), SimpleMenuActions::Back)
    .finish(&mut history);
}

fn on_enter_escape_menu(
    font: Res<UiFont>,
    commands: Commands,
    mut history: ResMut<MenuItemSelectionHistory>,
    current_level: Res<CurrentLevel>,
    mut paused: ResMut<PauseState>,
) {
    // The menu sets [paused()] to true on first entry
    // by setting one of the OR inputs to that method.
    paused.set_menu_paused(true);
    MenuItemBuilder::new(
        commands,
        OverlayState::EscapeMenu,
        ProgramState::InGame,
        font.0.clone(),
        1.0,
        &history,
    )
    // (), SimpleMenuActions::ResumeGame)
    .add_item("Audio", (), SimpleMenuActions::AudioMenu)
    .add_item("Video", (), SimpleMenuActions::VideoMenu)
    .add_item("Controls", (), SimpleMenuActions::ControlsMenu)
    .add_item("Stop", (), SimpleMenuActions::StopGame)
    .add_item(format!("Resume ({})", current_level.label), (), SimpleMenuActions::ResumeGame)
    .finish(&mut history);
}

fn on_exit_escape_menu(mut pause: ResMut<PauseState>) {
    // Unpause if the menu paused.
    // (Has no effect on user pause (key event) which also counts as a pause)
    pause.set_menu_paused(false);
}

#[derive(Debug, Clone)]
pub(crate) enum SliderMenuActions {
    FovSlider,
    MoveSensitivityXSlider,
    MoveSensitivityYSlider,
    MoveSensitivityZSlider,
    TurnSensitivityXSlider,
    TurnSensitivityYSlider,
    TurnSensitivityZSlider,
}

impl MenuItemHandler for SliderMenuActions {}

#[derive(Debug, Clone)]
pub(crate) enum EnumMenuActions {
    DifficultyEnum,
    SelectStartLevelEnum,
    AntialiasingEnum,
    TextureQualityEnum,
}

impl MenuItemHandler for EnumMenuActions {
    fn handle(&mut self, world: &mut World, event: &MenuActionMessage) {
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, world);
        if let MenuActionMessage::Activate(_) = event
            && let EnumMenuActions::SelectStartLevelEnum = self
        {
            start_game(commands.reborrow());
        }
        queue.apply(world);
    }
}

#[derive(Debug, Clone)]
pub(crate) enum VolumeMenuActions {
    MainVolumeSlider,
    MusicVolumeSlider,
    EffectsVolumeSlider,
    UiVolumeSlider,
    // AmbientVolumeSlider,
}

impl MenuItemHandler for VolumeMenuActions {}

fn on_enter_audio_menu(
    font: Res<UiFont>,
    mut commands: Commands,
    program_state: Res<State<ProgramState>>,
    mut history: ResMut<MenuItemSelectionHistory>,
) {
    use bevy_seedling::prelude::*;

    macro_rules! make_volume_getter_setter_mute {
        ($getter:ident $setter:ident $get_mute:ident $set_mute:ident => $bus_or_pool:path) => {
            let $getter = commands.register_system(IntoSystem::into_system(
                |In(entity): In<Entity>,
                 mut slider_q: Query<&mut MenuSlider>,
                 vol_q: Single<&mut UserVolume, With<$bus_or_pool>>| {
                    slider_q.get_mut(entity).unwrap().current = Some(vol_q.volume.linear());
                },
            ));
            let $setter = commands.register_system(IntoSystem::into_system(
                |In(v): In<f32>, mut vol_q: Single<&mut UserVolume, With<$bus_or_pool>>| {
                    vol_q.volume = Volume::Linear(v);
                },
            ));
            let $get_mute = commands.register_system(IntoSystem::into_system(
                |In(entity): In<Entity>,
                 mut toggle_q: Query<&mut MenuToggle>,
                 vol_q: Single<&mut UserVolume, With<$bus_or_pool>>| {
                    toggle_q.get_mut(entity).unwrap().current = Some(!vol_q.muted);
                },
            ));
            let $set_mute = commands.register_system(IntoSystem::into_system(
                |In(v): In<bool>, mut vol_q: Single<&mut UserVolume, With<$bus_or_pool>>| {
                    vol_q.muted = !v;
                },
            ));
        };
    }

    make_volume_getter_setter_mute!(get_master set_master get_master_muted set_master_muted => MainBus);
    make_volume_getter_setter_mute!(get_music set_music  get_music_muted set_music_muted => SamplerPool<Music>);
    make_volume_getter_setter_mute!(get_effects set_effects  get_effects_muted set_effects_muted  => SamplerPool<Sfx>);
    make_volume_getter_setter_mute!(get_ui set_ui  get_ui_muted set_ui_muted  => SamplerPool<UiSfx>);

    let make_audio_slider = |getter, setter, defval| -> MenuSlider {
        MenuSlider::new(
            getter,
            setter,
            move || defval,
            |v| (v * 100.0).round(),
            |v| v / 100.0,
            0.0..=100.0,
            1.0,
        )
    };

    MenuItemBuilder::new(
        commands,
        OverlayState::AudioMenu,
        *program_state.get(),
        font.0.clone(),
        1.0,
        &history,
    )
    .add_item(
        "Master Volume",
        (
            make_audio_slider(get_master, set_master, Some(0.7)),
            MenuToggle::new(get_master_muted, set_master_muted),
        ),
        VolumeMenuActions::MainVolumeSlider,
    )
    .add_item(
        "Music Volume",
        (
            make_audio_slider(get_music, set_music, Some(0.5)),
            MenuToggle::new(get_music_muted, set_music_muted),
        ),
        VolumeMenuActions::MusicVolumeSlider,
    )
    .add_item(
        "Effects Volume",
        (
            make_audio_slider(get_effects, set_effects, Some(0.7)),
            MenuToggle::new(get_effects_muted, set_effects_muted),
        ),
        VolumeMenuActions::EffectsVolumeSlider,
    )
    .add_item(
        "UI Volume",
        (
            make_audio_slider(get_ui, set_ui, Some(1.0)),
            MenuToggle::new(get_ui_muted, set_ui_muted),
        ),
        VolumeMenuActions::UiVolumeSlider,
    )
    .add_item("Back", (), SimpleMenuActions::Back)
    .finish(&mut history);
}

enum ControlMenuToggleActions {
    TurnInvertX,
    TurnInvertY,
}

impl MenuItemHandler for ControlMenuToggleActions {}

fn on_enter_controls_menu(
    font: Res<UiFont>,
    program_state: Res<State<ProgramState>>,
    mut commands: Commands,
    mut history: ResMut<MenuItemSelectionHistory>,
) {
    // Scales are edited logarithmically.
    fn sens_to_ui(v: f32) -> f32 {
        if v > 0.0 { v.log2() } else { 0.0 }
    }
    fn sens_from_ui(v: f32) -> f32 {
        v.exp2()
    }

    macro_rules! make_getter_setter {
        ($getter:ident $setter:ident => $field1:ident $field2:tt) => {
            let $getter = commands.register_system(IntoSystem::into_system(
                |In(entity): In<Entity>,
                 res: Res<PlayerControllerSettings>,
                 mut slider_q: Query<&mut MenuSlider>| {
                    slider_q.get_mut(entity).unwrap().current = Some(res.$field1.$field2);
                },
            ));
            let $setter = commands.register_system(IntoSystem::into_system(
                |In(v): In<f32>, mut res: ResMut<PlayerControllerSettings>| {
                    res.$field1.$field2 = v;
                },
            ));
        };
    }

    macro_rules! make_toggle {
        ($getter:ident $setter:ident => $field:ident) => {
            let $getter = commands.register_system(IntoSystem::into_system(
                |In(entity): In<Entity>,
                 mut toggle_q: Query<&mut MenuToggle>,
                 res: Res<PlayerControllerSettings>| {
                     let current = res.$field;
                    toggle_q.get_mut(entity).unwrap().current = Some(current);
                },
            ));
            let $setter = commands.register_system(IntoSystem::into_system(
                |In(v): In<bool>, mut res: ResMut<PlayerControllerSettings>| {
                    res.$field = v;
                },
            ));
        };
    }

    make_getter_setter!(get_move_x set_move_x => move_scale x);
    make_getter_setter!(get_move_y set_move_y => move_scale y);
    make_getter_setter!(get_move_z set_move_z => move_scale z);
    make_getter_setter!(get_turn_x set_turn_x => turn_scale x);
    make_getter_setter!(get_turn_y set_turn_y => turn_scale y);
    make_getter_setter!(get_turn_z set_turn_z => turn_scale z);

    make_toggle!(get_invert_turn_x set_invert_turn_x => invert_turn_x);
    make_toggle!(get_invert_turn_y set_invert_turn_y => invert_turn_y);

    MenuItemBuilder::new(
        commands,
        OverlayState::ControlsMenu,
        *program_state.get(),
        font.0.clone(),
        0.75,
        &history,
    )
    .add_item(
        "Move Left/Right Power",
        MenuSlider::new(
            get_move_x,
            set_move_x,
            || Some(PlayerControllerSettings::default().move_scale.x),
            sens_to_ui,
            sens_from_ui,
            -5.0f32..=5.0f32,
            0.1,
        ),
        SliderMenuActions::MoveSensitivityXSlider,
    )
    .add_item(
        "Move Up/Down Power",
        MenuSlider::new(
            get_move_y,
            set_move_y,
            || Some(PlayerControllerSettings::default().move_scale.y),
            sens_to_ui,
            sens_from_ui,
            -5.0f32..=5.0f32,
            0.1,
        ),
        SliderMenuActions::MoveSensitivityYSlider,
    )
    .add_item(
        "Move Forward/Back Power",
        MenuSlider::new(
            get_move_z,
            set_move_z,
            || Some(PlayerControllerSettings::default().move_scale.z),
            sens_to_ui,
            sens_from_ui,
            -5.0f32..=5.0f32,
            0.1,
        ),
        SliderMenuActions::MoveSensitivityZSlider,
    )
    .add_item("Invert Turn X", (
        MenuToggle::new(get_invert_turn_x, set_invert_turn_x),
    ), ControlMenuToggleActions::TurnInvertX)
    .add_item("Invert Turn Y", (
        MenuToggle::new(get_invert_turn_y, set_invert_turn_y),
    ), ControlMenuToggleActions::TurnInvertY)
    .add_item(
        "Turn X Power",
        MenuSlider::new(
            get_turn_x,
            set_turn_x,
            || Some(PlayerControllerSettings::default().turn_scale.x),
            sens_to_ui,
            sens_from_ui,
            -5.0f32..=5.0f32,
            0.1,
        ),
        SliderMenuActions::TurnSensitivityXSlider,
    )
    .add_item(
        "Turn Y Power",
        MenuSlider::new(
            get_turn_y,
            set_turn_y,
            || Some(PlayerControllerSettings::default().turn_scale.y),
            sens_to_ui,
            sens_from_ui,
            -5.0f32..=5.0f32,
            0.1,
        ),
        SliderMenuActions::TurnSensitivityYSlider,
    )
    .add_item(
        "Turn Z Power",
        MenuSlider::new(
            get_turn_z,
            set_turn_z,
            || Some(PlayerControllerSettings::default().turn_scale.z),
            sens_to_ui,
            sens_from_ui,
            -5.0f32..=5.0f32,
            0.1,
        ),
        SliderMenuActions::TurnSensitivityZSlider,
    )
    .add_item("Back", (), SimpleMenuActions::Back)
    .finish(&mut history);
}

fn on_enter_video_menu(
    font: Res<UiFont>,
    program_state: Res<State<ProgramState>>,
    mut commands: Commands,
    mut history: ResMut<MenuItemSelectionHistory>,
) {
    let get_fov = commands.register_system(IntoSystem::into_system(
        |In(entity): In<Entity>, s: Res<VideoSettings>, mut slider_q: Query<&mut MenuSlider>| {
            slider_q.get_mut(entity).unwrap().current = Some(s.fov_degrees);
        },
    ));
    let set_fov = commands.register_system(IntoSystem::into_system(
        |In(v): In<f32>, mut commands: Commands, mut s: ResMut<VideoSettings>| {
            s.fov_degrees = v;
            commands.init_resource::<VideoCameraSettingsChanged>();
        },
    ));

    macro_rules! make_res_enum_getter_setter {
        ($getter:ident $setter:ident => $enum:ident $res:ident $field:tt) => {
            let $getter = commands.register_system(IntoSystem::into_system(
                |In(entity): In<Entity>, mut enum_q: Query<&mut MenuEnum>, res: Res<$res>| {
                    enum_q.get_mut(entity).unwrap().current = Some(
                        $enum::VARIANTS
                            .iter()
                            .position(|e| *e == res.$field)
                            .unwrap(),
                    );
                },
            ));
            let $setter = commands.register_system(IntoSystem::into_system(
                |In(v): In<usize>, mut res: ResMut<$res>, mut commands: Commands| {
                    res.$field = $enum::VARIANTS[v];
                    commands.init_resource::<VideoEffectSettingsChanged>();
                },
            ));
        };
    }

    make_res_enum_getter_setter!(get_anti set_anti => Antialiasing VideoSettings antialiasing);
    make_res_enum_getter_setter!(get_tex_qual set_tex_qual => TextureQuality VideoSettings texture_quality);

    MenuItemBuilder::new(
        commands,
        OverlayState::VideoMenu,
        *program_state.get(),
        font.0.clone(),
        1.0,
        &history,
    )
    .add_item(
        "Field of View",
        MenuSlider::new(
            get_fov,
            set_fov,
            || Some(VideoSettings::default().fov_degrees),
            |v| v,
            |v| v.round(),
            5.0f32..=120.0f32,
            5.0,
        ),
        SliderMenuActions::FovSlider,
    )
    .add_item(
        "Antialiasing",
        MenuEnum::new(
            get_anti,
            set_anti,
            || Antialiasing::VARIANTS.len(),
            |index| Antialiasing::VARIANTS[index].to_string(),
        ),
        EnumMenuActions::AntialiasingEnum,
    )
    // .add_item(
    //     "Mesh Quality",
    //     MenuEnum::new(
    //         get_mesh_qual,
    //         set_mesh_qual,
    //         || MeshQuality::VARIANTS.len(),
    //         |index| MeshQuality::VARIANTS[index].to_string(),
    //     ),
    //     EnumMenuActions::MeshQualityEnum,
    // )
    .add_item(
        "Texture Quality",
        MenuEnum::new(
            get_tex_qual,
            set_tex_qual,
            || TextureQuality::VARIANTS.len(),
            |index| TextureQuality::VARIANTS[index].to_string(),
        ),
        EnumMenuActions::TextureQualityEnum,
    )
    .add_item("Back", (), SimpleMenuActions::Back)
    .finish(&mut history);
}

fn start_game(mut commands: Commands) {
    commands.set_state(OverlayState::Loading);
    commands.set_state(ProgramState::InGame);
    commands.set_state(GameplayState::AssetsLoaded);
    // commands.insert_resource(ConnectToServer);
}

/// Current difficulty.
#[derive(Resource, Default, Debug, Clone, PartialEq, Reflect)]
#[reflect(Resource)]
#[type_path = "game"]

pub struct LevelDifficulty(pub Difficulty);

/// Difficulty rating.
#[derive(
    Resource,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Default,
    Reflect,
    strum::EnumIter,
    strum_macros::Display,
    strum::VariantArray,
)]
#[reflect(Resource)]
#[type_path = "game"]
pub enum Difficulty {
    Easy,
    #[default]
    Normal,
    Hard,
}

struct MyGamePlugin;

impl Plugin for MyGamePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(
                OnExit(ProgramState::New),
                ensure_levels
            )
            .add_systems(
                OnEnter(GameplayState::Setup),
                (
                    level_spawn_started,
                    spawn_level,
                ).chain()
            )

            .add_systems(
                OnExit(GameplayState::Setup),
                (
                    level_spawn_finished,
                ).chain()
            )

            .add_systems(
                OnTransition{ exited: GameplayState::Playing, entered: GameplayState::Setup },
                (
                    hide_instructions,
                    despawn_level,
                )
            )
            .add_systems(OnEnter(LevelState::LevelLoaded),
                (
                    add_player,
                    start_skybox_setup,
                    show_instructions,
                ).chain()
                .run_if(in_state(ProgramState::InGame))
            )

            .add_systems(OnExit(LevelState::Playing),
                hide_instructions,
            )

            .add_systems(
                OnEnter(LevelState::Advance),
                advance_level
            )

            .add_systems(
                Update,
                (
                    init_player_settings,
                    spawn_player_on_start,
                )
                .chain()
                .run_if(added_player_start)
                .run_if(in_state(GameplayState::Playing))
            )

            .add_systems(
                OnEnter(LevelState::Won),
                won_level,
            )
            .add_systems(
                OnEnter(LevelState::Lost),
                lost_level
            )
            .add_systems(
                Update,
                check_end_level
                    .run_if(resource_exists::<AutoEndLevelTimer>)
            )
        ;
    }
}


const END_LEVEL_DELAY_SECS: u64 = 3;

/// Countdown to next or same level.
#[derive(Resource, Reflect, Default)]
#[reflect(Resource, Default)]
#[type_path = "game"]
pub struct AutoEndLevelTimer(pub(crate) Timer);


fn won_level(
    mut commands: Commands,
    mut score_q: Single<(&mut Text, &mut TextColor), With<GameStatusArea>>,
) {
    let (ref mut text, ref mut color) = *score_q;
    text.0 = "Passed!".to_string();
    color.0 = Color::Srgba(tailwind::LIME_300);

    commands.insert_resource(AutoEndLevelTimer(Timer::new(Duration::from_secs(END_LEVEL_DELAY_SECS), TimerMode::Once)));
}

fn lost_level(
    mut commands: Commands,
    mut score_q: Single<(&mut Text, &mut TextColor), With<GameStatusArea>>,
) {
    let (ref mut text, ref mut color) = *score_q;
    text.0 = "Failed...\nTry again!".to_string();
    color.0 = Color::Srgba(tailwind::RED_700);

    commands.insert_resource(AutoEndLevelTimer(Timer::new(Duration::from_secs(END_LEVEL_DELAY_SECS), TimerMode::Once)));
}

fn check_end_level(
    mut commands: Commands,
    mut end_timer: ResMut<AutoEndLevelTimer>,
    time: Res<Time<Physics>>,
) {
    if !end_timer.0.tick(time.delta()).is_finished() {
        return;
    }

    // Restarts level.
    commands.set_state(LevelState::Advance);
}

/// If you have plugins registering levels in response
/// to e.g. `OnEnter(ProgramState::New)`, their order is unpredictable.
/// This sorts them.
pub(crate) fn ensure_levels(mut level_list: ResMut<LevelList>) {
    level_list.0.sort_by(|a, b| a.id.cmp(&b.id));
}

fn add_player(
    mut commands: Commands,
) {
    // Our model doesn't have a PlayerStart, so make one.
    commands.spawn((
        Name::new("PlayerStart"),
        PlayerStart,
        Transform::IDENTITY,
    ));
}

fn start_skybox_setup(
    mut commands: Commands,
    // world_camera_q: Query<Entity, (With<Camera3d>, With<WorldCamera>)>,
    // skyboxes: Res<SkyboxAssets>,
) {
    // let cam = world_camera_q.single().unwrap();

    // let (brightness, skybox) = (light_consts::lux::CLEAR_SUNRISE, skyboxes.pure_sky.clone());
    // let with_reflection_probe = Some((cam, 100.0));
    // commands.entity(cam).insert(SkyboxModel {
    //     skybox: Skybox {
    //         image: skybox,
    //         brightness,
    //         ..default()
    //     },
    //     xfrm: SkyboxTransform::From1_0_2f_3f_4_5,
    //     with_reflection_probe,
    //     enabled: true, //state.show_skybox,
    // });

    // commands.insert_resource(SkyboxSetup {
    //     waiting_skybox: true,
    //     waiting_reflections: false,
    // });
    // commands.set_state(LevelState::Configuring);
    // } else {
    // }

    // None in this game.
    commands.set_state(LevelState::Playing);
}

pub(crate) fn level_spawn_started(mut commands: Commands, mut pause: ResMut<PauseState>) {
    commands.set_state(LevelState::Initializing);
    commands.set_state(OverlayState::Loading);

    // Prevent moving/interacting while loading UI is up.
    pause.set_menu_paused(true);

    commands.remove_resource::<AutoEndLevelTimer>();
}

// fn observe_spawn_mesh(
//     event: On<SceneInstanceReady>,
//     child_q: Query<&Children>,
//     meshes: Query<&Mesh3d>,
//     mut commands: Commands,
// ) {
//     dbg!(event.event_target());
//     for ent in child_q.iter_descendants(event.event_target()) {
//         if meshes.contains(ent) {
//             commands.entity(ent).insert((
//                 ColliderConstructor::ConvexHullFromMesh,
//                 CollisionLayers::new(
//                     GameLayer::World,
//                     [
//                         GameLayer::Default,
//                         GameLayer::World,
//                         GameLayer::Player,
//                         GameLayer::Projectiles,
//                     ],
//                 ),
//             ));
//         }
//     }
// }

pub(crate) fn level_spawn_finished(
    mut commands: Commands,
    mut pause: ResMut<PauseState>,
) {
    commands.set_state(OverlayState::Hidden);
    commands.set_state(LevelState::LevelLoaded);

    // Go for it, user (unless they did set_user_paused)
    pause.set_menu_paused(false);
}

pub(crate) fn spawn_level(
    mut commands: Commands,
    level_list: Res<LevelList>,
    level_index: Res<LevelIndex>,
    world: Res<WorldMarkerEntity>,
    mut score_q: Query<&mut Text, (With<ScoreArea>, Without<GameStatusArea>)>,
    mut status_q: Query<&mut Text, (With<GameStatusArea>, Without<ScoreArea>)>,
) {
    setup_level(commands.reborrow(), &level_list, &level_index);

    let level = &level_list.0[level_index.0];
    log::info!("Entering level {}", level.label);

    commands
        .spawn((
            DespawnOnExit(GameplayState::Playing),
            SceneRoot(level.scene.clone()),
            ChildOf(world.0),
        ))
        .observe(|_event: On<SceneInstanceReady>, mut commands: Commands,| {
            commands.set_state(GameplayState::Playing);
        })
    ;

    score_q.single_mut().unwrap().clear();
    status_q.single_mut().unwrap().clear();
}

pub(crate) fn despawn_level(
    mut commands: Commands,
    sounds_q: Query<Entity, With<bevy_seedling::sample::SamplePlayer>>,
    spawned_q: Query<Entity, With<Spawned>>,
    player_q: Query<Entity, With<Player>>,
) {
    for ent in sounds_q.iter() {
        commands.entity(ent).try_despawn();
    }
    for ent in spawned_q.iter() {
        commands.entity(ent).try_despawn();
    }
    for ent in player_q.iter() {
        commands.entity(ent).try_despawn();
    }
}

pub(crate) fn spawn_player_on_start(world: &mut World) {
    // Make the player collision model and Player
    let player_ent = spawn_fps_player(
        world,
        Uuid::default(),
        QUAKE_SCALE,
        Transform::IDENTITY,
    );

    // Move to start position/orientation.
    let mut start_q = world.query_filtered::<&Transform, With<PlayerStart>>();
    let Some(xfrm) = start_q.iter(world).next() else {
        log::error!("no PlayerStart");
        return;
    };
    drop(start_q);
    let xfrm = xfrm.clone();

    let mut queue = CommandQueue::default();
    let mut commands = Commands::new(&mut queue, world);

    // Put and orient the new Player where the PlayerStart is.
    commands.entity(player_ent).insert((
        PlayerLook { rotation: xfrm.rotation, .. default() },
        xfrm
    ));

    queue.apply(world);
}

pub(crate) fn setup_level(
    mut commands: Commands,
    level_list: &LevelList,
    level_index: &LevelIndex,
) {
    let index = level_index.0;
    if index >= level_list.0.len() {
        log::error!("no items in LevelList");
        commands.remove_resource::<CurrentLevel>();
        commands.set_state(ProgramState::Error);
        return;
    }

    let level = &level_list.0[level_index.0];
    commands.insert_resource(CurrentLevel(level.clone()));
}

fn init_player_settings(
    move_q: Query<&PlayerCameraMode, With<LevelRoot>>,
    mut commands: Commands,
    mut settings: ResMut<PlayerInputSettings>,
) {
    let mode = if let Ok(mode) = move_q.single() {
        mode.clone()
    } else {
        log::warn!("no PlayerCameraMode in LevelRoot");
        PlayerCameraMode(PlayerMode::Fps)
    };
    match mode.0 {
        PlayerMode::Fps => *settings = PlayerInputSettings::for_fps(),
        PlayerMode::Space => *settings = PlayerInputSettings::for_space(),
    }
    commands.insert_resource(mode.0);
}

fn show_instructions(
    mut commands: Commands,
    // showed: Option<Res<ShowedTutorial>>,
    ui_font: Res<UiFont>,
    instructions_q: Single<Entity, With<InstructionsArea>>,
) {
    // if showed.is_some() {
    //     return;
    // }

    // commands.insert_resource(ShowedTutorial);

    let mut text_ent = Entity::PLACEHOLDER;

    commands.entity(*instructions_q).insert(Visibility::Inherited)  // show
    .with_children(|builder| {
        text_ent = builder.spawn((
            DespawnOnExit(GameplayState::Playing),
            Text::new("Move around with WSAD.",
            ),
            TextLayout::new(Justify::Center, LineBreak::WordBoundary),
            TextFont {
                font: ui_font.0.clone(),
                font_size: 32.0,
                .. default()
            },
            TextColor(Color::WHITE.with_alpha(0.5)),
            TextShadow {
                offset: Vec2::splat(2.),
                color: Color::linear_rgba(0., 0., 0., 0.0),
            },
        )).id();
    });

    // Fade in and out.

    let color_tween = Tween::new(
        EaseMethod::EaseFunction(EaseFunction::CubicOut),
        Duration::from_secs_f32(3.0),
        TextColorLens {
            start: Color::WHITE.with_alpha(0.0),
            end: Color::WHITE.with_alpha(1.0),
        }
    )
    .with_repeat(2, bevy_tweening::RepeatStrategy::MirroredRepeat);

    let shadow_tween = Tween::new(
        EaseMethod::EaseFunction(EaseFunction::CubicOut),
        Duration::from_secs_f32(3.0),
        TextShadowColorLens {
            start: Color::linear_rgba(0., 0., 0., 0.0),
            end: Color::linear_rgba(0., 0., 0., 1.0),
        }
    )
    .with_repeat(2, bevy_tweening::RepeatStrategy::MirroredRepeat);

    commands.entity(text_ent).try_insert((
        DespawnOnExit(GameplayState::Playing),
        TweenAnim::new(color_tween).with_destroy_on_completed(true),

        // Add another TweenAnim.
        children![(
            TweenAnim::new(shadow_tween).with_destroy_on_completed(true),
            AnimTarget::component::<TextShadow>(text_ent),
        )]
    ));
}

pub(crate) fn advance_level(
    mut commands: Commands,
    spawned_q: Query<Entity, With<Spawned>>,
) {
    for ent in spawned_q.iter() {
        commands.entity(ent).try_despawn();
    }
    commands.set_state(OverlayState::Loading);
    commands.set_state(GameplayState::Setup);
}


/// If the player collides with the [DeathboxCollider],
/// teleport the player back to start.
///
fn check_player_out_of_bounds(
    mut commands: Commands,
    parent_q: Query<&ChildOf>,
    player_q: Query<&Transform, With<Player>>,
    scene_q: Query<&SceneRoot>,
    sensor_q: Query<&CollidingEntities, With<DeathboxCollider>>,
    player_start_q: Query<&Transform, With<PlayerStart>>,
) {
    for coll in sensor_q.iter() {
        for ent in coll.iter() {
            let mut parent = *ent;
            loop {
                if let Ok(xfrm) = player_q.get(parent) {
                    let xfrm_tween = Tween::new(
                        EaseMethod::EaseFunction(EaseFunction::BackOut),
                        Duration::from_secs_f32(0.5),
                        TransformPositionLens {
                            start: xfrm.translation,
                            end: player_start_q.iter().next().unwrap().translation,
                        }
                    );
                    commands.entity(*ent).try_insert((
                        TweenAnim::new(xfrm_tween).with_destroy_on_completed(true),
                    ));

                    break;
                }
                if scene_q.contains(parent) {
                    break;
                }
                if let Ok(parent0) = parent_q.get(parent) {
                    parent = parent0.0;
                } else {
                    break;
                }
            }
        }
    }
}
