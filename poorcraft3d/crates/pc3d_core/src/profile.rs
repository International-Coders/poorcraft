//! P3D-004: the engine's measuring stick
//! (docs/POORCRAFT-3D/16-IMPLEMENTATION-WORK-BREAKDOWN.md, P3D-000).
//!
//! The performance principles (03-ENGINE-BOUNDARIES-AND-STACK.md) name the
//! work that must be counted: mesh, fluid, path, entity ticks, network
//! bytes, memory. This module owns that vocabulary in one place —
//! subsystems increment, never redefine — plus frame-time capture with
//! nearest-rank percentiles and a deterministic baseline record the binary
//! prints via `--baseline`. Budgets arrive with the subsystems; this is the
//! ruler they will be held against.

/// The counter vocabulary, in snapshot order. Appending is allowed;
/// reordering or renaming is a regression for every saved baseline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CounterId {
    MeshWork,
    FluidWork,
    PathRequests,
    EntityTicks,
    NetworkBytes,
    SaveBytes,
    PatchRebuilds,
    JournalEvents,
}

impl CounterId {
    /// All counters, snapshot order.
    pub const ALL: [CounterId; 8] = [
        CounterId::MeshWork,
        CounterId::FluidWork,
        CounterId::PathRequests,
        CounterId::EntityTicks,
        CounterId::NetworkBytes,
        CounterId::SaveBytes,
        CounterId::PatchRebuilds,
        CounterId::JournalEvents,
    ];

    pub fn name(self) -> &'static str {
        match self {
            CounterId::MeshWork => "mesh_work",
            CounterId::FluidWork => "fluid_work",
            CounterId::PathRequests => "path_requests",
            CounterId::EntityTicks => "entity_ticks",
            CounterId::NetworkBytes => "network_bytes",
            CounterId::SaveBytes => "save_bytes",
            CounterId::PatchRebuilds => "patch_rebuilds",
            CounterId::JournalEvents => "journal_events",
        }
    }
}

/// Named u64 counters. Single-threaded by design until the simulation host
/// lands; snapshots iterate [`CounterId::ALL`] order, never a map.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Counters {
    values: [u64; CounterId::ALL.len()],
}

impl Counters {
    pub fn inc(&mut self, id: CounterId) {
        self.add(id, 1);
    }
    pub fn add(&mut self, id: CounterId, n: u64) {
        self.values[id as usize] = self.values[id as usize].saturating_add(n);
    }
    pub fn get(&self, id: CounterId) -> u64 {
        self.values[id as usize]
    }
    /// (id, value) pairs in enum order — the deterministic snapshot.
    pub fn snapshot(&self) -> Vec<(CounterId, u64)> {
        CounterId::ALL.iter().map(|&id| (id, self.get(id))).collect()
    }
}

/// Fixed-capacity frame-time ring (ms). Nearest-rank percentiles over the
/// sorted sample; digests fold raw bits in arrival order so identical
/// workloads digest identically.
pub struct FrameTimes {
    ring: Vec<f32>,
    head: usize,
    filled: usize,
}

impl Default for FrameTimes {
    fn default() -> Self {
        Self::new(1024)
    }
}

impl FrameTimes {
    pub fn new(capacity: usize) -> Self {
        FrameTimes { ring: vec![0.0; capacity.max(1)], head: 0, filled: 0 }
    }

    pub fn push(&mut self, frame_ms: f32) {
        if !frame_ms.is_finite() || frame_ms < 0.0 {
            return;
        }
        self.ring[self.head] = frame_ms;
        self.head = (self.head + 1) % self.ring.len();
        self.filled = (self.filled + 1).min(self.ring.len());
    }

    pub fn len(&self) -> usize {
        self.filled
    }

    pub fn is_empty(&self) -> bool {
        self.filled == 0
    }

    /// Nearest-rank percentile of the current sample (p in [0, 1]).
    pub fn percentile(&self, p: f32) -> f32 {
        if self.filled == 0 {
            return 0.0;
        }
        let mut sorted: Vec<f32> = self.ring[..self.filled].to_vec();
        sorted.sort_by(|a, b| a.total_cmp(b));
        let rank = ((p.clamp(0.0, 1.0) * self.filled as f32).ceil() as usize)
            .clamp(1, self.filled);
        sorted[rank - 1]
    }

