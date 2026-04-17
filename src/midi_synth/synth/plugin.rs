///! Wrap rustysynth
use std::{sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}}, time::Duration};
#[cfg(not(target_arch = "wasm32"))]
use std::thread;
use bevy_seedling::spatial::SpatialListener3D;
use firewheel::atomic_float::AtomicF32;
#[cfg(target_arch = "wasm32")]
use wasm_thread as thread;

use bevy::{ecs::entity::EntityHashMap, prelude::*};
#[cfg(feature = "firewheel")]
use bevy_seedling::node::RegisterNode as _;
use crossbeam_channel::{Receiver, Sender};
use rustysynth::{Synthesizer, SynthesizerSettings};
use serde::{Deserialize, Serialize};

use crate::PauseState;

use crate::midi_synth::{asset::{SoundFont, SoundFontLoader}, synth::{MidiMessage, MidiRenderMessage, firewheel_nodes::{MidiSynthPlayerNode, SynthDecoder}}};

/// The plugin.
#[derive(Default, Clone, Copy)]
pub struct MidiSynthPlugin;

impl Plugin for MidiSynthPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "firewheel")]
        app.register_node::<MidiSynthPlayerNode>();

        app.init_asset::<SoundFont>()
            .init_asset_loader::<SoundFontLoader>()
            .register_type::<MidiSynthParams>()
            .init_resource::<MidiSynths>()
            .init_resource::<MidiSynthsPaused>()
            .add_systems(
                PreUpdate,
                (
                    ensure_synths,
                    update_synths,
                    cleanup_synths,
                    check_pause_request_for_synths,
                )
            )
        ;
    }
}

/// Flag determining whether synths are paused.
#[derive(Resource, Default)]
pub struct MidiSynthsPaused(pub Arc<AtomicBool>);

/// Parameters for midi synthesis.
#[derive(Clone, Copy, Debug, PartialEq, Reflect, Serialize, Deserialize)]
pub struct MidiSynthParams {
    /// Number of channels (1 or 2).
    pub channel_count: u8,

    /// Amount of samples per each channel. Allows you to tweak audio latency, the more the value the more
    /// latency will be and vice versa. Keep in mind, that your data callback must be able to render the
    /// samples while previous portion of data is being played, otherwise you'll get a glitchy audio.
    pub channel_sample_count: usize,

    /// Sample rate of your audio data. Typical values are: 11025 Hz, 22050 Hz, 44100 Hz (default), 48000 Hz,
    /// 96000 Hz
    pub sample_rate: usize,

    /// Reverb level, 0 = none.
    pub reverb: f32,
}

impl Default for MidiSynthParams {
    fn default() -> Self {
        Self {
            channel_count: 1,
            sample_rate: 48000,
            channel_sample_count: 512,
            reverb: 0.0,
        }
    }
}

#[derive(Debug, TypePath)]
pub(crate) enum SynthState {
    LoadHandle {
        sound_font: Handle<SoundFont>,
        pending: Vec<MidiMessage>,
    },
    Loaded {
        synthesizer: Arc<Mutex<Synthesizer>>,
    },
}

/// A world-positioned midi synth.
#[derive(Component)]
pub struct MidiSynth {
    pub(crate) params: MidiSynthParams,
    entity: Entity,
    synth_state: SynthState,
    thread_handle: Option<thread::JoinHandle<()>>,

    pub thread_quit: Arc<AtomicBool>,
    muted: Arc<AtomicBool>,
    pub volume_linear: Arc<AtomicF32>,
    render_sender: Sender<MidiRenderMessage>,
    render_receiver: Receiver<MidiRenderMessage>,
    pub sample_receiver: Receiver<Vec<f32>>,
    pub sample_sender: Sender<Vec<f32>>,
}

#[derive(Component, Default)]
pub struct MidiSynthListener {

}

// #[cfg(feature = "kira")]
// type AudioVolume = kira::Decibels;
// #[cfg(feature = "firewheel")]
// type AudioVolume = firewheel::Volume;

