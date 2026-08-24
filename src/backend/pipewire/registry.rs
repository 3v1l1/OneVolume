use std::rc::Rc;
use std::sync::mpsc::Sender;

use pipewire::{core::CoreRc, registry::RegistryRc, types::ObjectType};

use libspa::param::ParamType;

use super::capture;
use super::runtime::SharedRuntime;
use crate::backend::detector::Detector;
use crate::backend::events::PipeWireEvent;

pub fn attach(
    runtime: SharedRuntime,
    registry: &RegistryRc,
    core: &CoreRc,
    sender: Sender<PipeWireEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    let registry_clone = registry.clone();
    let runtime_clone = runtime.clone();
    let runtime_clone_for_remove = runtime.clone();
    let core_clone = core.clone();

    let listener = registry
        .add_listener_local()
        .global(move |global| {
            let runtime = runtime_clone.clone();

            if global.type_ != ObjectType::Node {
                return;
            }

            let props = match global.props.as_ref() {
                Some(props) => props,
                // A node with no properties at all can't be an audio
                // stream we care about (no media.class to check).
                None => return,
            };

            let media_class = props.get("media.class").unwrap_or("");

            // Step 2: ignore everything that isn't an app's audio
            // playback stream — this filters out the dummy/freewheel
            // drivers, the webcam, mic/speaker sinks, MIDI bridges,
            // etc. before we ever bind or print anything about them.
            if !Detector::is_audio_playback_stream(media_class) {
                return;
            }

            // Prefer application.name (what browsers/media players set
            // for their playback stream); fall back to node.name for
            // apps that don't set it.
            let app_name = props
                .get("application.name")
                .or_else(|| props.get("node.name"))
                .unwrap_or("");

            let node_id = global.id;
            let is_supported = Detector::is_supported(app_name);

            // Safety dedup: if this exact node id is already tracked
            // (its `global` callback shouldn't normally fire twice,
            // but this makes it impossible to accidentally double-bind
            // regardless), skip it.
            if runtime.borrow().has_stream(node_id) {
                return;
            }

            if is_supported {
                println!();
                println!("🎬 Supported application detected!");
                println!();
                println!("  {app_name}");
                println!();
            } else {
                // Some other app's audio stream — not one we're set up
                // to level yet. Log it quietly so it's easy to spot
                // which apps to add to config::SUPPORTED_APPS.
                println!("Audio stream from \"{app_name}\" (not on the supported list yet)");
                return;
            }

            use pipewire::node::Node;

            let node: Node = match registry_clone.bind(global) {
                Ok(node) => node,
                Err(err) => {
                    eprintln!("Bind failed: {err}");
                    return;
                }
            };

            // Seed the ActiveStream entry right away so the UI has
            // something to show even before the first info callback
            // arrives.
            runtime.borrow_mut().upsert_active_stream(
                node_id,
                app_name.to_string(),
                "Unknown".to_string(),
            );

            let runtime_for_info = runtime.clone();
            let core_for_info = core_clone.clone();
            let sender_for_info = sender.clone();
            let app_name_for_info = app_name.to_string();

            let listener = node
                .add_listener_local()
                .info(move |info| {
                    let new_state = format!("{:?}", info.state());

                    // PipeWire re-fires info() every time the node's
                    // volume changes — which, once the leveler is
                    // running, can be ~20x/sec. Only print when the
                    // *state* (Suspended/Running/Idle) actually
                    // changes, so real transitions are visible instead
                    // of drowned in noise. ActiveStream is still kept
                    // fully up to date either way.
                    let previous_state = runtime_for_info.borrow().active_stream_state(node_id);
                    if previous_state.as_deref() != Some(new_state.as_str()) {
                        super::node::print_info(info);
                    }

                    runtime_for_info.borrow_mut().upsert_active_stream(
                        node_id,
                        app_name_for_info.clone(),
                        new_state,
                    );

                    // THE ACTUAL FIX: base the stop/resume decision on
                    // whether anything is really "Running" right now
                    // (same thing the "N node(s) receiving it" counter
                    // already reads), not on whether the node still
                    // exists at all. A paused/idle app stays registered
                    // — this is what catches that case, instead of
                    // only global_remove (which only fires when the
                    // node is fully destroyed).
                    let anything_running =
                        !runtime_for_info.borrow().running_leveled_nodes().is_empty();

                    if anything_running {
                        if let Err(err) = capture::ensure_capture_running(
                            &core_for_info,
                            runtime_for_info.clone(),
                            sender_for_info.clone(),
                        ) {
                            eprintln!("Failed to start/resume capture: {err}");
                        }
                    } else {
                        capture::stop_capture(&runtime_for_info);
                    }
                })
                .param(|_seq, _id, _index, _next, _| {})
                .register();

            node.subscribe_params(&[ParamType::Props]);
            node.enum_params(1, Some(ParamType::Props), 0, u32::MAX);

            // The info/param listener stays alive for the life of the
            // program (leaked, same as the stream in capture.rs).
            std::mem::forget(listener);

            let node_rc = Rc::new(node);

            // Register this node so the (single, global) capture
            // stream's process() callback starts writing computed
            // gain to it whenever it's "Running".
            runtime.borrow_mut().add_leveled_node(node_id, node_rc);

            // Create the capture stream on first use, or reconnect it
            // if it had been stopped (last node previously removed).
            // No-ops if it's already running.
            if let Err(err) =
                capture::ensure_capture_running(&core_clone, runtime.clone(), sender.clone())
            {
                eprintln!("Failed to start/resume capture: {err}");
            }
        })
        .global_remove(move |id| {
            let mut rt = runtime_clone_for_remove.borrow_mut();
            rt.stop_leveling(id);
            rt.remove_active_stream(id);

            // Real lifecycle fix: when the last supported node
            // disappears, actually stop the capture stream
            // (Stream::disconnect()) instead of leaving it running
            // against nothing. It stays creatable-again via
            // ensure_capture_running the next time a supported app
            // starts playing.
            if rt.leveled_node_count() == 0 {
                drop(rt);
                capture::stop_capture(&runtime_clone_for_remove);
            }
        })
        .register();

    std::mem::forget(listener);

    Ok(())
}
