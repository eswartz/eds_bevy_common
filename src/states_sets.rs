use bevy::prelude::*;

/// This reflects the 2D overlay state.
#[derive(States, Default, Reflect, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[states(scoped_entities)]
#[reflect(State, Default)]
#[type_path = "game"]
pub enum OverlayState {
    /// No overlay.
    #[default]
    Hidden,
    /// Loading assets or levels.
    Loading,
    /// Main menu is up at startup.
    MainMenu,
    /// Escape Menu is up during gameplay.
    EscapeMenu,
    /// Game menu is up.
    GameMenu,
    /// Options menu is up.
    OptionsMenu,
    /// Audio menu is up.
    AudioMenu,
    /// Video menu is up.
    VideoMenu,
    /// Control menu is up.
    ControlsMenu,
    /// Game Over is up.
    GameOverScreen,
    /// Error is up.
    ErrorScreen,
    // /// egui controls are up
    // DebugGuiVisible,
}

impl OverlayState {
    pub fn is_menu(&self) -> bool {
        matches!(self,
            Self::MainMenu
            | Self::GameMenu
            | Self::OptionsMenu
            | Self::AudioMenu
            | Self::VideoMenu
            | Self::ControlsMenu
            | Self::EscapeMenu
        )
    }

    pub fn is_hidden(&self) -> bool {
        *self == OverlayState::Hidden
    }
}


/// State machine for overall program behavior.
#[derive(States, Reflect, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[states(scoped_entities)]
#[reflect(State, Default)]
#[type_path = "game"]
pub enum ProgramState {
    /// State before initial assets loaded.
    #[default]
    Initializing,
    /// Assets could not be loaded.
    Error,
    /// State when starting fresh, assets loaded.
    New,
    /// The main menu, shown to decide how to enter the game, and shown after exiting the game.
    /// (This runs outside the entire GameplayState lifecycle.)
    LaunchMenu,
    /// This state means the game proper is running.
    /// This state realizes [GameplayState].
    /// (though possibly paused, scripted, or behind a transient menu.
    /// Leaving this state destroys the game.
    InGame,
    /// Completed the program. This is not used internally,
    /// but may be useful for tracking when the program is exiting.
    Completed,
}

/// While in [ProgramState::InGame] ("in a game"),
/// these are the various modes the game can progress through.
/// These are very loose to avoid constraining game design
/// but define well-known sequence points in the user interface.
#[derive(SubStates, Reflect, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[states(scoped_entities)]
#[reflect(State, Default)]
#[source(ProgramState = ProgramState::InGame)]
#[type_path = "game"]
pub enum GameplayState {
    #[default]
    New,
    /// The assets are loaded (via [crate::assets]).
    /// This typically runs once per game.
    /// You need to manually progress to [GameplayState::Setup].
    AssetsLoaded,
    /// This state prompts loading the [CurrentLevel] level.
    /// This state is re-entered between levels.
    Setup,
    /// Game in progress.
    Playing,
    /// Game completed.
    Done,
}


/// State of a level (there is only one level in play at a time).
#[derive(SubStates, Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
#[states(scoped_entities)]
#[source(ProgramState = ProgramState::InGame)]
#[reflect(Default)]
#[type_path = "game"]
pub enum LevelState {
    /// Default state
    #[default]
    Initializing,
    /// Gameplay content has been loaded and initialized.
    /// then switch to Configuring or Playing.
    LevelLoaded,
    /// Intermediate state between loading and playing where
    /// items in the world need to be configured before playing.
    /// Systems should process items with the [crate::markers::ConfigureBeforePlaying]
    /// marker before switching to [Self::Playing].
    Configuring,
    /// Ready to play.
    Playing,
    /// In Win state.
    Won,
    /// In Lost state.
    Lost,
    /// Switching to some level ([crate::prelude::NextLevelIndex]).
    Advance,
    /// Prompt to enter a level from the start ([crate::prelude::CurrentLevel]).
    Enter,
}
