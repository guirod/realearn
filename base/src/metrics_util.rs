use crossbeam_channel::{Receiver, Sender};
use once_cell::sync::Lazy;
use std::thread;
use std::time::{Duration, Instant};

static METRICS_ENABLED: Lazy<bool> = Lazy::new(|| std::env::var("REALEARN_METRICS").is_ok());
static METRICS_CHANNEL: Lazy<MetricsChannel> = Lazy::new(Default::default);

/// A simple function that doesn't expose anything to the metrics endpoint but warns if a
/// threshold is exceeded. Doesn't do anything in release builds (except executing the function).
pub fn warn_if_takes_too_long<R>(label: &'static str, max: Duration, f: impl FnOnce() -> R) -> R {
    #[cfg(debug_assertions)]
    {
        let before = Instant::now();
        let r = f();
        let elapsed = before.elapsed();
        if elapsed > max {
            tracing_warn!(
                "Operation took too long: \"{label}\" ({})ms",
                elapsed.as_millis()
            );
        }
        r
    }
    #[cfg(not(debug_assertions))]
    {
        let _ = (label, max);
        f()
    }
}

pub fn metrics_are_enabled() -> bool {
    *METRICS_ENABLED
}

/// Initializes the metrics channel.  
pub fn init_metrics() {
    if !metrics_are_enabled() {
        return;
    }
    let _ = *METRICS_CHANNEL;
    // We record metrics async because we are mostly in real-time threads when recording metrics.
    // The metrics and metrics-exporter-prometheus crates sometimes do allocations. If this would
    // just provoke audio dropouts, then fine ... users shouldn't collect metrics anyway under
    // normal circumstances, in live scenarios certainly never! But it could also distort results.
    thread::Builder::new()
        .name(String::from("ReaLearn metrics"))
        .spawn(move || {
            keep_recording_metrics(METRICS_CHANNEL.receiver.clone());
        })
        .unwrap();
}

/// Synchronously records the occurrence of the given event.
pub fn record_occurrence(id: &'static str) {
    if !metrics_are_enabled() {
        return;
    }
    metrics::increment_counter!(id);
}

/// Asynchronously measures and records the time of the given operation and exposes it at the
/// metrics endpoint.
pub fn measure_time<R>(id: &'static str, f: impl FnOnce() -> R) -> R {
    if !metrics_are_enabled() {
        return f();
    }
    let start = Instant::now();
    let result = f();
    record_duration_internal(id, start.elapsed());
    result
}

/// Records the given duration into a histogram.
pub fn record_duration(id: &'static str, delta: Duration) {
    if !metrics_are_enabled() {
        return;
    }
    record_duration_internal(id, delta);
}

pub fn record_duration_internal(id: &'static str, delta: Duration) {
    let task = MetricsTask::Histogram { id, delta };
    if METRICS_CHANNEL.sender.try_send(task).is_err() {
        tracing::debug!("ReaLearn metrics channel is full");
    }
}

struct MetricsChannel {
    sender: Sender<MetricsTask>,
    receiver: Receiver<MetricsTask>,
}

impl Drop for MetricsChannel {
    fn drop(&mut self) {
        println!("Dropping ReaLearn MetricsChannel...");
    }
}

impl Default for MetricsChannel {
    fn default() -> Self {
        let (sender, receiver) = crossbeam_channel::bounded(5000);
        Self { sender, receiver }
    }
}

enum MetricsTask {
    Histogram { id: &'static str, delta: Duration },
}

fn keep_recording_metrics(receiver: Receiver<MetricsTask>) {
    while let Ok(task) = receiver.recv() {
        match task {
            MetricsTask::Histogram { id, delta } => {
                metrics::histogram!(id, delta);
            }
        }
    }
}
