use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use serde::Serialize;

#[cfg(not(target_arch = "wasm32"))]
pub mod bench_support;
pub mod fixtures;
#[cfg(not(target_arch = "wasm32"))]
pub mod harness;
pub mod scenario;

static ENABLED: AtomicBool = AtomicBool::new(false);
static SESSION: OnceLock<Mutex<ProfilingSession>> = OnceLock::new();

#[derive(Debug, Clone, Copy, Default)]
pub struct ProfilingConfig {
    pub enabled: bool,
    pub capture_after_frame: u64,
    pub capture_trace: bool,
}

#[derive(Debug, Clone)]
pub struct ReportMetadata {
    pub scenario: String,
    pub bevy_version: String,
    pub git_revision: Option<String>,
    pub build_profile: String,
    pub frames: u64,
    pub warmup_frames: u64,
    pub wall_clock_ms: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileSummary {
    pub scenario: String,
    pub bevy_version: String,
    pub git_revision: Option<String>,
    pub build_profile: String,
    pub frames: u64,
    pub warmup_frames: u64,
    pub wall_clock_ms: f64,
    pub frame_time_ms: QuantileSummary,
    pub named_spans: HashMap<String, SpanSummary>,
    pub counters: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct QuantileSummary {
    pub avg: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
    pub max: f64,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct SpanSummary {
    pub count: usize,
    pub avg_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub max_ms: f64,
    pub total_ms: f64,
}

#[derive(Debug, Clone, Serialize)]
struct TraceFile {
    #[serde(rename = "traceEvents")]
    trace_events: Vec<TraceEvent>,
}

#[derive(Debug, Clone, Serialize)]
struct TraceEvent {
    name: String,
    cat: &'static str,
    ph: &'static str,
    ts: f64,
    dur: f64,
    pid: u32,
    tid: u32,
}

pub fn reset(config: ProfilingConfig) {
    ENABLED.store(config.enabled, Ordering::Relaxed);
    let session = SESSION.get_or_init(|| Mutex::new(ProfilingSession::default()));
    let mut guard = session.lock().unwrap();
    *guard = ProfilingSession::new(config);
}

pub fn begin_frame(frame: u64) {
    with_session(|session| session.begin_frame(frame));
}

pub fn end_frame(frame: u64) {
    with_session(|session| session.end_frame(frame));
}

pub fn add_counter(name: &'static str, value: f64) {
    with_session(|session| session.add_counter(name, value));
}

pub fn set_gauge(name: impl Into<String>, value: f64) {
    let name = name.into();
    with_session(|session| session.set_gauge(name, value));
}

pub fn set_last(name: impl Into<String>, value: f64) {
    let name = name.into();
    with_session(|session| session.set_last(name, value));
}

pub fn scope(name: &'static str) -> ProfilingScope {
    if !ENABLED.load(Ordering::Relaxed) {
        return ProfilingScope::disabled();
    }
    let start_micros = SESSION
        .get()
        .and_then(|session| {
            session.lock().ok().and_then(|guard| {
                guard
                    .started_at
                    .map(|started| started.elapsed().as_secs_f64() * 1_000_000.0)
            })
        })
        .unwrap_or_default();
    ProfilingScope {
        name,
        started_at: Some(Instant::now()),
        start_micros,
    }
}

pub fn report(metadata: ReportMetadata) -> ProfileSummary {
    let Some(session) = SESSION.get() else {
        return ProfileSummary {
            scenario: metadata.scenario,
            bevy_version: metadata.bevy_version,
            git_revision: metadata.git_revision,
            build_profile: metadata.build_profile,
            frames: metadata.frames,
            warmup_frames: metadata.warmup_frames,
            wall_clock_ms: metadata.wall_clock_ms,
            frame_time_ms: QuantileSummary::default(),
            named_spans: HashMap::new(),
            counters: HashMap::new(),
        };
    };
    session.lock().unwrap().report(metadata)
}

pub fn write_trace(path: &std::path::Path) -> Result<()> {
    let Some(session) = SESSION.get() else {
        return Ok(());
    };
    let guard = session.lock().unwrap();
    if guard.trace_events.is_empty() {
        return Ok(());
    }
    let bytes = serde_json::to_vec_pretty(&TraceFile {
        trace_events: guard.trace_events.clone(),
    })
    .context("encode chrome trace")?;
    std::fs::write(path, bytes)
        .with_context(|| format!("write chrome trace {}", path.display()))?;
    Ok(())
}

pub struct ProfilingScope {
    name: &'static str,
    started_at: Option<Instant>,
    start_micros: f64,
}

impl ProfilingScope {
    fn disabled() -> Self {
        Self {
            name: "",
            started_at: None,
            start_micros: 0.0,
        }
    }
}

impl Drop for ProfilingScope {
    fn drop(&mut self) {
        let Some(started_at) = self.started_at else {
            return;
        };
        with_session(|session| {
            session.record_span(self.name, self.start_micros, started_at.elapsed())
        });
    }
}

#[derive(Debug, Default)]
struct ProfilingSession {
    config: ProfilingConfig,
    current_frame: u64,
    started_at: Option<Instant>,
    frame_started_at: Option<Instant>,
    frame_samples_ms: Vec<f64>,
    span_samples_ms: HashMap<&'static str, Vec<f64>>,
    counter_totals: HashMap<&'static str, f64>,
    gauge_totals: HashMap<String, (f64, u64)>,
    last_values: HashMap<String, f64>,
    trace_events: Vec<TraceEvent>,
}

impl ProfilingSession {
    fn new(config: ProfilingConfig) -> Self {
        Self {
            config,
            current_frame: 0,
            started_at: Some(Instant::now()),
            frame_started_at: None,
            frame_samples_ms: Vec::new(),
            span_samples_ms: HashMap::new(),
            counter_totals: HashMap::new(),
            gauge_totals: HashMap::new(),
            last_values: HashMap::new(),
            trace_events: Vec::new(),
        }
    }

    fn begin_frame(&mut self, frame: u64) {
        self.current_frame = frame;
        self.frame_started_at = Some(Instant::now());
    }

    fn end_frame(&mut self, frame: u64) {
        if !self.should_capture(frame) {
            self.frame_started_at = None;
            return;
        }
        if let Some(started_at) = self.frame_started_at.take() {
            self.frame_samples_ms
                .push(started_at.elapsed().as_secs_f64() * 1000.0);
        }
    }

    fn add_counter(&mut self, name: &'static str, value: f64) {
        if !self.should_capture(self.current_frame) {
            return;
        }
        *self.counter_totals.entry(name).or_default() += value;
    }

    fn set_gauge(&mut self, name: String, value: f64) {
        if !self.should_capture(self.current_frame) {
            return;
        }
        let entry = self.gauge_totals.entry(name).or_default();
        entry.0 += value;
        entry.1 = entry.1.saturating_add(1);
    }

    fn set_last(&mut self, name: String, value: f64) {
        if !self.should_capture(self.current_frame) {
            return;
        }
        self.last_values.insert(name, value);
    }

    fn record_span(&mut self, name: &'static str, start_micros: f64, duration: Duration) {
        if !self.should_capture(self.current_frame) {
            return;
        }
        let ms = duration.as_secs_f64() * 1000.0;
        self.span_samples_ms.entry(name).or_default().push(ms);
        if self.config.capture_trace {
            self.trace_events.push(TraceEvent {
                name: name.to_string(),
                cat: "perf",
                ph: "X",
                ts: start_micros,
                dur: duration.as_secs_f64() * 1_000_000.0,
                pid: 1,
                tid: 1,
            });
        }
    }

    fn should_capture(&self, frame: u64) -> bool {
        self.config.enabled && frame >= self.config.capture_after_frame
    }

    fn report(&self, metadata: ReportMetadata) -> ProfileSummary {
        let mut named_spans = HashMap::new();
        for (name, samples) in &self.span_samples_ms {
            named_spans.insert((*name).to_string(), span_summary(samples));
        }

        let mut counters = HashMap::new();
        for (name, value) in &self.counter_totals {
            counters.insert((*name).to_string(), *value);
        }
        for (name, value) in &self.last_values {
            counters.insert(name.clone(), *value);
        }
        for (name, (sum, count)) in &self.gauge_totals {
            let avg = if *count == 0 {
                0.0
            } else {
                *sum / *count as f64
            };
            counters.insert(format!("{}_avg", name), avg);
        }

        ProfileSummary {
            scenario: metadata.scenario,
            bevy_version: metadata.bevy_version,
            git_revision: metadata.git_revision,
            build_profile: metadata.build_profile,
            frames: metadata.frames,
            warmup_frames: metadata.warmup_frames,
            wall_clock_ms: metadata.wall_clock_ms,
            frame_time_ms: quantiles(&self.frame_samples_ms),
            named_spans,
            counters,
        }
    }
}

fn with_session(f: impl FnOnce(&mut ProfilingSession)) {
    if !ENABLED.load(Ordering::Relaxed) {
        return;
    }
    let Some(session) = SESSION.get() else {
        return;
    };
    let mut guard = session.lock().unwrap();
    f(&mut guard);
}

fn span_summary(samples: &[f64]) -> SpanSummary {
    let q = quantiles(samples);
    SpanSummary {
        count: samples.len(),
        avg_ms: q.avg,
        p50_ms: q.p50,
        p95_ms: q.p95,
        p99_ms: q.p99,
        max_ms: q.max,
        total_ms: samples.iter().copied().sum(),
    }
}

fn quantiles(samples: &[f64]) -> QuantileSummary {
    if samples.is_empty() {
        return QuantileSummary::default();
    }
    let mut sorted = samples.to_vec();
    sorted.sort_by(|left, right| left.total_cmp(right));
    QuantileSummary {
        avg: sorted.iter().copied().sum::<f64>() / sorted.len() as f64,
        p50: percentile(&sorted, 0.50),
        p95: percentile(&sorted, 0.95),
        p99: percentile(&sorted, 0.99),
        max: *sorted.last().unwrap_or(&0.0),
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

#[macro_export]
macro_rules! perf_scope {
    ($name:expr) => {
        let _profiling_scope = $crate::profiling::scope($name);
    };
}

#[macro_export]
macro_rules! perf_counter_add {
    ($name:expr, $value:expr) => {
        $crate::profiling::add_counter($name, $value as f64);
    };
}

#[macro_export]
macro_rules! perf_gauge {
    ($name:expr, $value:expr) => {
        $crate::profiling::set_gauge($name, $value as f64);
    };
}

#[macro_export]
macro_rules! perf_last {
    ($name:expr, $value:expr) => {
        $crate::profiling::set_last($name, $value as f64);
    };
}
