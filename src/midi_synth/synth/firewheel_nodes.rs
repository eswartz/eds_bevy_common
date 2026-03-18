
use bevy::platform::sync::{Arc, atomic::{AtomicBool, Ordering},};
use bevy_seedling::prelude::ChannelCount;
use firewheel::{
    channel_config::ChannelConfig, diff::{Diff, Patch}, node::{AudioNode, AudioNodeInfo, AudioNodeProcessor, ConstructProcessorContext, ProcBuffers}
};

use std::collections::VecDeque;

use bevy::prelude::*;
use crossbeam_channel::{Receiver, Sender};

use super::{MidiRenderMessage, MidiSynthParams};

// /// This needs Default because of [MidiSynthPlayerNodeConfig].
// #[derive(Default)]
#[derive(Clone)]
pub(crate) struct SynthDecoder {
    quit: Arc<AtomicBool>,
    muted: Arc<AtomicBool>,
    handle: Option<Entity>,
    pub sample_rate: u32,
    stereo: bool,
    sender: Option<Sender<MidiRenderMessage>>,
    receiver: Receiver<Vec<f32>>,
    // writer: Arc<Mutex<firewheel::nodes::stream::writer::StreamWriterState>>,
    // writer: Arc<Mutex<firewheel::nodes::stream::writer::StreamWriterState>>,

    // /// One sample per channel.
    // queued_samples: Vec<f32>,
}

impl PartialEq for SynthDecoder {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
        && self.sample_rate == other.sample_rate
        && self.stereo == other.stereo
    }
}


impl SynthDecoder {
    pub(crate) fn new(
        params: &MidiSynthParams,
        handle: Entity,
        sender: Sender<MidiRenderMessage>,
        receiver: Receiver<Vec<f32>>,

        // writer: Arc<Mutex<firewheel::nodes::stream::writer::StreamWriterState>>,
        quit: Arc<AtomicBool>,
        muted: Arc<AtomicBool>,
    ) -> Self {
        // This always wraps in Some() because we are forced to use Default because of [MidiSynthPlayerNodeConfig].
        Self {
            quit,
            muted,
            handle: Some(handle),
            sample_rate: params.sample_rate as u32,
            stereo: params.channel_count > 1,
            sender: Some(sender),
            receiver,
            // writer,
            // queued_samples: Vec::new(),
        }
    }
}

pub(crate) struct SynthDecoderNodeProcessor {
    stereo: bool,
    sample_rate: u32,
    sender: Sender<MidiRenderMessage>,
    receiver: Receiver<Vec<f32>>,
    /// Samples we fetched from the Receiver but did not emit yet.
    head: VecDeque<f32>,
    quit: Arc<AtomicBool>,
    muted: Arc<AtomicBool>,
    // /// Average dt (secs) from the last few frames.
    // dt: f64,
}

impl SynthDecoderNodeProcessor {
    pub(crate) fn new(
        stereo: bool,
        sample_rate: u32,
        sender: Sender<MidiRenderMessage>,
        receiver: Receiver<Vec<f32>>,
        quit: Arc<AtomicBool>,
        muted: Arc<AtomicBool>,
    ) -> Self {
        // Kickstart.
        let _ = sender.send(MidiRenderMessage::RenderFrame((sample_rate / 32) as usize));
        Self {
            stereo,
            sample_rate,
            sender,
            head: VecDeque::with_capacity(sample_rate as usize),
            receiver,
            quit,
            muted,
            // dt: 1.0,
        }
    }
}

impl SynthDecoderNodeProcessor {
    fn fetch_data(&mut self) -> usize {
        if self.quit.load(Ordering::Relaxed) {
            return self.head.len()
        }

        // Drain incoming data.
        while let Ok(chunk) = self.receiver.try_recv() {
            // log::info!("adding {}", chunk.len());
            for samp in chunk {
                self.head.push_back(samp);
            }
        }

        // Ask for more if needed.
        let chunk = (self.sample_rate / 16) as usize;
        if self.head.len() <= chunk {
            // Kick off more generation.
            let _ = self.sender.send(MidiRenderMessage::RenderFrame(chunk));
        }

        self.head.len()
    }


