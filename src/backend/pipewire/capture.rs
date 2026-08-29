//! Connects the Leveler brain to a real PipeWire monitor stream.

use std::sync::mpsc::Sender;
use std::time::Instant;

use pipewire::{
    keys,
    properties::properties,
    stream::{StreamFlags, StreamRc},
};

use libspa::param::ParamType;
use libspa::param::audio::{AudioFormat, AudioInfoRaw};
use libspa::pod::serialize::PodSerializer;
use libspa::pod::{Object, Pod, Value};
use libspa::utils::Direction;

use super::runtime::SharedRuntime;
use crate::backend::events::{LiveState, PipeWireEvent, UiCommand};
use crate::backend::leveler::{Leveler, LevelerConfig, db_to_linear, peak_dbfs};
use crate::backend::peak_limiter::{PeakLimiter, PeakLimiterConfig};

/// Build the audio format pod and connect the capture stream.
fn connect_stream(stream: &StreamRc) -> Result<(), Box<dyn std::error::Error>> {
    let mut audio_info = AudioInfoRaw::new();

    audio_info.set_format(AudioFormat::F32LE);
    audio_info.set_channels(2);

    let format_obj = Object {
        type_: libspa::utils::SpaTypes::ObjectParamFormat.as_raw(),
        id: ParamType::EnumFormat.as_raw(),
        properties: audio_info.into(),
    };

    let bytes =
        PodSerializer::serialize(std::io::Cursor::new(Vec::new()), &Value::Object(format_obj))?
            .0
            .into_inner();

    let format_pod = Pod::from_bytes(&bytes).ok_or("failed to build format pod")?;

    let mut params = [format_pod];

    stream.connect(
        Direction::Input,
        None,
        StreamFlags::AUTOCONNECT | StreamFlags::MAP_BUFFERS,
        &mut params,
    )?;

    Ok(())
}