// #[cfg(feature = "kira")]
// fn linear_to_decibels(x: f32) -> AudioVolume {
//     if x <= 0.0 { AudioVolume::SILENCE } else { kira::Decibels(20_f32 * x.log10()) }
// }
// #[cfg(feature = "firewheel")]
// fn linear_to_decibels(x: f32) -> AudioVolume {
//     if x <= 0.0 { AudioVolume::SILENT } else { AudioVolume::Decibels(20_f32 * x.log10()) }
// }

impl MidiSynth {

    /// Create with the given configuration.
    #[cfg(feature = "kira")]
    pub fn new(manager: Arc<Mutex<kira::AudioManager>>, params: MidiSynthParams, sound_font: Handle<SoundFont>, muted: Arc<AtomicBool>, entity: Entity) -> Result<Self> {
        use kira::*;
        use kira::track::*;
        use kira::effect::reverb::*;
        use kira::effect::filter::*;
        let mut mgr = manager.lock().unwrap();
        let reverb_send = mgr.add_send_track(
            SendTrackBuilder::new()
            .with_effect(ReverbBuilder::new()
                .mix(Mix::WET)
                .damping(0.25)
                .feedback(params.reverb as f64))
        ).unwrap();
        let listener = mgr.add_listener(glam::Vec3::ZERO, glam::Quat::IDENTITY).unwrap();
        let spatial_track = mgr.add_spatial_sub_track(&listener, glam::Vec3::ZERO,
            SpatialTrackBuilder::new()
                .persist_until_sounds_finish(false)
                .distances(SpatialTrackDistances {
                    min_distance: 0.1,
                    max_distance: 500.0,
                })
                .spatialization_strength(1.0)
                .with_effect(
                    FilterBuilder::new().cutoff(Value::FromListenerDistance(Mapping {
                            input_range: (0.0, 200.0),
                            output_range: (18000.0, 8000.0),
                            easing: Easing::Linear,
                    })),
                )
                .with_send(
                    &reverb_send,
                    Value::FromListenerDistance(Mapping {
                            input_range: (0.0, 200.0),
                            output_range: (Decibels(-12.0), Decibels(12.0)),
                            easing: Easing::Linear,
                    }),
                ),
            ).unwrap();

        drop(mgr);
        let (render_sender, render_receiver) = crossbeam_channel::unbounded();

        Ok(Self {
            params,
            entity,
            synth_state: SynthState::LoadHandle { sound_font, pending: vec![] },
            reverb_send,
            listener,
            spatial_track: Arc::new(Mutex::new(spatial_track)),
            volume_linear: 1.0,
            thread_quit: Arc::new(AtomicBool::new(false)),
            muted,
            thread_handle: None,
            render_sender,
            render_receiver,
        })
    }
    /// Create with the given configuration.
    #[cfg(feature = "firewheel")]
    pub fn new(
        // cx: &mut AudioContext,
        // stream_writer_id: firewheel::node::NodeID,
        params: MidiSynthParams,
        sound_font:
        Handle<SoundFont>,
        muted: Arc<AtomicBool>,
        entity: Entity,
        sample_sender: Sender<Vec<f32>>,
        sample_receiver: Receiver<Vec<f32>>,
    ) -> Result<Self> {
        let (render_sender, render_receiver) = crossbeam_channel::unbounded::<MidiRenderMessage>();

        Ok(Self {
            params,
            entity,
            synth_state: SynthState::LoadHandle { sound_font, pending: vec![] },
            volume_linear: Arc::new(AtomicF32::new(1.0)),
            thread_quit: Arc::new(AtomicBool::new(false)),
            muted,
            thread_handle: None,
            render_sender,
            render_receiver,
            sample_sender,
            sample_receiver,
        })
    }

    /// Send an event for the synth to play on the next frame.
    pub fn handle_event(&mut self, message: MidiMessage) {
        // dbg!(message);
        match &mut self.synth_state {
            SynthState::LoadHandle { pending, .. } => {
                pending.push(message);
            }
            SynthState::Loaded { synthesizer, .. } => {
                let data1 = message.data_1_byte() as i32;
                let data2 = message.data_2_byte() as i32;
                let channel = message.channel() as i32;
                let command = message.command() as i32;
                synthesizer.lock().unwrap().process_midi_message(channel, command, data1, data2);
            }
        }
    }
    pub fn handle_message(&self, synthesizer: &mut Synthesizer, message: MidiMessage) {
        let data1 = message.data_1_byte() as i32;
        let data2 = message.data_2_byte() as i32;
        let channel = message.channel() as i32;
        let command = message.command() as i32;
        synthesizer.process_midi_message(channel, command, data1, data2);
    }