    pub fn p50(&self) -> f32 {
        self.percentile(0.50)
    }
    pub fn p95(&self) -> f32 {
        self.percentile(0.95)
    }
    pub fn min(&self) -> f32 {
        self.percentile(0.0)
    }
    pub fn max(&self) -> f32 {
        self.percentile(1.0)
    }

    /// Arrival-order bit digest — identical samples, identical digest.
    pub fn digest(&self) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for i in 0..self.filled {
            let idx = (self.head + self.ring.len() - self.filled + i) % self.ring.len();
            for b in self.ring[idx].to_bits().to_le_bytes() {
                h ^= b as u64;
                h = h.wrapping_mul(0x100000001b3);
            }
        }
        h
    }
}

/// Explicit-call memory counters. A global allocator hook is a later,
/// opt-in change; until then callers that know they allocated report here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct MemoryCounters {
    pub allocated_bytes: u64,
    pub freed_bytes: u64,
    pub allocations: u64,
}

impl MemoryCounters {
    pub fn on_alloc(&mut self, bytes: u64) {
        self.allocated_bytes = self.allocated_bytes.saturating_add(bytes);
        self.allocations = self.allocations.saturating_add(1);
    }
    pub fn on_free(&mut self, bytes: u64) {
        self.freed_bytes = self.freed_bytes.saturating_add(bytes);
    }
    pub fn net_bytes(&self) -> i64 {
        self.allocated_bytes as i64 - self.freed_bytes as i64
    }
}

/// One machine-readable profile record: who measured, on what, and what the
/// numbers were. `to_json` is hand-rolled and key-ordered so the same
/// inputs always produce the same bytes — the baseline is diffable text.
#[derive(Clone, Debug, PartialEq)]
pub struct BaselineRecord {
    pub profile_name: String,
    pub arch: String,
    pub os: String,
    /// The P3D format epoch the record was taken under.
    pub format_epoch: u32,
    pub counters: Counters,
    pub frames: usize,
    pub p50_ms: f32,
    pub p95_ms: f32,
    pub min_ms: f32,
    pub max_ms: f32,
}

impl BaselineRecord {
    pub fn to_json(&self) -> String {
        let mut s = String::with_capacity(512);
        s.push('{');
        s.push_str(&format!("\"profile\":{},", json_str(&self.profile_name)));
        s.push_str(&format!("\"arch\":{},", json_str(&self.arch)));
        s.push_str(&format!("\"os\":{},", json_str(&self.os)));
        s.push_str(&format!("\"format_epoch\":{},", self.format_epoch));
        s.push_str("\"counters\":{");
        for (i, (id, v)) in self.counters.snapshot().iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            s.push_str(&format!("\"{}\":{}", id.name(), v));
        }
        s.push_str("},");
        s.push_str(&format!("\"frames\":{},", self.frames));
        s.push_str(&format!("\"frame_ms\":{{\"p50\":{:.3},\"p95\":{:.3},\"min\":{:.3},\"max\":{:.3}}}",
            self.p50_ms, self.p95_ms, self.min_ms, self.max_ms));
        s.push('}');
        s
    }
}

