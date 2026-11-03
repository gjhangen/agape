//! Places: a weighted graph of the venue (Path operation). Costs are walk seconds.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

#[derive(Clone, Debug)]
pub struct Place {
    pub id: String,
    pub kind: String,
    pub parent: Option<String>,
    pub open: bool,
}

#[derive(Clone, Debug)]
pub struct Edge { pub to: String, pub secs: u32, pub step_free: bool }

#[derive(Default)]
pub struct Graph {
    pub places: BTreeMap<String, Place>,
    pub edges: BTreeMap<String, Vec<Edge>>,
}

impl Graph {
    pub fn add(&mut self, id: &str, kind: &str, parent: Option<&str>) {
        self.places.insert(id.into(), Place { id: id.into(), kind: kind.into(), parent: parent.map(String::from), open: true });
    }
    /// Undirected edge.
    pub fn link(&mut self, a: &str, b: &str, secs: u32, step_free: bool) {
        self.edges.entry(a.into()).or_default().push(Edge { to: b.into(), secs, step_free });
        self.edges.entry(b.into()).or_default().push(Edge { to: a.into(), secs, step_free });
    }
    pub fn close(&mut self, id: &str) { if let Some(p) = self.places.get_mut(id) { p.open = false; } }

    /// Shortest walk in seconds, avoiding closed places; `step_free` restricts to accessible edges.
    pub fn path(&self, from: &str, to: &str, step_free: bool) -> Option<(u32, Vec<String>)> {
        let mut dist: BTreeMap<&str, u32> = BTreeMap::new();
        let mut prev: BTreeMap<&str, &str> = BTreeMap::new();
        let mut q = BinaryHeap::new();
        dist.insert(from, 0);
        q.push(Reverse((0u32, from)));
        while let Some(Reverse((d, u))) = q.pop() {
            if u == to {
                let mut path = vec![to.to_string()];
                let mut c = to;
                while let Some(p) = prev.get(c) { path.push(p.to_string()); c = p; }
                path.reverse();
                return Some((d, path));
            }
            if d > *dist.get(u).unwrap_or(&u32::MAX) { continue; }
            for e in self.edges.get(u).into_iter().flatten() {
                if step_free && !e.step_free { continue; }
                if !self.places.get(&e.to).map_or(false, |p| p.open) { continue; }
                let nd = d + e.secs;
                if nd < *dist.get(e.to.as_str()).unwrap_or(&u32::MAX) {
                    dist.insert(&e.to, nd);
                    prev.insert(&e.to, u);
                    q.push(Reverse((nd, &e.to)));
                }
            }
        }
        None
    }

    /// Is `id` inside `ancestor` (Scope operation)?
    pub fn within(&self, id: &str, ancestor: &str) -> bool {
        let mut c = Some(id.to_string());
        while let Some(x) = c {
            if x == ancestor { return true; }
            c = self.places.get(&x).and_then(|p| p.parent.clone());
        }
        false
    }
}
