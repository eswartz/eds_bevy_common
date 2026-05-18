#![feature(iter_array_chunks)]

pub mod app;
pub mod states_sets;
pub mod conditions;
pub mod markers;
pub mod layers;
pub mod areas;

pub mod product;
pub mod levels;
pub mod despawn_on_reset;

pub mod player_spawning;
pub mod collision_hooks;

pub mod deathbox;

pub mod base_dir;
pub mod texutils;
pub mod model_utils;

#[cfg(feature = "input_bei")]
pub mod actions_common_bei;
pub mod actions_common;
pub mod audio;
pub mod debug_egui;
pub mod gui;
pub mod lifecycle;
pub mod menus_common;
pub mod stats;
pub mod video;
pub mod world_state;

pub mod player_camera;
pub mod player_client;
pub mod player_controller;
pub mod player_move_look;
pub mod player_input;

pub mod assets;
pub mod crosshair;
pub mod effects;
pub mod lights;
pub mod skybox;
pub mod split_into_cubes;
pub mod detail_normal;
pub mod parallax_depth;

#[cfg(feature = "highlighting")]
pub mod highlighting;
#[cfg(feature = "grabbing")]
pub mod grabbing;

pub mod menu_audio;

#[cfg(feature = "midi_synth")]
pub mod synth;
#[cfg(feature = "midi_synth")]
pub mod client_synth;
#[cfg(feature = "midi_synth")]
pub mod midi_synth;

// FIX THIS SOON

pub use base_dir::*;
#[cfg(feature = "input_bei")]
pub use actions_common_bei::*;
pub use actions_common::*;
pub use audio::*;
pub use despawn_on_reset::*;
pub use debug_egui::*;
pub use gui::*;
pub use lifecycle::*;
pub use markers::*;
pub use layers::*;
pub use menus_common::*;
pub use product::*;
pub use states_sets::*;
pub use texutils::*;
pub use video::*;
pub use world_state::*;
pub use skybox::*;
pub use player_camera::*;
pub use player_client::*;
pub use player_controller::*;
pub use player_move_look::*;
pub use player_input::*;
pub use conditions::*;
pub use crosshair::*;
pub use effects::*;
pub use assets::*;
pub use levels::*;
pub use player_spawning::*;
pub use collision_hooks::*;
pub use areas::*;
pub use deathbox::*;
pub use stats::*;
pub use split_into_cubes::*;
pub use detail_normal::*;
pub use parallax_depth::*;
pub use menu_audio::*;
pub use model_utils::*;
pub use lights::*;
pub use app::*;

#[cfg(feature = "midi_synth")]
pub use synth::*;
#[cfg(feature = "midi_synth")]
pub use client_synth::*;
#[cfg(feature = "highlighting")]
pub use highlighting::*;
#[cfg(feature = "grabbing")]
pub use grabbing::*;
