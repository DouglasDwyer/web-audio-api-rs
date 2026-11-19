//! The AudioNode interface and concrete types

use crate::render::{
    AudioParamValues, AudioProcessor, AudioRenderQuantum, AudioWorkletGlobalScope,
};
use crate::AudioBufferIter;

// traits
mod audio_node;
pub use audio_node::*;
mod scheduled_source;
pub use scheduled_source::*;

// nodes
#[cfg(feature = "AnalyserNode")]
mod analyser;
#[cfg(feature = "AnalyserNode")]
pub use analyser::*;
#[cfg(feature = "AudioBufferSourceNode")]
mod audio_buffer_source;
#[cfg(feature = "AudioBufferSourceNode")]
pub use audio_buffer_source::*;
#[cfg(feature = "BiquadFilterNode")]
mod biquad_filter;
#[cfg(feature = "BiquadFilterNode")]
pub use biquad_filter::*;
#[cfg(feature = "ChannelMergerNode")]
mod channel_merger;
#[cfg(feature = "ChannelMergerNode")]
pub use channel_merger::*;
#[cfg(feature = "ChannelSplitterNode")]
mod channel_splitter;
#[cfg(feature = "ChannelSplitterNode")]
pub use channel_splitter::*;
#[cfg(feature = "ConstantSourceNode")]
mod constant_source;
#[cfg(feature = "ConstantSourceNode")]
pub use constant_source::*;
#[cfg(feature = "ConvolverNode")]
mod convolver;
#[cfg(feature = "ConvolverNode")]
pub use convolver::*;
#[cfg(feature = "DelayNode")]
mod delay;
#[cfg(feature = "DelayNode")]
pub use delay::*;
mod destination;
pub use destination::*;
#[cfg(feature = "DynamicsCompressorNode")]
mod dynamics_compressor;
#[cfg(feature = "DynamicsCompressorNode")]
pub use dynamics_compressor::*;
#[cfg(feature = "GainNode")]
mod gain;
#[cfg(feature = "GainNode")]
pub use gain::*;
#[cfg(feature = "IIRFilterNode")]
mod iir_filter;
#[cfg(feature = "IIRFilterNode")]
pub use iir_filter::*;
#[cfg(feature = "MediaElementAudioSourceNode")]
mod media_element_source;
#[cfg(feature = "MediaElementAudioSourceNode")]
pub use media_element_source::*;
#[cfg(feature = "MediaStreamAudioDestinationNode")]
mod media_stream_destination;
#[cfg(feature = "MediaStreamAudioDestinationNode")]
pub use media_stream_destination::*;
#[cfg(feature = "MediaStreamAudioSourceNode")]
mod media_stream_source;
#[cfg(feature = "MediaStreamAudioSourceNode")]
pub use media_stream_source::*;
#[cfg(feature = "MediaStreamTrackAudioSourceNode")]
mod media_stream_track_source;
#[cfg(feature = "MediaStreamTrackAudioSourceNode")]
pub use media_stream_track_source::*;
#[cfg(feature = "OscillatorNode")]
mod oscillator;
#[cfg(feature = "OscillatorNode")]
pub use oscillator::*;
#[cfg(feature = "PannerNode")]
mod panner;
#[cfg(feature = "PannerNode")]
pub use panner::*;
#[cfg(feature = "ScriptProcessorNode")]
mod script_processor;
#[cfg(feature = "ScriptProcessorNode")]
pub use script_processor::*;
#[cfg(feature = "StereoPannerNode")]
mod stereo_panner;
#[cfg(feature = "StereoPannerNode")]
pub use stereo_panner::*;
#[cfg(feature = "WaveShaperNode")]
mod waveshaper;
#[cfg(feature = "WaveShaperNode")]
pub use waveshaper::*;

// `MediaStreamRenderer` is internally used by `MediaElementAudioSourceNode` and
// `MediaStreamAudioSourceNode`.
struct MediaStreamRenderer<R> {
    stream: R,
    finished: bool,
}

impl<R> MediaStreamRenderer<R> {
    fn new(stream: R) -> Self {
        Self {
            stream,
            // scheduler,
            finished: false,
        }
    }
}

impl<R: AudioBufferIter> AudioProcessor for MediaStreamRenderer<R> {
    fn process(
        &mut self,
        _inputs: &[AudioRenderQuantum],
        outputs: &mut [AudioRenderQuantum],
        _params: AudioParamValues<'_>,
        _scope: &AudioWorkletGlobalScope,
    ) -> bool {
        // single output node
        let output = &mut outputs[0];

        // @note - maybe we need to disciminate between a paused and depleted term
        match self.stream.next() {
            Some(Ok(buffer)) => {
                let channels = buffer.number_of_channels();
                output.set_number_of_channels(channels);
                output
                    .channels_mut()
                    .iter_mut()
                    .zip(buffer.channels())
                    .for_each(|(o, i)| o.copy_from_slice(i.as_slice()));
            }
            Some(Err(e)) => {
                log::warn!("Error playing audio stream: {}", e);
                self.finished = true; // halt playback
                output.make_silent()
            }
            None => {
                if !self.finished {
                    log::debug!("Stream finished");
                    self.finished = true;
                }
                output.make_silent()
            }
        }

        !self.finished
    }
}
