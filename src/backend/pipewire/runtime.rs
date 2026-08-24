use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::mpsc::Receiver};

use pipewire::node::Node;
use pipewire::stream::StreamRc;

use crate::backend::events::UiCommand;

pub type SharedRuntime = Rc<RefCell<Runtime>>;

/// A supported app's audio stream, tracked purely as data — no
/// PipeWire types in here at all. This is what the UI (or diagnostics,
/// for now) should read from, instead of re-querying PipeWire or
/// parsing println! output.
#[derive(Debug, Clone)]
pub struct ActiveStream {
    /// Not read directly yet — callers already have the id from the
    /// HashMap key when iterating internally. Kept for the planned
    /// "list all active streams" UI view (via `active_streams()`),
    /// where a caller holding an owned `ActiveStream` outside the map
    /// would need it.
    #[allow(dead_code)]
    pub id: u32,
    pub application: String,
    pub state: String,
}

pub struct Runtime {
    /// Nodes we're actively leveling, keyed by node id, so the shared
    /// capture callback can reach every one of them to write volume
    /// back on. Multiple supported apps/streams all end up in here —
    /// they all share the ONE capture stream below, since it measures
    /// the whole speaker output anyway, not any single app's audio.
    leveled_nodes: HashMap<u32, Rc<Node>>,

    /// The one global capture stream, once it's been created. A real,
    /// cloneable `StreamRc` (not leaked) — stopping/resuming it means
    /// calling `.disconnect()`/`.connect()` on this, not tearing down
    /// and rebuilding.
    capture_stream: Option<StreamRc>,

    /// Set when capture stops (0 nodes actually running) — checked and
    /// cleared by the capture process() callback on its next buffer,
    /// which calls `Leveler::reset()` so gain snaps back to neutral
    /// instead of drifting on quiet background noise with nothing to
    /// apply it to.
    leveler_reset_requested: bool,

    /// Supported apps currently playing audio, keyed by node id.
    /// Inserted when a supported app's stream is first seen, updated
    /// on every state change, removed when the stream/app closes.
    active_streams: HashMap<u32, ActiveStream>,

    /// Held until `start_global_capture` claims it (via
    /// `take_command_receiver`) the first time capture actually
    /// starts. `Option` because `Receiver` isn't `Clone` and this can
    /// only be taken once — matches the "capture starts exactly once"
    /// dedup already in place for the stream itself.
    command_receiver: Option<Receiver<UiCommand>>,
}

impl Runtime {
    pub fn new(command_receiver: Receiver<UiCommand>) -> SharedRuntime {
        Rc::new(RefCell::new(Self {
            leveled_nodes: HashMap::new(),
            capture_stream: None,
            leveler_reset_requested: false,
            active_streams: HashMap::new(),
            command_receiver: Some(command_receiver),
        }))
    }

    pub fn take_command_receiver(&mut self) -> Option<Receiver<UiCommand>> {
        self.command_receiver.take()
    }

    pub fn request_leveler_reset(&mut self) {
        self.leveler_reset_requested = true;
    }

    pub fn take_leveler_reset_request(&mut self) -> bool {
        std::mem::replace(&mut self.leveler_reset_requested, false)
    }

    pub fn add_leveled_node(&mut self, id: u32, node: Rc<Node>) {
        self.leveled_nodes.insert(id, node);
    }

    pub fn stop_leveling(&mut self, id: u32) {
        self.leveled_nodes.remove(&id);
    }

    pub fn leveled_node_count(&self) -> usize {
        self.leveled_nodes.len()
    }

    /// Every node that's both registered for leveling AND currently
    /// in the "Running" state — i.e. the set of nodes that should
    /// actually receive the computed gain right now. A node that's
    /// registered but Suspended/Idle (paused) is skipped, without
    /// needing a separate per-node pause flag: its own ActiveStream
    /// state already tells us that.
    pub fn running_leveled_nodes(&self) -> Vec<(u32, Rc<Node>)> {
        self.leveled_nodes
            .iter()
            .filter(|(id, _)| {
                self.active_streams
                    .get(id)
                    .map(|s| s.state == "Running")
                    .unwrap_or(false)
            })
            .map(|(id, node)| (*id, node.clone()))
            .collect()
    }

    /// Name of a currently-"Running" supported app, if any — for the
    /// UI's "Watching: ___" display. Picks the first match if more
    /// than one app happens to be playing at once; good enough for a
    /// single-line "what's playing" summary.
    pub fn current_running_app_name(&self) -> Option<String> {
        self.active_streams
            .values()
            .find(|s| s.state == "Running")
            .map(|s| s.application.clone())
    }

    pub fn set_capture_stream(&mut self, stream: StreamRc) {
        self.capture_stream = Some(stream);
    }

    pub fn capture_stream(&self) -> Option<StreamRc> {
        self.capture_stream.clone()
    }

    /// Insert (first time seen) or update (state change) a stream entry.
    pub fn upsert_active_stream(&mut self, id: u32, application: String, state: String) {
        self.active_streams
            .entry(id)
            .and_modify(|s| s.state = state.clone())
            .or_insert(ActiveStream {
                id,
                application,
                state,
            });
    }

    pub fn remove_active_stream(&mut self, id: u32) {
        self.active_streams.remove(&id);
    }

    /// Everything the UI needs to show "what's playing right now" — no
    /// PipeWire query required. Not called yet (the single-app
    /// `current_running_app_name` covers today's UI mockup), but kept
    /// for the planned "list all active streams" view rather than
    /// deleted — see roadmap item for multi-app display.
    #[allow(dead_code)]
    pub fn active_streams(&self) -> Vec<ActiveStream> {
        self.active_streams.values().cloned().collect()
    }

    pub fn has_stream(&self, id: u32) -> bool {
        self.active_streams.contains_key(&id)
    }

    /// Just the state string for one stream, if we're tracking it — used
    /// to detect a real change vs. PipeWire re-notifying the same state.
    pub fn active_stream_state(&self, id: u32) -> Option<String> {
        self.active_streams.get(&id).map(|s| s.state.clone())
    }
}
