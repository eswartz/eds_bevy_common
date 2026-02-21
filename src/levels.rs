use bevy::prelude::*;

pub struct LevelsPlugin;

impl Plugin for LevelsPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(LevelList(default()))
            .insert_resource(LevelIndex(0))
            ;
    }
}


/// This defines a specific level.
#[derive(Reflect, Default, Clone, PartialEq, Debug)]
#[reflect(Default)]
#[type_path = "game"]
pub struct LevelInfo {
    /// Internal id.
    pub id: String,
    /// Label for menus / status screens.
    pub label: String,
    /// The scene that comprises the level (swapped in to [crate::WorldMarker]).
    pub scene: Handle<Scene>,
}

/// This defines the list of levels.
#[derive(Resource, Reflect, Default, Debug)]
#[reflect(Resource, Default)]
#[type_path = "game"]
pub struct LevelList(pub Vec<LevelInfo>);

pub fn is_in_level(id: &str) -> impl Fn(Option<Res<CurrentLevel>>) -> bool {
    move |level: Option<Res<CurrentLevel>>| -> bool {
        level.is_some_and(|l| {
            l.0.id == id
        })
    }
}

/// The current level, which holds a copy of the current level info.
#[derive(Resource, Reflect, Debug, Deref)]
#[reflect(Resource)]
#[type_path = "game"]
pub struct CurrentLevel(pub LevelInfo);

/// The level index into [LevelList].
#[derive(Resource, Default, Reflect)]
#[reflect(Resource, Default)]
pub struct LevelIndex(pub usize);