/// Create the global capture stream.
pub fn start_global_capture(
    core: &pipewire::core::CoreRc,
    runtime: SharedRuntime,
    sender: Sender<PipeWireEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🎧 Starting capture (first supported app appeared)");

    let leveler_config = LevelerConfig::default();

    // Hard safety ceiling.
    let max_output_multiplier = db_to_linear(leveler_config.max_boost_db);

    let dry_run = std::env::var("ONEVOLUME_DRY_RUN").is_ok();

    if dry_run {
        println!("🧪 DRY RUN MODE — computing gain but NOT touching real volume");
    }

    // Take the UI command receiver once.
    let command_receiver = runtime.borrow_mut().take_command_receiver();

    let mut enabled = true;

    let mut leveler = Leveler::new(leveler_config);
    let mut peak_limiter = PeakLimiter::new(PeakLimiterConfig::default());

    let mut last_tick = Instant::now();
    let mut last_apply = Instant::now();
    let mut last_debug_print = Instant::now();

    // Stage 1 loudness window: accumulate RMS energy for ~400 ms.
    let mut loudness_sum_sq = 0.0_f64;
    let mut loudness_samples = 0usize;
    let mut loudness_window_elapsed = 0.0_f32;
    let mut last_loudness_db = -100.0_f32;
    const LOUDNESS_WINDOW_SECONDS: f32 = 0.8;

    let mut printed_first_buffer = false;
    let mut session_peak_db = -100.0_f32;

    let mut props = properties! {
        *keys::MEDIA_TYPE => "Audio",
        *keys::MEDIA_CATEGORY => "Monitor",
        *keys::MEDIA_ROLE => "Movie",
    };

    // Capture the sink monitor — actual speaker output.
    props.insert("stream.capture.sink", "true");

    let stream = StreamRc::new(core.clone(), "onevolume-meter", props)?;

    let runtime_for_closure = runtime.clone();

    let _listener = stream
        .add_local_listener_with_user_data(())
        .param_changed(|_stream, _data, id, pod| {
            if id == ParamType::Format.as_raw()
                && let Some(pod) = pod
            {
                let mut info = AudioInfoRaw::new();

                match info.parse(pod) {
                    Ok(_) => {
                        println!(
                            "🔧 Negotiated format: {:?} | {} ch | {} Hz",
                            info.format(),
                            info.channels(),
                            info.rate()
                        );
                    }

                    Err(err) => {
                        println!("🔧 Format param changed but couldn't parse it: {err:?}");
                    }
                }
            }
        })
        .process(move |stream, _| {
            let Some(mut buffer) = stream.dequeue_buffer() else {
                return;
            };

            // Reset leveler after capture has been stopped.
            if runtime_for_closure
                .borrow_mut()
                .take_leveler_reset_request()
            {
                leveler.reset();
                peak_limiter.reset();
                session_peak_db = -100.0;
                loudness_sum_sq = 0.0;
                loudness_samples = 0;
                loudness_window_elapsed = 0.0;
                last_loudness_db = -100.0;

                println!("🎚️ Leveler reset to neutral gain");
            }

            // Process UI commands.
            if let Some(rx) = &command_receiver {
                while let Ok(command) = rx.try_recv() {
                    match command {
                        UiCommand::SetEnabled(new_enabled) => {
                            enabled = new_enabled;

                            // Always reset when toggling.
                            leveler.reset();
                            peak_limiter.reset();
                            loudness_sum_sq = 0.0;
                            loudness_samples = 0;
                            loudness_window_elapsed = 0.0;
                            last_loudness_db = -100.0;
                            session_peak_db = -100.0;

                            if enabled {
                                println!("▶️ OneVolume enabled — leveler reset to neutral");
                            } else {
                                println!("⏸️ OneVolume disabled — volume handed back to normal");
                            }
                        }
                    }
                }
            }

            let datas = buffer.datas_mut();

            if datas.is_empty() {
                return;
            }

            let mut peak_db = -100.0_f32;

            // PipeWire may give us more than one data block.
            for data in datas.iter_mut() {
                let chunk = data.chunk();

                let offset = chunk.offset() as usize;
                let size = chunk.size() as usize;

                if size == 0 {
                    continue;
                }

                let Some(bytes) = data.data() else {
                    continue;
                };

                if offset >= bytes.len() {
                    continue;
                }

                let end = (offset + size).min(bytes.len());

                let samples_bytes = &bytes[offset..end];

                let samples = bytemuck_cast_f32(samples_bytes);

                if samples.is_empty() {
                    continue;
                }

                // Accumulate RMS energy for the sustained loudness detector.
                // This produces a true time-windowed RMS instead of reacting
                // to whichever ~20 ms buffer happens to be loudest.
                let sum_sq: f64 = samples.iter().map(|s| (*s as f64) * (*s as f64)).sum();

                loudness_sum_sq += sum_sq;
                loudness_samples += samples.len();

                let peak = peak_dbfs(samples);
                peak_db = peak_db.max(peak);

                if !printed_first_buffer {
                    printed_first_buffer = true;

                    println!(
                        "📦 First buffer: {} bytes, {} f32 samples",
                        samples_bytes.len(),
                        samples.len()
                    );

                    println!(
                        "📦 First 8 sample values: {:?}",
                        &samples[..samples.len().min(8)]
                    );
                }
            }

            if loudness_samples == 0 && peak_db <= -99.9 {
                return;
            }

            if peak_db > session_peak_db {
                session_peak_db = peak_db;
            }

            let now = Instant::now();

            let dt = now
                .duration_since(last_tick)
                .as_secs_f32()
                .clamp(0.001, 0.1);

            last_tick = now;

            // Stage 1: sustained loudness control uses a ~400 ms
            // windowed RMS measurement.
            loudness_window_elapsed += dt;

            let window_ready = loudness_window_elapsed >= LOUDNESS_WINDOW_SECONDS;

            let windowed_level_db = if window_ready && loudness_samples > 0 {
                let mean_sq = loudness_sum_sq / loudness_samples as f64;
                let rms = mean_sq.sqrt();

                loudness_sum_sq = 0.0;
                loudness_samples = 0;
                loudness_window_elapsed = 0.0;

                last_loudness_db = if rms > 0.0 {
                    (20.0 * rms.log10() as f32).max(-100.0)
                } else {
                    -100.0
                };

                Some(last_loudness_db)
            } else {
                None
            };

            let loudness_gain_db = if let Some(level_db) = windowed_level_db {
                if enabled {
                    leveler.process(level_db, LOUDNESS_WINDOW_SECONDS)
                } else {
                    0.0
                }
            } else if enabled {
                leveler.current_gain_db()
            } else {
                0.0
            };

            // Stage 2: fast peak protection.
            // Feed the raw peak directly to the limiter so sudden blasts,
            // explosions, gunshots, and yelling can trigger protection
            // immediately without being delayed by the RMS leveler.
            let peak_gain_db = if enabled {
                peak_limiter.process(peak_db, dt)
            } else {
                0.0
            };

            // Stage 1 and Stage 2 operate independently; their dB corrections add.
            let gain_db = (loudness_gain_db + peak_gain_db).clamp(-30.0, 6.0);

            let running_nodes = runtime_for_closure.borrow().running_leveled_nodes();

            // UI/debug update once per second.
            if now.duration_since(last_debug_print).as_secs_f32() >= 1.0 {
                println!(
                    "🎧 Speaker meter: RMS {:.1} dB | Peak {:.1} dB \
                     (session max {:.1} dB) | loudness gain {:+.1} dB | \
                     peak gain {:+.1} dB | final {:+.1} dB | {} node(s) receiving it",
                    last_loudness_db,
                    peak_db,
                    session_peak_db,
                    loudness_gain_db,
                    peak_gain_db,
                    gain_db,
                    running_nodes.len()
                );

                last_debug_print = now;

                let current_app = runtime_for_closure.borrow().current_running_app_name();

                let state = LiveState {
                    capture_running: true,
                    current_app,
                    active_stream_count: running_nodes.len(),
                    loudness_db: last_loudness_db,
                    gain_db,
                    enabled,
                };

                let _ = sender.send(PipeWireEvent::StateUpdate(state));
            }

            // Apply volume roughly every 50ms.
            if now.duration_since(last_apply).as_secs_f32() >= 0.05 {
                last_apply = now;

                let multiplier = if enabled {
                    db_to_linear(gain_db).clamp(0.0, max_output_multiplier)
                } else {
                    1.0
                };

                if dry_run {
                    println!(
                        "🧪 DRY RUN: would set {:.0}% ({:+.1} dB) \
                         on {} node(s)",
                        multiplier * 100.0,
                        gain_db,
                        running_nodes.len()
                    );
                } else {
                    for (_id, node) in &running_nodes {
                        apply_volume(node, 2, multiplier);
                    }
                }
            }
        })
        .register();

    connect_stream(&stream)?;

    println!("🎧 Capture running");

    // Keep callbacks alive for the application lifetime.
    std::mem::forget(_listener);

    // Runtime owns the stream.
    runtime.borrow_mut().set_capture_stream(stream);

    Ok(())
}

