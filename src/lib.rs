#![feature(iter_array_chunks)]
#![recursion_limit = "256"]

pub mod prelude;

pub mod app;
pub mod areas;
pub mod conditions;
pub mod layers;
pub mod markers;
pub mod math;
pub mod physics;
pub mod standard_material_model;
pub mod states_sets;

pub mod despawn_on_reset;
pub mod levels;

pub mod collision_hooks;
pub mod player_environment;
pub mod player_types;

pub mod deathbox;

pub mod base_dir;
pub mod model_utils;
pub mod texutils;

pub mod actions_common;
#[cfg(feature = "input_bei")]
pub mod actions_common_bei;
pub mod audio;
#[cfg(feature = "input_bei")]
pub mod bei_better_pulse;
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
pub mod player_input;
pub mod player_move_look;

pub mod assets;
pub mod crosshair;
pub mod detail_normal;
pub mod effects;
pub mod lights;
pub mod parallax_depth;
pub mod skybox;
pub mod spawn_helpers;
pub mod split_into_cubes;

#[cfg(feature = "grabbing")]
pub mod grabbing;
#[cfg(feature = "highlighting")]
pub mod highlighting;
pub mod outlines;

pub mod menu_audio;
pub mod menu_utils;

#[cfg(feature = "midi_synth")]
pub mod client_synth;
#[cfg(feature = "midi_synth")]
pub mod midi_synth;
#[cfg(feature = "midi_synth")]
pub mod synth;

//pub mod time_stretch;

pub mod colliders;
pub mod flashlight;
pub mod sound_sampler;
pub mod surface_material;

pub mod world_load_save;
