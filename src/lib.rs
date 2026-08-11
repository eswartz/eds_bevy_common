#![feature(iter_array_chunks)]

pub mod prelude;

pub mod physics;
pub mod app;
pub mod states_sets;
pub mod conditions;
pub mod markers;
pub mod layers;
pub mod areas;

pub mod levels;
pub mod despawn_on_reset;

pub mod player_types;
pub mod player_environment;
pub mod collision_hooks;

pub mod deathbox;

pub mod base_dir;
pub mod texutils;
pub mod model_utils;

#[cfg(feature = "input_bei")]
pub mod actions_common_bei;
#[cfg(feature = "input_bei")]
pub mod bei_better_pulse;
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
pub mod materials;

pub mod outlines;
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

//pub mod time_stretch;

pub mod colliders;
pub mod flashlight;
pub mod surface_material;
pub mod sound_sampler;

pub mod world_load_save;
