//! One venue, one thread. The thread owns its Night; nothing else touches it. Every input that
//! changes the world (the clock moving, a person acting) is written to the venue's journal before
//! the caller hears back, so a crashed or restarted venue rebuilds itself exactly by replaying the
//! journal through the same deterministic kernel.

use agape_kernel::rules::Template;
use agape_kernel::sim::{Config, Night};
use agape_kernel::world::WorldDef;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, oneshot};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Init {
    pub id: String,
    pub name: String,
    pub world: Value,
    pub template: Value,
    pub seed: u64,
    pub fans: usize,
    pub runners: usize,
    /// Simulated minutes to run before people arrive (the pilot's crowd is simulated).
    pub start_min: i64,
    /// Which kernel this night was started on. A journal only replays exactly on the same one.
    #[serde(default)]
    pub build: String,
}

/// This server's kernel, as a hash of its sources (set when the image is built).
pub const BUILD: &str = match option_env!("AGAPE_KERNEL_BUILD") { Some(b) => b, None => "dev" };

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Entry { Init(Init), Step(u32), Act { who: String, v: Value } }

pub enum Cmd {
    View { who: String, v: Value, reply: oneshot::Sender<Value> },
    Act { who: String, v: Value, reply: oneshot::Sender<Value> },
    /// Advance the clock (from the venue's own timer or a test).
    Step { ticks: u32, reply: Option<oneshot::Sender<Value>> },
    Stop,
}

#[derive(Clone)]
pub struct Handle {
    pub id: String,
    pub name: String,
    /// What this night was started from (the next night starts from the same, with a new seed).
    pub init: Init,
    /// Set once the night is over: the game is final and nothing is still on its way.
    pub over: std::sync::Arc<std::sync::atomic::AtomicBool>,
    tx: mpsc::Sender<Cmd>,
    pub events: broadcast::Sender<String>,
}

impl Handle {
    pub async fn view(&self, who: &str, v: Value) -> Value {
        let (reply, rx) = oneshot::channel();
        if self.tx.send(Cmd::View { who: who.into(), v, reply }).is_err() { return json!({ "error": "venue is restarting" }); }
        rx.await.unwrap_or_else(|_| json!({ "error": "venue is restarting" }))
    }
    pub async fn act(&self, who: &str, v: Value) -> Value {
        let (reply, rx) = oneshot::channel();
        if self.tx.send(Cmd::Act { who: who.into(), v, reply }).is_err() { return json!({ "ok": false, "error": "venue is restarting" }); }
        rx.await.unwrap_or_else(|_| json!({ "ok": false, "error": "venue is restarting" }))
    }
    pub async fn step(&self, ticks: u32) -> Value {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(Cmd::Step { ticks, reply: Some(reply) });
        rx.await.unwrap_or(Value::Null)
    }
    pub fn stop(&self) { let _ = self.tx.send(Cmd::Stop); }
}

struct Journal { file: File, pending_steps: u32 }

impl Journal {
    fn write(&mut self, e: &Entry) {
        let mut line = serde_json::to_vec(e).expect("journal entry");
        line.push(b'\n');
        self.file.write_all(&line).expect("journal write");
    }
    /// Steps are coalesced, then written before any act so the order is kept.
    fn flush_steps(&mut self) {
        if self.pending_steps > 0 { let n = self.pending_steps; self.pending_steps = 0; self.write(&Entry::Step(n)); }
    }
    fn sync(&mut self) { self.file.sync_data().expect("journal sync"); }
}

fn build(init: &Init) -> Result<Night, String> {
    let w: WorldDef = serde_json::from_value(init.world.clone()).map_err(|e| e.to_string())?;
    let t: Template = serde_json::from_value(init.template.clone()).map_err(|e| e.to_string())?;
    let mut n = Night::new(w, t, Config { seed: init.seed, fans: init.fans, runners: init.runners, you_seat: None, ..Config::default() });
    for _ in 0..init.start_min * 6 { n.step(); }
    Ok(n)
}

/// Acts the journal must keep: anything that may change the world. Reads and quotes don't.
fn changes_world(v: &Value) -> bool {
    !matches!(v["do"].as_str(), Some("quote" | "ticket_quote" | "ride_quote" | "window"))
}

