//! Opt-in diagnostics only. Samples never assert receiver or physical delivery.
use std::{
    io::Write,
    sync::mpsc,
    time::{Duration, Instant},
};

#[derive(Default)]
struct Metric {
    count: u64,
    sum_us: u128,
    max_us: u128,
}
impl Metric {
    fn add(&mut self, elapsed: Duration) {
        self.count += 1;
        self.sum_us += elapsed.as_micros();
        self.max_us = self.max_us.max(elapsed.as_micros());
    }
    fn mean_ms(&self) -> f64 {
        self.sum_us as f64 / self.count.max(1) as f64 / 1000.0
    }
}

pub(super) struct SpoutOutputTiming {
    output_id: u64,
    name: String,
    sink: mpsc::SyncSender<String>,
    window: Instant,
    previous_tick: Option<Instant>,
    cadence: Metric,
    render: Metric,
    send: Metric,
    keepalive: Metric,
    send_authority_ok: u64,
    dimensions: (u32, u32),
    origin: Instant,
    reports: u32,
    previous_pts: Option<u64>,
    pts_changes: u64,
}

impl SpoutOutputTiming {
    pub(super) fn from_env(output_id: u64, name: &str) -> Option<Self> {
        if std::env::var("SYNDOCAL_SPOUT_TIMING").ok().as_deref() != Some("1") {
            return None;
        }
        // stderr depends on how the app was launched. An explicitly supplied
        // existing directory receives route-specific files instead.
        let path = std::env::var_os("SYNDOCAL_SPOUT_TIMING_DIR").map(std::path::PathBuf::from);
        let (sink, records) = mpsc::sync_channel::<String>(2);
        std::thread::Builder::new()
            .name("spout-timing-log".into())
            .spawn(move || {
                let mut file = path.filter(|path| path.is_dir()).and_then(|path| {
                    std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(path.join(format!(
                            "spout-timing-{}-{output_id}.log",
                            std::process::id()
                        )))
                        .ok()
                });
                while let Ok(record) = records.recv() {
                    if let Some(file) = file.as_mut() {
                        let _ = writeln!(file, "{record}");
                    } else {
                        eprintln!("{record}");
                    }
                }
            })
            .ok()?;
        Some(Self {
            output_id,
            name: name.into(),
            sink,
            window: Instant::now(),
            previous_tick: None,
            cadence: Metric::default(),
            render: Metric::default(),
            send: Metric::default(),
            keepalive: Metric::default(),
            send_authority_ok: 0,
            dimensions: (0, 0),
            origin: Instant::now(),
            reports: 0,
            previous_pts: None,
            pts_changes: 0,
        })
    }
    pub(super) fn expired(&self, now: Instant) -> bool {
        self.reports >= 90 || now.duration_since(self.origin) >= Duration::from_secs(180)
    }
    pub(super) fn tick(&mut self, now: Instant, sdk: impl FnOnce() -> Option<(i64, f64)>) {
        if let Some(previous) = self.previous_tick.replace(now) {
            self.cadence.add(now.duration_since(previous));
        }
        if now.duration_since(self.window) < Duration::from_secs(2) {
            return;
        }
        let record = format!("SpoutTiming output={} name={:?} size={}x{} window_ms={} loop_n={} cadence_mean_ms={:.3} cadence_max_ms={:.3} render_n={} render_mean_ms={:.3} render_max_ms={:.3} send_authority_n={} send_authority_mean_ms={:.3} send_authority_max_ms={:.3} keepalive_n={} keepalive_mean_ms={:.3} keepalive_max_ms={:.3} send_authority_ok={} rendered_pts_changes={} sdk_frame_fps={:?} receiver_delivery=unmeasured",
            self.output_id, self.name, self.dimensions.0, self.dimensions.1, now.duration_since(self.window).as_millis(),
            self.cadence.count, self.cadence.mean_ms(), self.cadence.max_us as f64 / 1000.0,
            self.render.count, self.render.mean_ms(), self.render.max_us as f64 / 1000.0,
            self.send.count, self.send.mean_ms(), self.send.max_us as f64 / 1000.0,
            self.keepalive.count, self.keepalive.mean_ms(), self.keepalive.max_us as f64 / 1000.0, self.send_authority_ok, self.pts_changes, sdk());
        // Diagnostics cannot block the sender if a console or file stalls.
        let _ = self.sink.try_send(record);
        self.window = now;
        self.reports += 1;
        self.pts_changes = 0;
        self.cadence = Metric::default();
        self.render = Metric::default();
        self.send = Metric::default();
        self.keepalive = Metric::default();
        self.send_authority_ok = 0;
    }
    pub(super) fn rendered(&mut self, elapsed: Duration) {
        self.render.add(elapsed);
    }
    pub(super) fn sent(&mut self, elapsed: Duration, ok: bool, keepalive: bool) {
        if keepalive {
            self.keepalive.add(elapsed);
        } else {
            self.send.add(elapsed);
        }
        self.send_authority_ok += u64::from(ok);
    }
    pub(super) fn frame(&mut self, width: u32, height: u32, pts: u64) {
        self.dimensions = (width, height);
        if self
            .previous_pts
            .replace(pts)
            .is_some_and(|previous| previous != pts)
        {
            self.pts_changes += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spout_timing_aggregates_only_at_two_seconds_and_never_blocks_full_sink() {
        let (sink, records) = mpsc::sync_channel(1);
        let start = Instant::now();
        let mut timing = SpoutOutputTiming {
            output_id: 7,
            name: "Foreground".into(),
            sink,
            window: start,
            previous_tick: None,
            cadence: Metric::default(),
            render: Metric::default(),
            send: Metric::default(),
            keepalive: Metric::default(),
            send_authority_ok: 0,
            dimensions: (3840, 2160),
            origin: start,
            reports: 0,
            previous_pts: None,
            pts_changes: 0,
        };
        timing.frame(3840, 2160, 0);
        timing.frame(3840, 2160, 33);
        timing.frame(3840, 2160, 33);
        timing.rendered(Duration::from_millis(40));
        timing.sent(Duration::from_millis(3), true, false);
        timing.tick(start + Duration::from_millis(1999), || {
            panic!("SDK metrics only at aggregate boundary")
        });
        assert!(records.try_recv().is_err());
        timing.tick(start + Duration::from_secs(2), || Some((8, 25.0)));
        timing.tick(start + Duration::from_secs(4), || None); // full queue drops, never waits
        let record = records.try_recv().unwrap();
        assert!(record.contains("render_mean_ms=40.000"));
        assert!(record.contains("send_authority_mean_ms=3.000"));
        assert!(record.contains("size=3840x2160"));
        assert!(record.contains("receiver_delivery=unmeasured"));
        assert_eq!(timing.render.count, 0);
        assert!(record.contains("rendered_pts_changes=1"));
        assert!(timing.expired(start + Duration::from_secs(180)));
        timing.reports = 90;
        assert!(timing.expired(start));
    }
}
