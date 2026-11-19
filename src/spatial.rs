//! Spatialization/Panning primitives
//!
//! Required for panning algorithm, distance and cone effects of panner nodes

use crate::context::{AudioContextRegistration, BaseAudioContext};
use crate::node::{
    AudioNode, AudioNodeOptions, ChannelConfig, ChannelCountMode, ChannelInterpretation,
};
use crate::param::{AudioParam, AudioParamDescriptor, AudioParamInner, AutomationRate};
use crate::render::{
    AudioParamValues, AudioProcessor, AudioRenderQuantum, AudioWorkletGlobalScope,
};

use std::sync::OnceLock;

/// AudioParam settings for the cartesian coordinates
pub(crate) const PARAM_OPTS: AudioParamDescriptor = AudioParamDescriptor {
    name: String::new(),
    min_value: f32::MIN,
    max_value: f32::MAX,
    default_value: 0.,
    automation_rate: AutomationRate::A,
};

/// Represents the position and orientation of the person listening to the audio scene
///
/// All [`PannerNode`](crate::node::PannerNode) objects spatialize in relation to the [BaseAudioContext's](crate::context::BaseAudioContext) listener.
///
/// # Usage
///
/// For example usage, check the [`PannerNode`](crate::node::PannerNode) docs.
#[derive(Debug)]
pub struct AudioListener {
    pub(crate) position_x: AudioParam,
    pub(crate) position_y: AudioParam,
    pub(crate) position_z: AudioParam,
    pub(crate) forward_x: AudioParam,
    pub(crate) forward_y: AudioParam,
    pub(crate) forward_z: AudioParam,
    pub(crate) up_x: AudioParam,
    pub(crate) up_y: AudioParam,
    pub(crate) up_z: AudioParam,
}

impl AudioListener {
    pub fn position_x(&self) -> &AudioParam {
        &self.position_x
    }
    pub fn position_y(&self) -> &AudioParam {
        &self.position_y
    }
    pub fn position_z(&self) -> &AudioParam {
        &self.position_z
    }
    pub fn forward_x(&self) -> &AudioParam {
        &self.forward_x
    }
    pub fn forward_y(&self) -> &AudioParam {
        &self.forward_y
    }
    pub fn forward_z(&self) -> &AudioParam {
        &self.forward_z
    }
    pub fn up_x(&self) -> &AudioParam {
        &self.up_x
    }
    pub fn up_y(&self) -> &AudioParam {
        &self.up_y
    }
    pub fn up_z(&self) -> &AudioParam {
        &self.up_z
    }
}

/// Wrapper for the [`AudioListener`] so it can be placed in the audio graph.
///
/// This node has no input, but takes the position/orientation AudioParams and copies them into the
/// 9 outputs. The outputs are connected to the PannerNodes (via an AudioParam).
///
/// The AudioListener is always connected to the AudioDestinationNode so at each
/// render quantum its positions are recalculated.
pub(crate) struct AudioListenerNode {
    registration: AudioContextRegistration,
    fields: AudioListener,
}

impl AudioNode for AudioListenerNode {
    fn registration(&self) -> &AudioContextRegistration {
        &self.registration
    }

    fn channel_config(&self) -> &ChannelConfig {
        static INSTANCE: OnceLock<ChannelConfig> = OnceLock::new();
        INSTANCE.get_or_init(|| {
            AudioNodeOptions {
                channel_count: 1,
                channel_count_mode: ChannelCountMode::Explicit,
                channel_interpretation: ChannelInterpretation::Discrete,
            }
            .into()
        })
    }

    fn number_of_inputs(&self) -> usize {
        0
    }

    fn number_of_outputs(&self) -> usize {
        9 // return all audio params as output
    }

    fn set_channel_count(&self, _v: usize) {
        panic!("NotSupportedError - AudioListenerNode has channel count constraints");
    }
    fn set_channel_count_mode(&self, _v: ChannelCountMode) {
        panic!("NotSupportedError - AudioListenerNode has channel count mode constraints");
    }
    fn set_channel_interpretation(&self, _v: ChannelInterpretation) {
        panic!("NotSupportedError - AudioListenerNode has channel interpretation constraints");
    }
}

impl AudioListenerNode {
    pub fn new<C: BaseAudioContext>(context: &C) -> Self {
        context.base().register(move |registration| {
            let forward_z_opts = AudioParamDescriptor {
                default_value: -1.,
                ..PARAM_OPTS
            };
            let up_y_opts = AudioParamDescriptor {
                default_value: 1.,
                ..PARAM_OPTS
            };

            let (p1, _v1) = context.create_audio_param(PARAM_OPTS, &registration);
            let (p2, _v2) = context.create_audio_param(PARAM_OPTS, &registration);
            let (p3, _v3) = context.create_audio_param(PARAM_OPTS, &registration);
            let (p4, _v4) = context.create_audio_param(PARAM_OPTS, &registration);
            let (p5, _v5) = context.create_audio_param(PARAM_OPTS, &registration);
            let (p6, _v6) = context.create_audio_param(forward_z_opts, &registration);
            let (p7, _v7) = context.create_audio_param(PARAM_OPTS, &registration);
            let (p8, _v8) = context.create_audio_param(up_y_opts, &registration);
            let (p9, _v9) = context.create_audio_param(PARAM_OPTS, &registration);

            let node = Self {
                registration,
                fields: AudioListener {
                    position_x: p1,
                    position_y: p2,
                    position_z: p3,
                    forward_x: p4,
                    forward_y: p5,
                    forward_z: p6,
                    up_x: p7,
                    up_y: p8,
                    up_z: p9,
                },
            };
            let proc = ListenerRenderer {};

            (node, Box::new(proc))
        })
    }

    pub fn into_fields(self) -> AudioListener {
        self.fields
    }
}

struct ListenerRenderer {}

impl AudioProcessor for ListenerRenderer {
    fn process(
        &mut self,
        _inputs: &[AudioRenderQuantum],
        _outputs: &mut [AudioRenderQuantum],
        _params: AudioParamValues<'_>,
        _scope: &AudioWorkletGlobalScope,
    ) -> bool {
        // do nothing, the Listener is just here to make sure the position/forward/up params render in order

        true // never drop
    }
}

/// Data holder for the BaseAudioContext so it can reconstruct the AudioListener on request
pub(crate) struct AudioListenerParams {
    pub position_x: AudioParamInner,
    pub position_y: AudioParamInner,
    pub position_z: AudioParamInner,
    pub forward_x: AudioParamInner,
    pub forward_y: AudioParamInner,
    pub forward_z: AudioParamInner,
    pub up_x: AudioParamInner,
    pub up_y: AudioParamInner,
    pub up_z: AudioParamInner,
}