/// Rebuild a venue from its journal, or start one from `init` if it has none.
fn restore(path: &Path, init: Option<Init>) -> Result<(Init, Night, Journal), String> {
    let exists = path.exists() && std::fs::metadata(path).map(|m| m.len() > 0).unwrap_or(false);
    let (init, mut night) = if exists {
        let f = File::open(path).map_err(|e| e.to_string())?;
        let mut lines = BufReader::new(f).lines();
        let first: Entry = serde_json::from_str(&lines.next().ok_or("empty journal")?.map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        let Entry::Init(init) = first else { return Err("journal must start with init".into()) };
        let mut n = build(&init)?;
        for line in lines {
            let line = line.map_err(|e| e.to_string())?;
            if line.trim().is_empty() { continue; }
            match serde_json::from_str::<Entry>(&line) {
                Ok(Entry::Step(k)) => for _ in 0..k { n.step(); },
                Ok(Entry::Act { who, v }) => { let _ = agape_kernel::api::act(&mut n, &who, &v); }
                Ok(Entry::Init(_)) => return Err("second init in journal".into()),
                Err(_) => break, // a torn last line from a crash: everything before it is intact
            }
        }
        (init, n)
    } else {
        let init = init.ok_or("no journal and nothing to start from")?;
        let n = build(&init)?;
        (init, n)
    };
    let file = OpenOptions::new().create(true).append(true).open(path).map_err(|e| e.to_string())?;
    let mut j = Journal { file, pending_steps: 0 };
    if !exists { j.write(&Entry::Init(init.clone())); j.sync(); }
    let _ = &mut night;
    Ok((init, night, j))
}

/// Start a venue's thread. `ticks_per_sec` is the clock: 0.1 is real time (one 10-second step every
/// 10 seconds); 0 means the clock only moves when told (tests).
pub fn spawn(dir: &Path, init: Option<Init>, id: &str, ticks_per_sec: f64) -> Result<Handle, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let path: PathBuf = dir.join(format!("{id}.journal"));
    let (init, mut night, mut journal) = restore(&path, init)?;
    let (tx, rx) = mpsc::channel::<Cmd>();
    let (events, _) = broadcast::channel(64);
    // a night begun on a different kernel won't replay exactly: finish it now and open the next
    let over = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(night.done || init.build != BUILD));
    let h = Handle { id: init.id.clone(), name: init.name.clone(), init: init.clone(), over: over.clone(), tx, events: events.clone() };
    std::thread::Builder::new().name(format!("venue-{id}")).spawn(move || {
        let mut due = 0.0f64;
        let mut last = Instant::now();
        let announce = |n: &Night| { let _ = events.send(json!({ "t": n.t }).to_string()); };
        loop {
            let wait = if ticks_per_sec > 0.0 { Duration::from_millis(100) } else { Duration::from_secs(3600) };
            match rx.recv_timeout(wait) {
                Ok(Cmd::View { who, v, reply }) => { let _ = reply.send(agape_kernel::api::view(&mut night, &who, &v)); }
                Ok(Cmd::Act { who, v, reply }) => {
                    let out = agape_kernel::api::act(&mut night, &who, &v);
                    if changes_world(&v) {
                        journal.flush_steps();
                        journal.write(&Entry::Act { who, v });
                        journal.sync();
                        announce(&night);
                    }
                    let _ = reply.send(out);
                }
                Ok(Cmd::Step { ticks, reply }) => {
                    for _ in 0..ticks { if night.step() { journal.pending_steps += 1; } }
                    journal.flush_steps(); journal.sync();
                    announce(&night);
                    if night.done { over.store(true, std::sync::atomic::Ordering::Relaxed); }
                    if let Some(r) = reply { let _ = r.send(json!({ "t": night.t })); }
                }
                Ok(Cmd::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => { journal.flush_steps(); journal.sync(); return; }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
            if ticks_per_sec > 0.0 {
                let now = Instant::now();
                due += now.duration_since(last).as_secs_f64() * ticks_per_sec;
                last = now;
                let mut moved = false;
                while due >= 1.0 { due -= 1.0; if night.step() { journal.pending_steps += 1; moved = true; } }
                if journal.pending_steps >= 6 { journal.flush_steps(); journal.sync(); }
                if moved { announce(&night); }
                if night.done { over.store(true, std::sync::atomic::Ordering::Relaxed); }
            }
        }
    }).map_err(|e| e.to_string())?;
    Ok(h)
}
