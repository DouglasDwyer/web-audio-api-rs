//! Resource cleanup of `DelayNode`.
//!
//! Regression tests that a non-cyclic delay subgraph is disposed once every control handle is
//! dropped, while a delay inside a feedback cycle keeps rendering its tail.

use web_audio_api::context::{BaseAudioContext, OfflineAudioContext};
use web_audio_api::node::{AudioNode, AudioScheduledSourceNode};

/// A plain delay connected to the destination, then fully dropped, must still emit the delayed copy
/// of its finite input.
#[test]
fn plain_delay_fire_and_forget_still_delays() {
    let sample_rate = 48_000.0_f32;
    let length = sample_rate as usize;
    let mut context = OfflineAudioContext::new(1, length, sample_rate);

    {
        let mut src = context.create_constant_source();
        src.offset().set_value(1.0);

        let delay = context.create_delay(1.0);
        delay.delay_time().set_value(0.15);

        src.connect(&delay);
        delay.connect(&context.destination());

        src.start_at(0.0);
        src.stop_at(0.02);
    }

    let out = context.start_rendering_sync();
    let ch = out.get_channel_data(0);

    // a 20 ms pulse delayed by 150 ms: silence at 100 ms, the pulse at 160 ms
    assert!(
        ch[(0.10 * sample_rate) as usize].abs() < 1e-6,
        "output should be silent before the delay time"
    );
    assert!(
        ch[(0.16 * sample_rate) as usize].abs() > 0.5,
        "delayed signal missing"
    );
}

/// A delay in a feedback cycle keeps rendering its decaying echo tail after every handle is
/// dropped. Its reader flags the writer as breaking a cycle, so the writer reports side effects and
/// is not reclaimed while the loop is still ringing.
#[test]
fn feedback_delay_fire_and_forget_keeps_echoing() {
    let sample_rate = 48_000.0_f32;
    let length = sample_rate as usize; // 1 second
    let mut context = OfflineAudioContext::new(1, length, sample_rate);

    {
        // 20 ms DC pulse
        let mut src = context.create_constant_source();
        src.offset().set_value(1.0);

        let delay = context.create_delay(1.0);
        delay.delay_time().set_value(0.15);

        let feedback = context.create_gain();
        feedback.gain().set_value(0.6);

        //  src --> delay(writer)
        //          delay(reader) --> feedback --> delay(writer)   (cycle)
        //          delay(reader) --> destination
        src.connect(&delay);
        delay.connect(&feedback);
        feedback.connect(&delay);
        delay.connect(&context.destination());

        src.start_at(0.0);
        src.stop_at(0.02);

        // fire and forget: every handle drops here
    }

    let out = context.start_rendering_sync();
    let ch = out.get_channel_data(0);

    // the pulse is repeated every 150 ms, each repeat scaled by the 0.6 feedback gain
    let echo_early = ch[(0.16 * sample_rate) as usize].abs(); // direct delayed pulse (~1.0)
    let echo_mid = ch[(0.31 * sample_rate) as usize].abs(); // 1 feedback repeat  (~0.6)
    let echo_late = ch[(0.61 * sample_rate) as usize].abs(); // 3 feedback repeats (~0.216)

    assert!(echo_early > 0.5, "delayed signal missing");
    assert!(
        echo_late > 0.05,
        "feedback echoes died out - the writer was reclaimed"
    );
    assert!(
        echo_mid < echo_early && echo_late < echo_mid,
        "echo train is not decaying"
    );
}

/// A non-cyclic delay whose reader output is connected to nothing must be reclaimed from the render
/// graph once its handles are dropped - even while a source is still feeding its writer. Before the
/// fix, the writer lingered forever because it is registered as a cycle breaker.
#[cfg(feature = "diagnostics")]
#[test]
fn non_cyclic_delay_with_unused_output_is_disposed() {
    use std::sync::mpsc;
    use std::time::Duration;
    use web_audio_api::context::{AudioContext, AudioContextOptions};

    /// Poll the render graph for its current node count. The 'none' backend renders in real time,
    /// so callers sleep between snapshots to let the lifecycle collector run.
    fn node_count(context: &AudioContext) -> usize {
        let (tx, rx) = mpsc::channel();
        context.run_diagnostics(move |d| {
            let _ = tx.send(d);
        });
        rx.recv_timeout(Duration::from_secs(1))
            .expect("timed out waiting for diagnostics")
            .graph
            .node_count
    }

    let context = AudioContext::new(AudioContextOptions {
        sink_id: "none".into(),
        ..AudioContextOptions::default()
    });
    std::thread::sleep(Duration::from_millis(150));
    let baseline = node_count(&context);

    {
        let mut src = context.create_constant_source();
        src.offset().set_value(1.0);

        let delay = context.create_delay(0.1);
        delay.delay_time().set_value(0.05);

        src.connect(&delay);
        // NB. the delay output is deliberately left unconnected
        src.start(); // no stop: keeps feeding the writer

        std::thread::sleep(Duration::from_millis(100));
        assert!(
            node_count(&context) > baseline,
            "delay subgraph was never registered"
        );

        // fire and forget: drop the source and delay handles
    }

    std::thread::sleep(Duration::from_millis(600));

    assert_eq!(
        node_count(&context),
        baseline,
        "the unused non-cyclic delay subgraph was not reclaimed after its handles were dropped"
    );
}
