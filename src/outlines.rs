use bevy::color::palettes::tailwind;
use bevy_mod_outline::*;

use bevy::prelude::*;

/// Enable `bevy_mod_outline`.
/// Added automatically by `HighlightingPlugin` and `GrabbingPlugin`.
pub struct OutlinesPlugin;

impl Plugin for OutlinesPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<OutlinePlugin>() {
            app.add_plugins(OutlinePlugin::JUMP_FLOOD);
        }
    }
}

/// This defines the default style items.
/// The given components are added (and removed) as needed.
#[derive(Reflect, Clone)]
#[reflect(Clone)]
#[type_path = "game"]
pub struct OutlineStyle {
    pub volume: OutlineVolume,
    pub stencil: Option<OutlineStencil>,
    pub inherit: Option<InheritOutline>,
}

impl Default for OutlineStyle {
    fn default() -> Self {
        Self {
            volume: OutlineVolume {
                visible: true,
                colour: tailwind::FUCHSIA_500.into(),
                width: 4.0,
            },
            stencil: None,
            inherit: None,
        }
    }
}
impl OutlineStyle {
    pub fn default_highlighting() -> Self {
        Self {
            volume: OutlineVolume {
                visible: true,
                width: 2.0,
                colour: Color::WHITE.with_alpha(0.5),
            },
            stencil: None,
            inherit: None,
        }
    }

    pub fn default_grabbing() -> Self {
        Self {
            volume: OutlineVolume {
                width: 4.0,
                colour: tailwind::LIME_500.with_alpha(0.75).into(),
                visible: true,
            },
            stencil: None,
            inherit: None,
        }
    }
}

impl OutlineStyle {
    pub fn apply_to<'a>(&self, mut ent_commands: EntityCommands<'a>) {
        ent_commands.try_insert(self.volume.clone());
        if let Some(stencil) = &self.stencil {
            ent_commands.try_insert(stencil.clone());
        }
        if let Some(inherit) = &self.inherit {
            ent_commands.try_insert(inherit.clone());
        }
    }
    pub fn remove_from<'a>(&self, mut ent_commands: EntityCommands<'a>) {
        ent_commands.try_remove::<OutlineVolume>();
        if self.stencil.is_some() {
            ent_commands.try_remove::<OutlineStencil>();
        }
        if self.inherit.is_some() {
            ent_commands.try_remove::<InheritOutline>();
        }
    }
}
