//! A node that is connected to the destination must be rendered in every render quantum, no matter
//! what the control thread does to *other* parts of the graph.
//!
//! Here a recorder worklet is fed by a source and connected to the destination. The control thread
//! repeatedly creates bursts of fire-and-forget nodes (created, then immediately dropped, so the
//! render thread frees them again). The recorder has to see every single render quantum, i.e. its
//! `current_frame` has to advance by exactly 128 between consecutive calls. Instead, some quanta
//! are skipped for the recorder: a 128 sample hole in the audio that reaches the output.
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::{Duration, Instant};

use web_audio_api::context::{AudioContext, AudioContextOptions, BaseAudioContext};
use web_audio_api::node::{AudioNode, AudioScheduledSourceNode};
use web_audio_api::worklet::{
    AudioParamValues, AudioWorkletGlobalScope, AudioWorkletNode, AudioWorkletNodeOptions,
    AudioWorkletProcessor,
};
use web_audio_api::AudioParamDescriptor;

/// Remembers the `current_frame` of every quantum in which it was rendered.
struct Recorder(Arc<Mutex<Vec<u64>>>);

impl AudioWorkletProcessor for Recorder {
    type ProcessorOptions = Arc<Mutex<Vec<u64>>>;

    fn constructor(opts: Self::ProcessorOptions) -> Self {
        Self(opts)
    }

    fn parameter_descriptors() -> Vec<AudioParamDescriptor> {
        Vec::new()
    }

    fn process<'a, 'b>(
        &mut self,
        _inputs: &'b [&'a [&'a [f32]]],
        _outputs: &'b mut [&'a mut [&'a mut [f32]]],
        _params: AudioParamValues<'b>,
        scope: &'b AudioWorkletGlobalScope,
    ) -> bool {
        self.0.lock().unwrap().push(scope.current_frame);
        true
    }
}

/// Returns the (frame, number of skipped quanta) of every hole in the recorder's calls.
fn skipped_quanta(nodes_per_burst: usize) -> Vec<(u64, u64)> {
    let context = AudioContext::new(AudioContextOptions {
        sink_id: "none".into(),
        ..AudioContextOptions::default()
    });

    let frames = Arc::new(Mutex::new(Vec::new()));
    let recorder = AudioWorkletNode::new::<Recorder>(
        &context,
        AudioWorkletNodeOptions {
            processor_options: frames.clone(),
            ..AudioWorkletNodeOptions::default()
        },
    );
    recorder.connect(&context.destination());
    let mut source = context.create_constant_source();
    source.connect(&recorder);
    source.start();

    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(3) {
        for _ in 0..nodes_per_burst {
            // fire and forget: nothing is connected, the handle is dropped right away
            let _ = context.create_gain();
        }
        sleep(Duration::from_millis(5));
    }
    sleep(Duration::from_millis(100));

    let frames = frames.lock().unwrap().clone();
    let quantum = 128;
    frames
        .windows(2)
        .filter(|w| w[1] != w[0] + quantum)
        .map(|w| (w[0], (w[1] - w[0]) / quantum - 1))
        .collect()
}

#[test]
fn test_connected_node_is_rendered_every_quantum() {
    // control: no control thread activity
    assert_eq!(skipped_quanta(0), []);

    let skipped = skipped_quanta(100);
    assert!(
        skipped.is_empty(),
        "the recorder was not rendered in {} quanta, e.g. (frame, quanta skipped): {:?}",
        skipped.len(),
        &skipped[..skipped.len().min(5)]
    );
}