    /// Returns true if the sound font has been loaded.
    pub fn is_ready(&self) -> bool {
        matches!(self.synth_state, SynthState::Loaded { .. })
    }
}

// #[derive(Serialize, Deserialize)]
struct SynthThreadParams {
    params: MidiSynthParams,
    thread_quit: Arc<AtomicBool>,
    muted: Arc<AtomicBool>,
    synthesizer: Arc<Mutex<Synthesizer>>,
    render_receiver: Receiver<MidiRenderMessage>,
    sample_sender: Sender<Vec<f32>>,
}

// impl DeserializeOwned for SynthThreadParams {
// }
// impl web_thread::Post for SynthThreadParams {}

impl MidiSynth {
    fn start(
        &mut self,
        synthesizer: Arc<Mutex<Synthesizer>>,
        mut commands: Commands,
        entity: Entity,
    ) -> anyhow::Result<SynthThread> {
        // Shouldn't happen, but in case we start twice...
        self.stop();

        let thread_quit = self.thread_quit.clone();
        let muted = self.muted.clone();
        let thread_params = self.params;
        #[cfg(feature = "kira")]
        let thread_track = self.spatial_track.clone();

        let decoder = SynthDecoder::new(
            &thread_params, self.entity, self.render_sender.clone(),
            self.sample_receiver.clone(),
            thread_quit.clone(),
            muted.clone(),
            self.volume_linear.clone(),
        );

        // let render_sender = self.render_sender.clone();
        let render_receiver = self.render_receiver.clone();
        let sample_sender = self.sample_sender.clone();

        let params = SynthThreadParams {
            params: thread_params,
            thread_quit,
            muted,
            synthesizer,
            render_receiver,
            sample_sender,
        };

        let thread_handle = thread::spawn(move || synth_thread(params));
        self.thread_handle = Some(thread_handle);

        #[cfg(feature = "firewheel")]
        {
            use bevy_seedling::edge::Connect as _;
            use crate::MusicBus;

            use crate::midi_synth::synth::firewheel_nodes::MidiSynthPlayerNodeConfig;
            let fwid = commands.entity(entity).insert((
                // ChildOf(entity),
                // Name::new("MidiSynth player"),
                MidiSynthPlayerNode,
                MidiSynthPlayerNodeConfig(Arc::new(decoder)),
            )).id();
            // let id = commands.spawn((
            //     ChildOf(entity),
            //     Name::new("MidiSynth player"),
            //     MidiSynthPlayerNode,
            //     MidiSynthPlayerNodeConfig(Arc::new(decoder)),
            // )).id();
            commands.entity(fwid).connect(MusicBus);
        }

        Ok(SynthThread{ thread_quit: self.thread_quit.clone() })
    }

    fn stop(&mut self) {
        if let Some(thread_handle) = self.thread_handle.take() {
            // i.e. still alive or configured
            self.thread_quit.store(true, Ordering::SeqCst);
            let _ = self.render_sender.send(MidiRenderMessage::Idle(0));

            #[cfg(not(target_arch = "wasm32"))]
            let _ = thread_handle.join();
        }
    }
}

impl Drop for MidiSynth {
    fn drop(&mut self) {
        self.stop();
    }
}

