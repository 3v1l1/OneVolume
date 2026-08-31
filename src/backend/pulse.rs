use std::sync::{
    Arc, Mutex,
    mpsc::{self, Receiver, Sender},
};
use std::thread;
use std::time::Duration;

use libpulse_binding as pulse;
use pulse::{
    callbacks::ListResult,
    context::{Context, FlagSet, State},
    mainloop::standard::Mainloop,
    proplist::Proplist,
    volume::{ChannelVolumes, Volume, VolumeLinear},
};

#[derive(Debug, Clone)]
struct PulseCommand {
    app_name: String,
    gain_db: f32,
}

#[derive(Debug, Clone, Copy)]
struct AppStream {
    index: u32,
    baseline: Volume,
}

#[derive(Clone)]
pub struct PulseController {
    sender: Sender<PulseCommand>,
}

impl PulseController {
    pub fn start() -> Self {
        let (tx, rx) = mpsc::channel::<PulseCommand>();

        thread::Builder::new()
            .name("onevolume-pulse".to_string())
            .spawn(move || run_controller(rx))
            .expect("failed to spawn OneVolume Pulse controller");

        Self { sender: tx }
    }

    pub fn set_gain_db(&self, app_name: &str, gain_db: f32) {
        println!("📤 PulseController::set_gain_db(app={app_name:?}, gain={gain_db:+.1} dB)");

        let _ = self.sender.send(PulseCommand {
            app_name: app_name.to_string(),
            gain_db,
        });
    }
}