    /// Channel-interleaved (but channel-count-independent) fetcher
    /// of data passed to us through the synth callback.
    fn next_sample(&mut self) -> Option<f32> {
        // If we're shutting down, do nothing as hard as possible.
        if self.quit.load(Ordering::Relaxed) {
            return None;
        }

        let Some(frame) = self.head.pop_front() else {
            return None
        };

        // If muted, fine, just return nothing
        // (after intentionally consuming cached frame above).
        if self.muted.load(Ordering::Relaxed) {
            return default();
        }

        Some(frame)
    }
}

#[derive(Diff, Patch, Debug, Clone, Component /*, Default, PartialEq, Hash, Eq */)]
pub struct MidiSynthPlayerNode;

impl Default for MidiSynthPlayerNodePatch {
    fn default() -> Self {
        panic!()
    }
}

#[derive(Component, PartialEq)]
#[derive(Clone)]
pub struct MidiSynthPlayerNodeConfig(pub Arc<SynthDecoder>);

// This is required for AudioNode but we don't want defaults.
impl Default for MidiSynthPlayerNodeConfig {
    fn default() -> Self {
        panic!("add the MidiSynthPlayerNodeConfig component explicitly")
    }
}


impl AudioNode for MidiSynthPlayerNode {
    type Configuration = MidiSynthPlayerNodeConfig;

    fn info(&self, configuration: &Self::Configuration) -> AudioNodeInfo {
        let decoder = &configuration.0;
        AudioNodeInfo::new()
            .debug_name("midi synth")
            .channel_config(ChannelConfig {
                num_inputs: ChannelCount::ZERO,
                num_outputs: if decoder.stereo { ChannelCount::STEREO } else { ChannelCount::MONO },
            })
    }

    fn construct_processor(
        &self,
        configuration: &Self::Configuration,
        cx: ConstructProcessorContext,
    ) -> impl AudioNodeProcessor {

        let decoder = &configuration.0;
        SynthDecoderNodeProcessor::new(decoder.stereo,
            cx.stream_info.sample_rate.get(),
            decoder.sender.clone().unwrap(),
            decoder.receiver.clone(),
            decoder.quit.clone(),
            decoder.muted.clone(),
        )
    }
}

impl AudioNodeProcessor for SynthDecoderNodeProcessor {
    fn new_stream(&mut self, stream_info: &firewheel::StreamInfo, context: &mut firewheel::node::ProcStreamCtx) {
        let _ = stream_info;
        let _ = context;
    }
    fn stream_stopped(&mut self, context: &mut firewheel::node::ProcStreamCtx) {
        let _ = context;
        self.quit.store(true, Ordering::SeqCst)
    }

    fn process(
        &mut self,
        proc_info: &firewheel::node::ProcInfo,
        ProcBuffers { inputs: _, outputs }: ProcBuffers,
        events: &mut firewheel::event::ProcEvents,
        _extra: &mut firewheel::node::ProcExtra,
    ) -> firewheel::node::ProcessStatus {

        for _patch in events.drain_patches::<MidiSynthPlayerNode>() {
            unreachable!("we have no config state, right?")
            // self.params.apply(patch);
        }

        let to_fill = proc_info.frames;
        let _avail = self.fetch_data();

        let mut last_defined_sample_index = 0;
        let mut underrun = false;
        let (out_left, rest) = outputs.split_first_mut().unwrap();
        if self.stereo {
            let out_right = &mut rest[0];

            for i in 0..to_fill {
                if let (Some(l), Some(r)) = (self.next_sample(), self.next_sample()) {
                    out_left[i] = l;
                    out_right[i] = r;
                    last_defined_sample_index = i;
                } else {
                    // Underrun. Have clear the buffers otherwise.
                    out_left[i] = 0.;
                    out_right[i] = 0.;
                    underrun = true;
                }
            }
        } else {
            for i in 0..to_fill {
                if let Some(l) = self.next_sample() {
                    out_left[i] = l;
                    last_defined_sample_index = i;
                } else {
                    // Underrun. Have to clear the buffers otherwise.
                    out_left[i] = 0.;
                    underrun = true;
                }
            }
        }

        let _ = underrun;
        // if underrun {
        //     log::warn!("underrun!");
        // }

        if last_defined_sample_index == 0 {
            firewheel::node::ProcessStatus::ClearAllOutputs
        } else {
            firewheel::node::ProcessStatus::OutputsModified
        }
    }


}