/// Ensure the global capture stream exists and is connected.
pub fn ensure_capture_running(
    core: &pipewire::core::CoreRc,
    runtime: SharedRuntime,
    sender: Sender<PipeWireEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    let existing = runtime.borrow().capture_stream();

    match existing {
        None => start_global_capture(core, runtime, sender),

        Some(stream) => {
            use pipewire::stream::StreamState;

            if stream.state() == StreamState::Unconnected {
                println!(
                    "🎧 Resuming capture \
                     (a supported app started playing again)"
                );

                connect_stream(&stream)?;

                println!("🎧 Capture running");
            }

            Ok(())
        }
    }
}

/// Stop the global capture stream.
pub fn stop_capture(runtime: &SharedRuntime) {
    let stream_opt = runtime.borrow().capture_stream();

    if let Some(stream) = stream_opt {
        use pipewire::stream::StreamState;

        if stream.state() == StreamState::Unconnected {
            return;
        }

        println!("🎧 Stopping capture (no supported apps playing)");

        let _ = stream.disconnect();

        runtime.borrow_mut().request_leveler_reset();

        println!("🎧 Capture stopped");
    }
}

/// Apply a linear volume multiplier to a PipeWire node.
///
/// 1.0 = 100%
/// 0.5 = -6 dB
/// 2.0 = +6 dB
fn apply_volume(node: &pipewire::node::Node, channels: u32, multiplier: f32) {
    let volumes = libspa::pod::ValueArray::Float(vec![multiplier; channels as usize]);

    let props_obj = Object {
        type_: libspa::utils::SpaTypes::ObjectParamProps.as_raw(),
        id: ParamType::Props.as_raw(),

        properties: vec![libspa::pod::Property {
            key: libspa::sys::SPA_PROP_channelVolumes,

            flags: libspa::pod::PropertyFlags::empty(),

            value: Value::ValueArray(volumes),
        }],
    };

    let bytes =
        match PodSerializer::serialize(std::io::Cursor::new(Vec::new()), &Value::Object(props_obj))
        {
            Ok(result) => result.0.into_inner(),

            Err(err) => {
                eprintln!("⚠️ Failed to serialize volume pod: {err}");
                return;
            }
        };

    let Some(pod) = Pod::from_bytes(&bytes) else {
        eprintln!("⚠️ Failed to create PipeWire volume pod");
        return;
    };

    // IMPORTANT:
    //
    // pipewire-rs 0.9.2's Node::set_param() returns ().
    // It is NOT a Result, so do not use `if let Err(...)`.
    node.set_param(ParamType::Props, 0, pod);
}

/// Interpret PipeWire's F32LE audio buffer as f32 samples.
///
/// The project negotiates F32LE and runs on little-endian Linux.
fn bytemuck_cast_f32(bytes: &[u8]) -> &[f32] {
    let len = bytes.len() / std::mem::size_of::<f32>();

    unsafe { std::slice::from_raw_parts(bytes.as_ptr() as *const f32, len) }
}
