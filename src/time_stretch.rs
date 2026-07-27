use bevy::prelude::*;
use bevy_seedling::firewheel::{channel_config::ChannelConfig, diff::{Diff, Patch}, event::ProcEvents, node::{AudioNode, AudioNodeInfo, AudioNodeProcessor, ConstructProcessorContext, EmptyConfig, NodeError, ProcBuffers, ProcExtra, ProcInfo, ProcessStatus}};
use timestretch::*;

#[derive(Patch, Diff, Debug, Clone, Copy, Component, PartialEq)]
pub struct TimeStretchNode {
    pub stretch_factor: f32,
}

impl AudioNode for TimeStretchNode {
    // Here we specify the configuration.
    //
    // Even if no configuration is required, `bevy_seedling` will
    // expect this to implement `Component`. You should generally reach for
    // Firehweel's `EmptyConfig` type in such a scenario.
    type Configuration = EmptyConfig;

    fn info(&self, _config: &Self::Configuration) -> Result<AudioNodeInfo, NodeError> {
        Ok(AudioNodeInfo::new()
            .debug_name("Timeshift")
            .channel_config(ChannelConfig {
                num_inputs: 1.into(),
                num_outputs: 1.into(),
            })
        )

    }

    fn construct_processor(
        &self,
        _config: &Self::Configuration,
        cx: ConstructProcessorContext,
    ) -> Result<impl AudioNodeProcessor, NodeError> {
        if (self.stretch_factor - 1.0).abs() < 0.0101 {
           return Ok(TimeStretchProcessor { params: None, output_chunk: default() })
        }

        let params = StretchParams::new(self.stretch_factor as _)
            .with_sample_rate(cx.stream_info.sample_rate.get() as _)
            .with_channels(1);

        Ok(TimeStretchProcessor {
            params: Some(params),
            output_chunk: Vec::with_capacity(cx.stream_info.sample_rate.get() as usize),
        })
    }
}

// You'll typically define a separate type for
// your audio processor calculations.
pub(crate) struct TimeStretchProcessor {
    pub(crate) params: Option<StretchParams>,
    output_chunk: Vec<f32>,
}

impl AudioNodeProcessor for TimeStretchProcessor {

    fn events(&mut self, info: &ProcInfo, events: &mut ProcEvents, _extra: &mut ProcExtra) {
        for patch in events.drain_patches::<TimeStretchNode>() {
            match patch {
                TimeStretchNodePatch::StretchFactor(stretch_factor) => {
                    if (stretch_factor - 1.0).abs() < 0.01 {
                        self.params = None;
                    } else {
                        let params = StretchParams::new(stretch_factor as _)
                            .with_sample_rate(info.sample_rate.get() as _)
                            .with_channels(1);
                        self.params = Some(params);
                    }
                }
            }
        }
    }

    fn process(
        &mut self,
        proc_info: &ProcInfo,
        ProcBuffers { inputs, outputs }: ProcBuffers,
        _: &mut ProcExtra,
    ) -> ProcessStatus {
        // Firewheel will inform you if an input channel is silent. If they're
        // all silent, we can simply skip processing and save CPU time.
        if proc_info.in_silence_mask.all_channels_silent(inputs.len()) {
            // All inputs are silent.
            return ProcessStatus::ClearAllOutputs;
        }

        for (input, output) in inputs.iter().zip(outputs.iter_mut()) {
            if let Some(params) = self.params.as_ref() {
                // One-shot stretching.
                match timestretch::stretch(input, params) {
                // match timestretch::stretch_to_bpm(input, 120.0, (120.0 * params.stretch_ratio), params) {
                    Err(e) => {
                        error!("stretch issue flush: {e}");
                        return ProcessStatus::ClearAllOutputs;
                    }
                    Ok(mut out) => {
                        self.output_chunk.append(&mut out);
                        if self.output_chunk.len() < output.len() {
                            // Delay until we have data.
                            return ProcessStatus::ClearAllOutputs;
                        } else {
                            let to_send = self.output_chunk.drain(0..output.len());
                            output.clone_from_slice(to_send.as_slice());
                        }
                    }
                }
            } else {
                output.clone_from_slice(&input[..]);
            }
        }

        ProcessStatus::OutputsModified
    }
}