fn run_controller(rx: Receiver<PulseCommand>) {
    let Some(mut mainloop) = Mainloop::new() else {
        eprintln!("⚠️ Pulse: failed to create mainloop");
        return;
    };

    let Some(mut proplist) = Proplist::new() else {
        eprintln!("⚠️ Pulse: failed to create proplist");
        return;
    };

    let _ = proplist.set_str(pulse::proplist::properties::APPLICATION_NAME, "OneVolume");

    let Some(mut context) = Context::new_with_proplist(&mainloop, "OneVolume", &proplist) else {
        eprintln!("⚠️ Pulse: failed to create context");
        return;
    };

    if let Err(err) = context.connect(None, FlagSet::NOFLAGS, None) {
        eprintln!("⚠️ Pulse: connection failed: {err}");
        return;
    }

    while context.get_state() != State::Ready {
        match context.get_state() {
            State::Failed | State::Terminated => {
                eprintln!("⚠️ Pulse: context failed");
                return;
            }
            _ => {}
        }

        if mainloop.iterate(true).is_error() {
            eprintln!("⚠️ Pulse: mainloop failed while connecting");
            return;
        }
    }

    println!("🔊 Pulse controller ready");

    let mut target_app: Option<String> = None;
    let mut target_stream: Option<AppStream> = None;
    let mut pending_gain: Option<f32> = None;
    let mut last_sent_gain: Option<f32> = None;

    loop {
        // Keep only the newest requested command.
        while let Ok(command) = rx.try_recv() {
            let target_changed = target_app.as_deref() != Some(command.app_name.as_str());

            if target_changed {
                println!("🎯 Pulse: target application → {:?}", command.app_name);

                target_app = Some(command.app_name.clone());
                target_stream = None;
                last_sent_gain = None;
            }

            pending_gain = Some(command.gain_db);
        }

        // Find the sink-input belonging to the currently active supported app.
        if target_stream.is_none()
            && let Some(app_name) = target_app.clone()
            && pending_gain.is_some()
        {
            println!("🔎 Pulse: searching sink-input for {:?}...", app_name);

            let found = Arc::new(Mutex::new(None::<AppStream>));
            let found_cb = Arc::clone(&found);

            let target_name = app_name.clone();

            let _op = context
                .introspect()
                .get_sink_input_info_list(move |result| {
                    let ListResult::Item(info) = result else {
                        return;
                    };

                    let application_name = info
                        .proplist
                        .get_str(pulse::proplist::properties::APPLICATION_NAME)
                        .unwrap_or_default();

                    let node_name = info.proplist.get_str("node.name").unwrap_or_default();

                    let application_id = info
                        .proplist
                        .get_str(pulse::proplist::properties::APPLICATION_ID)
                        .unwrap_or_default();

                    let process_binary = info
                        .proplist
                        .get_str(pulse::proplist::properties::APPLICATION_PROCESS_BINARY)
                        .unwrap_or_default();

                    if !matches_supported_target(
                        &target_name,
                        &application_name,
                        &node_name,
                        &application_id,
                        &process_binary,
                    ) {
                        return;
                    }

                    let baseline_raw = info
                        .volume
                        .as_ref()
                        .values
                        .first()
                        .copied()
                        .unwrap_or(Volume::NORMAL.0);

                    let mut slot = found_cb.lock().unwrap();

                    if slot.is_none() {
                        *slot = Some(AppStream {
                            index: info.index,
                            baseline: Volume(baseline_raw),
                        });

                        println!(
                            "✅ Pulse: found {:?} sink-input {}",
                            target_name, info.index
                        );
                    }
                });

            // Pump PulseAudio until the asynchronous enumeration
            // has had a chance to return a matching stream.
            for _ in 0..100 {
                if mainloop.iterate(false).is_error() {
                    eprintln!("⚠️ Pulse: mainloop failed during discovery");
                    return;
                }

                if found.lock().unwrap().is_some() {
                    break;
                }

                thread::sleep(Duration::from_millis(5));
            }

            target_stream = found.lock().unwrap().take();

            if let Some(stream) = target_stream {
                println!("🎯 Pulse: using {:?} sink-input {}", app_name, stream.index);
            } else {
                println!("ℹ️ Pulse: no sink-input found for {:?}", app_name);
            }
        }
        // changed meaningfully.
        if let (Some(stream), Some(gain_db)) = (target_stream, pending_gain.take()) {
            let gain_db = gain_db.clamp(-30.0, 6.0);

            let changed = last_sent_gain
                .map(|last| (gain_db - last).abs() >= 0.05)
                .unwrap_or(true);

            if changed {
                let linear = 10.0_f64.powf(f64::from(gain_db) / 20.0);

                let gain_volume = Volume::from(VolumeLinear(linear));

                let target_volume = Volume::multiply(stream.baseline, gain_volume);

                let mut volumes = ChannelVolumes::default();
                volumes.set(2, target_volume);

                let index = stream.index;

                println!(
                    "🔊 Pulse: requesting {:?} sink-input {} → {:+.1} dB",
                    target_app.as_deref().unwrap_or("unknown"),
                    index,
                    gain_db
                );

                let _op = context.introspect().set_sink_input_volume(
                    index,
                    &volumes,
                    Some(Box::new(move |ok| {
                        println!("🔊 Pulse: sink-input {} update success={ok}", index);
                    })),
                );

                last_sent_gain = Some(gain_db);
            }
        }

        if mainloop.iterate(false).is_error() {
            eprintln!("⚠️ Pulse: mainloop failed");
            break;
        }

        thread::sleep(Duration::from_millis(5));
    }
}

fn matches_supported_target(
    target: &str,
    application_name: &str,
    node_name: &str,
    application_id: &str,
    process_binary: &str,
) -> bool {
    let target = target.trim().to_ascii_lowercase();

    let values = [application_name, node_name, application_id, process_binary];

    // VLC exposes versioned runtime names such as:
    // "VLC media player (LibVLC 3.0.23)".
    if target == "vlc media player" || target.starts_with("vlc media player (") {
        return values.iter().any(|value| {
            let value = value.to_ascii_lowercase();
            value == "vlc media player"
                || value.starts_with("vlc media player (")
                || value == "org.videolan.vlc"
        });
    }

    values
        .iter()
        .any(|value| value.eq_ignore_ascii_case(&target))
}
