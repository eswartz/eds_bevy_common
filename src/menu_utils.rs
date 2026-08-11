use bevy::{ecs::world::CommandQueue, prelude::*};
use crate::prelude::*;

#[derive(Debug)]
pub enum SimpleMenuActions {
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
                    commands.set_state(LevelState::Enter);
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

/// Common menu for Options, showing for Audio/Video/Controls.
pub fn on_enter_options_menu(
    gui_assets: Res<CommonGuiAssets>,
    commands: Commands,
    program_state: Res<State<ProgramState>>,
    mut history: ResMut<MenuItemSelectionHistory>,
) {
    MenuItemBuilder::new(
        commands,
        OverlayState::OptionsMenu,
        *program_state.get(),
        gui_assets.std_ui.clone(),
        1.0,
        &history,
    )
    .add_item("Audio", (), SimpleMenuActions::AudioMenu)
    .add_item("Video", (), SimpleMenuActions::VideoMenu)
    .add_item("Controls", (), SimpleMenuActions::ControlsMenu)
    .add_item("Back", (), SimpleMenuActions::Back)
    .finish(&mut history);
}

#[derive(Debug, Clone)]
pub(crate) enum VolumeMenuActions {
    MainVolumeSlider,
    MusicVolumeSlider,
    EffectsVolumeSlider,
    UiVolumeSlider,
}

impl MenuItemHandler for VolumeMenuActions {}

/// Common volume menu creator.
#[cfg(feature = "firewheel")]
pub fn on_enter_audio_menu(
    gui_assets: Res<CommonGuiAssets>,
    mut commands: Commands,
    program_state: Res<State<ProgramState>>,
    mut history: ResMut<MenuItemSelectionHistory>,
) {
    use bevy_seedling::prelude::MainBus;
    use bevy_seedling::prelude::Volume;

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
    make_volume_getter_setter_mute!(get_music set_music  get_music_muted set_music_muted => MusicNode);
    make_volume_getter_setter_mute!(get_effects set_effects  get_effects_muted set_effects_muted  => SfxNode);
    make_volume_getter_setter_mute!(get_ui set_ui  get_ui_muted set_ui_muted  => UiSfxNode);

    let make_audio_slider = |getter, setter, defval| -> MenuSlider {
        MenuSlider::new(
            getter,
            setter,
            move || defval,
            |v| (v * 100.0).round(),
            |v| v / 100.0,
            0.0..=100.0,
            5.0,
        )
    };

    MenuItemBuilder::new(
        commands,
        OverlayState::AudioMenu,
        *program_state.get(),
        gui_assets.std_ui.clone(),
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

/// GUI helper to add a enumerated menu item that allows selecting different available levels.
pub fn add_level_selector(
    builder: &mut MenuItemBuilder,
    label: &str,
    level_list: &LevelList,
    current_level: Option<&CurrentLevel>,
    handler: impl MenuItemHandler + 'static,
) {
    fn get_level(In(entity): In<Entity>, mut enum_q: Query<&mut MenuEnum>,
        level_index: Res<LevelIndex>,
        next_level_index: Option<Res<NextLevelIndex>>,
    ) {
        let index = next_level_index.map_or(level_index.0, |nli| nli.0);
        enum_q.get_mut(entity).unwrap().current = Some(index);
    }
    fn set_level(In(v): In<usize>, mut commands: Commands) {
        commands.insert_resource(NextLevelIndex(v));
    }
    let get_level = builder.commands().register_system(IntoSystem::into_system(get_level));
    let set_level = builder.commands().register_system(IntoSystem::into_system(set_level));

    let level_infos = level_list.0.clone();
    let current_level = current_level.cloned();
    let level_count = level_infos.len();
    let level_names = level_infos.iter().map(|info| info.label.clone()).collect::<Vec<_>>();

    builder
        .add_item(
            label,
            MenuEnum::new(
                get_level,
                set_level,
                move || level_count,
                move |index| {
                    if let Some(level) = &current_level && level.info.id == level_infos[index].id {
                        format!("{} (reset)", level_names[index])
                    } else if !level_names.is_empty() {
                        level_names[index].clone()
                    } else {
                        "???".to_string()
                    }
                }
            ),
            handler,
        );
}