/// Minimal deterministic JSON string escape (quotes + backslash + controls).
fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Snapshots iterate in ENUM order regardless of increment order, and
    /// counters saturate instead of overflowing.
    #[test]
    fn p3d004_counter_snapshots_are_ordered_and_saturating() {
        let mut c = Counters::default();
        c.add(CounterId::JournalEvents, 5);
        c.inc(CounterId::MeshWork);
        c.add(CounterId::MeshWork, 9);
        let snap = c.snapshot();
        let names: Vec<&str> = snap.iter().map(|(id, _)| id.name()).collect();
        assert_eq!(
            names,
            vec![
                "mesh_work", "fluid_work", "path_requests", "entity_ticks",
                "network_bytes", "save_bytes", "patch_rebuilds", "journal_events"
            ]
        );
        assert_eq!(c.get(CounterId::MeshWork), 10);
        assert_eq!(c.get(CounterId::JournalEvents), 5);
        assert_eq!(c.get(CounterId::FluidWork), 0);
        c.add(CounterId::SaveBytes, u64::MAX);
        c.add(CounterId::SaveBytes, 100);
        assert_eq!(c.get(CounterId::SaveBytes), u64::MAX, "saturate, never wrap");
    }

    /// Nearest-rank percentiles on a known 100-sample set, plus the ring's
    /// wrap behavior and digest stability.
    #[test]
    fn p3d004_frame_percentiles_are_exact() {
        let mut ft = FrameTimes::new(128);
        for i in 1..=100u32 {
            ft.push(i as f32); // 1.0 .. 100.0 ms
        }
        assert_eq!(ft.len(), 100);
        assert_eq!(ft.min(), 1.0);
        assert_eq!(ft.max(), 100.0);
        assert_eq!(ft.p50(), 50.0, "nearest-rank p50 of 1..100 is the 50th");
        assert_eq!(ft.p95(), 95.0, "nearest-rank p95 is the 95th");
        // Wrap: pushing 40 more keeps only the newest 128 (60 old + 40 new).
        for i in 101..=140u32 {
            ft.push(i as f32);
        }
        assert_eq!(ft.len(), 128);
        assert_eq!(ft.min(), 13.0, "oldest samples fell off the ring");
        assert_eq!(ft.max(), 140.0);

        let d1 = ft.digest();
        let d2 = ft.digest();
        assert_eq!(d1, d2);
        ft.push(0.5);
        assert_ne!(ft.digest(), d1, "new sample must move the digest");

        // Invalid samples are dropped, not stored.
        let mut guard = FrameTimes::new(4);
        guard.push(f32::NAN);
        guard.push(-1.0);
        assert!(guard.is_empty());
    }

    /// Memory arithmetic: net = allocated - freed, saturating allocations.
    #[test]
    fn p3d004_memory_counters_net_correctly() {
        let mut m = MemoryCounters::default();
        m.on_alloc(100);
        m.on_alloc(50);
        m.on_free(30);
        assert_eq!(m.allocations, 2);
        assert_eq!(m.allocated_bytes, 150);
        assert_eq!(m.net_bytes(), 120);
        m.on_free(u64::MAX);
        assert_eq!(m.freed_bytes, u64::MAX, "saturate, never wrap negative");
        assert_eq!(m.net_bytes(), 150 - u64::MAX as i64);
    }

    /// The baseline record is diffable text: identical inputs produce
    /// identical JSON bytes, and the shape carries the whole story.
    #[test]
    fn p3d004_baseline_record_is_deterministic_json() {
        let record = || {
            let mut c = Counters::default();
            c.add(CounterId::MeshWork, 120);
            c.add(CounterId::EntityTicks, 600);
            BaselineRecord {
                profile_name: "p3d000-synthetic".into(),
                arch: std::env::consts::ARCH.into(),
                os: std::env::consts::OS.into(),
                format_epoch: 1,
                counters: c,
                frames: 3,
                p50_ms: 16.6,
                p95_ms: 16.9,
                min_ms: 16.5,
                max_ms: 16.9,
            }
        };
        let a = record().to_json();
        let b = record().to_json();
        assert_eq!(a, b, "same inputs must produce identical bytes");
        assert!(a.starts_with('{') && a.ends_with('}'));
        for key in [
            "\"profile\":\"p3d000-synthetic\"", "\"arch\":", "\"os\":",
            "\"format_epoch\":1", "\"mesh_work\":120", "\"entity_ticks\":600",
            "\"frames\":3", "\"p50\":16.600",
        ] {
            assert!(a.contains(key), "record missing {key}: {a}");
        }
        // Escaping is deterministic and safe.
        let mut esc = record();
        esc.profile_name = "quote\"back\\slash".into();
        let j = esc.to_json();
        assert!(j.contains("quote\\\"back\\\\slash"));
    }
}
