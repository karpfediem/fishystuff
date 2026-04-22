#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

use anyhow::{Context, Result};
use serde::Serialize;

#[cfg(not(target_arch = "wasm32"))]
pub mod bench_support;
#[cfg(target_arch = "wasm32")]
pub mod browser;
pub mod fixtures;
#[cfg(not(target_arch = "wasm32"))]
pub mod harness;
pub mod scenario;

static ENABLED: AtomicBool = AtomicBool::new(false);
static CAPTURE_FRAMES: AtomicBool = AtomicBool::new(false);
static CAPTURE_SCOPES: AtomicBool = AtomicBool::new(false);
static CURRENT_FRAME: AtomicU64 = AtomicU64::new(0);
static SESSION: OnceLock<Mutex<ProfilingSession>> = OnceLock::new();
#[cfg(target_arch = "wasm32")]
thread_local! {
    static LIVE_PROFILE_STATE: RefCell<LiveProfileState> =
        RefCell::new(LiveProfileState::default());
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ProfilingConfig {
    pub enabled: bool,
    pub capture_after_frame: u64,
    pub capture_trace: bool,
    pub capture_spans: bool,
    pub capture_frame_times: bool,
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
    CAPTURE_SCOPES.store(
        config.enabled && (config.capture_spans || config.capture_trace),
        Ordering::Relaxed,
    );
    CAPTURE_FRAMES.store(
        config.enabled && config.capture_frame_times,
        Ordering::Relaxed,
    );
    let session = SESSION.get_or_init(|| Mutex::new(ProfilingSession::default()));
    let mut guard = session.lock().unwrap();
    *guard = ProfilingSession::new(config);
}

pub fn begin_frame(frame: u64) {
    with_session(|session| session.begin_frame(frame));
}

pub fn end_frame(frame: u64) {
    if !CAPTURE_FRAMES.load(Ordering::Relaxed) {
        return;
    }
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
    if !CAPTURE_SCOPES.load(Ordering::Relaxed) {
        return ProfilingScope::disabled();
    }
    let now_ms = monotonic_now_ms();
    ProfilingScope {
        name,
        started_at_ms: Some(now_ms),
        start_micros: now_ms * 1000.0,
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
    started_at_ms: Option<f64>,
    start_micros: f64,
}

impl ProfilingScope {
    fn disabled() -> Self {
        Self {
            name: "",
            started_at_ms: None,
            start_micros: 0.0,
        }
    }
}

impl Drop for ProfilingScope {
    fn drop(&mut self) {
        let Some(started_at_ms) = self.started_at_ms else {
            return;
        };
        with_session(|session| {
            session.record_span(
                self.name,
                self.start_micros,
                (monotonic_now_ms() - started_at_ms).max(0.0),
            )
        });
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Default)]
struct LiveProfileState {
    scenario: String,
    warmup_frames: u64,
    start_frame: u64,
    started_at_ms: Option<f64>,
}

#[derive(Debug, Default)]
struct ProfilingSession {
    config: ProfilingConfig,
    current_frame: u64,
    frame_started_at_ms: Option<f64>,
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
            frame_started_at_ms: None,
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
        if !self.config.capture_frame_times {
            self.frame_started_at_ms = None;
            return;
        }
        self.frame_started_at_ms = Some(monotonic_now_ms());
    }

    fn end_frame(&mut self, frame: u64) {
        if !self.config.capture_frame_times {
            self.frame_started_at_ms = None;
            return;
        }
        if !self.should_capture(frame) {
            self.frame_started_at_ms = None;
            return;
        }
        if let Some(started_at_ms) = self.frame_started_at_ms.take() {
            self.frame_samples_ms
                .push((monotonic_now_ms() - started_at_ms).max(0.0));
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

    fn record_span(&mut self, name: &'static str, start_micros: f64, duration_ms: f64) {
        if !self.should_capture(self.current_frame) {
            return;
        }
        let ms = duration_ms.max(0.0);
        if self.config.capture_spans {
            self.span_samples_ms.entry(name).or_default().push(ms);
        }
        if self.config.capture_trace {
            self.trace_events.push(TraceEvent {
                name: name.to_string(),
                cat: "perf",
                ph: "X",
                ts: start_micros,
                dur: ms * 1000.0,
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

pub fn current_build_profile() -> String {
    option_env!("PROFILE").unwrap_or("unknown").to_string()
}

pub fn snapshot_last_values(names: &[&str]) -> HashMap<String, f64> {
    if !ENABLED.load(Ordering::Relaxed) {
        return HashMap::new();
    }
    let Some(session) = SESSION.get() else {
        return HashMap::new();
    };
    let guard = session.lock().unwrap();
    let mut values = HashMap::with_capacity(names.len());
    for name in names {
        if let Some(value) = guard.last_values.get(*name) {
            values.insert((*name).to_string(), *value);
        }
    }
    values
}

pub fn current_frame() -> u64 {
    CURRENT_FRAME.load(Ordering::Relaxed)
}

pub fn set_current_frame(frame: u64) {
    CURRENT_FRAME.store(frame, Ordering::Relaxed);
}

#[cfg(target_arch = "wasm32")]
pub fn start_live_session(
    scenario: impl Into<String>,
    warmup_frames: u64,
    capture_trace: bool,
    capture_spans: bool,
    capture_frame_times: bool,
) {
    let start_frame = current_frame();
    reset(ProfilingConfig {
        enabled: true,
        capture_after_frame: start_frame.saturating_add(warmup_frames),
        capture_trace,
        capture_spans,
        capture_frame_times,
    });
    LIVE_PROFILE_STATE.with(|state| {
        *state.borrow_mut() = LiveProfileState {
            scenario: scenario.into(),
            warmup_frames,
            start_frame,
            started_at_ms: Some(monotonic_now_ms()),
        };
    });
}

#[cfg(target_arch = "wasm32")]
pub fn clear_live_session() {
    reset(ProfilingConfig::default());
    set_current_frame(0);
    LIVE_PROFILE_STATE.with(|state| {
        *state.borrow_mut() = LiveProfileState::default();
    });
}

#[cfg(target_arch = "wasm32")]
pub fn live_report() -> ProfileSummary {
    LIVE_PROFILE_STATE.with(|state| {
        let state = state.borrow();
        let current_frame = current_frame();
        let measured_frames =
            current_frame.saturating_sub(state.start_frame.saturating_add(state.warmup_frames));
        let wall_clock_ms = state
            .started_at_ms
            .map(|started_at_ms| (monotonic_now_ms() - started_at_ms).max(0.0))
            .unwrap_or_default();
        report(ReportMetadata {
            scenario: if state.scenario.is_empty() {
                "browser".to_string()
            } else {
                state.scenario.clone()
            },
            bevy_version: "0.18.0".to_string(),
            git_revision: None,
            build_profile: current_build_profile(),
            frames: measured_frames,
            warmup_frames: state.warmup_frames,
            wall_clock_ms,
        })
    })
}

#[cfg(target_arch = "wasm32")]
pub fn trace_json() -> Result<String> {
    let Some(session) = SESSION.get() else {
        return Ok(serde_json::to_string(&TraceFile {
            trace_events: Vec::new(),
        })?);
    };
    let guard = session.lock().unwrap();
    Ok(serde_json::to_string(&TraceFile {
        trace_events: guard.trace_events.clone(),
    })?)
}

fn monotonic_now_ms() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Some(performance) = window.performance() {
                return performance.now();
            }
        }
        return js_sys::Date::now();
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        static CLOCK_BASE: OnceLock<Instant> = OnceLock::new();
        CLOCK_BASE.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.0
    }
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