/// There is one thread per active synthesizer.
fn synth_thread(
    SynthThreadParams {
        params,
        thread_quit,
        muted,
        synthesizer,
        render_receiver,
        sample_sender,
    }: SynthThreadParams,
) {
    // Allocate for up to 1 second of rendering.
    let mut left = vec![0f32; params.sample_rate / 60];
    let mut right = vec![0f32; params.sample_rate / 60];

    let stereo = params.channel_count > 1;

    let mut was_idle = false;
    loop {
        if thread_quit.load(Ordering::SeqCst) {
            break
        }

        let message = if cfg!(target_arch = "wasm32") {
            let Ok(message) = render_receiver.try_recv() else {
                thread::sleep(Duration::from_millis(10));
                continue;
            };
            message
        } else {
            let Ok(message) = render_receiver.recv() else {
                break;
            };
            message
        };

        match message {
            MidiRenderMessage::RenderFrame(count) => {
                if was_idle {
                    was_idle = false;
                }

                let mut count = count;
                while count > 0 {
                    let chunk_size = count.min(left.len());
                    count -= chunk_size;

                    if muted.load(Ordering::Relaxed) {
                        left.fill(0.);
                        right.fill(0.);
                    } else {
                        synthesizer.lock().unwrap().render(&mut left[0..chunk_size], &mut right[0..chunk_size]);
                    }

                    let data = if stereo {
                        let mut data = Vec::with_capacity(chunk_size * 2);
                        for i in 0..chunk_size {
                            data.push(left[i]);
                            data.push(right[i]);
                        }
                        data
                    } else {
                        let mut data = Vec::with_capacity(chunk_size);
                        for i in 0..chunk_size {
                            data.push((left[i] + right[i]) * 0.5);
                        }
                        data
                    };
                    let _ = sample_sender.send(data);
                }
            }
            MidiRenderMessage::Idle(frames) => {
                if !was_idle {
                    let _ = sample_sender.send(vec![0_f32; frames]);
                    // let _ = decoder.write(vec![0_f32; frames]);

                    // handle.pause(default());
                    was_idle = true;
                }
            }
        }
    }

}

struct SynthThread {
    thread_quit: Arc<AtomicBool>,
}

/// This maps an Entity with a MidiSynth to the thread that is processing it.
///
#[derive(Resource, Default)]
struct MidiSynths(EntityHashMap<SynthThread>);

fn ensure_synths(
    mut commands: Commands,
    sf_assets: Res<Assets<SoundFont>>,
    mut synths: ResMut<MidiSynths>,
    mut synth_q: Query<(Entity, &mut MidiSynth)>,
) {
    for (ent, mut synth) in synth_q.iter_mut() {
        let SynthState::LoadHandle { sound_font, pending } = &synth.synth_state else {
            // Already initialized.
            continue;
        };
        let Some(sound_font) = sf_assets.get(sound_font) else {
            // Still loading.
            continue;
        };

        let pending = pending.clone();

        let synth_settings = SynthesizerSettings::new(synth.params.sample_rate as i32);

        let synthesizer = Arc::new(Mutex::new(Synthesizer::new(&sound_font.content, &synth_settings).unwrap()));

        match synth.start(synthesizer.clone(), commands.reborrow(), ent) {
            Ok(thread) => {
                let exist = synths.0.insert(ent, thread);
                if let Some(exist) = exist {
                    exist.thread_quit.store(true, Ordering::SeqCst);
                }

                // Pass any pending events.
                {
                    let mut synthesizer = synthesizer.lock().unwrap();
                    for event in pending {
                        synth.handle_message(&mut synthesizer, event);
                    }
                }

                synth.synth_state = SynthState::Loaded { synthesizer };

            }
            Err(e) => {
                error!("failed to start synth: {e:?}");
            }
        }
    }
}

fn update_synths(
    synth_q: Query<(&MidiSynth, &GlobalTransform)>,
    listener_q: Query<&GlobalTransform, With<SpatialListener3D>>,
) {
    let Ok(listener_xfrm) = listener_q.single() else { return };

    synth_q.par_iter().for_each(|(synth, xfrm)| {
        let distance = listener_xfrm.translation().distance(xfrm.translation());
        synth.volume_linear.store(1.0 / distance, Ordering::Relaxed);
    });
}

/// Stop any threads when MidiSynth is removed.
fn cleanup_synths(
    mut removed: RemovedComponents<MidiSynth>,
    mut synths: ResMut<MidiSynths>,
) {
    for ent in removed.read() {
        if let Some(synth) = synths.0.remove(&ent) {
            synth.thread_quit.store(true, Ordering::SeqCst);
        }
    }
}

fn check_pause_request_for_synths(
    paused: ResMut<PauseState>,
    synths_paused: Res<MidiSynthsPaused>,
) {
    if !paused.is_changed() {
        return
    }
    let pause = paused.is_paused();
    synths_paused.0.store(pause, Ordering::SeqCst);
}
