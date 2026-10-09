//! RAM replay ring. Mirrors `src/replay_buffer/replay_buffer_ram.c`.
//!
//! Observable semantics match C exactly (overwrite-oldest ring, capped
//! count, binary time search, keyframe scan, snapshot isolation).
//! Implementation uses `VecDeque<Arc<Packet>>` instead of a raw index ring;
//! snapshots share packets by `Arc` like the C refcount clone.

use std::collections::VecDeque;
use std::sync::Arc;

use super::replay::{Cursor, Packet, RingView};

/// Fixed-capacity RAM ring. Panics on zero capacity (C asserts `> 0`).
pub struct RamRing {
    packets: VecDeque<Arc<Packet>>,
    capacity: usize,
}

impl RamRing {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "replay capacity must be non-zero");
        RamRing {
            packets: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Append, evicting the oldest packet when full (C overwrites `index`).
    pub fn append(&mut self, packet: Packet) {
        if self.packets.len() >= self.capacity {
            self.packets.pop_front();
        }
        self.packets.push_back(Arc::new(packet));
    }

    pub fn clear(&mut self) {
        self.packets.clear();
    }

    pub fn len(&self) -> usize {
        self.packets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.packets.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<Arc<Packet>> {
        self.packets.get(index).cloned()
    }

    /// Frozen share of the whole buffer (C `clone` semantics).
    pub fn snapshot(&self) -> Vec<Arc<Packet>> {
        self.packets.iter().cloned().collect()
    }

    /// Owned packets from `from` to the end (save path).
    pub fn snapshot_range(&self, from: Cursor) -> Vec<Packet> {
        self.packets
            .iter()
            .skip(from.packet)
            .map(|p| (**p).clone())
            .collect()
    }
}

impl RingView for RamRing {
    fn packet_count(&self) -> usize {
        self.packets.len()
    }

    /// Exact mirror of the C binary search (including `{0,0}` on empty).
    fn find_by_time(&self, seconds: i64, now: f64) -> Cursor {
        if self.packets.is_empty() {
            return Cursor::default();
        }
        let seconds = seconds as f64;
        let mut lower: usize = 0;
        let mut upper: usize = self.packets.len();
        let index = loop {
            let index = lower + (upper - lower) / 2;
            let passed = now - self.packets[index].timestamp;
            if passed >= seconds {
                if lower == index {
                    break index;
                }
                lower = index;
            } else {
                if upper == index {
                    break index;
                }
                upper = index;
            }
        };
        Cursor {
            packet: index,
            file: 0,
        }
    }

    fn find_keyframe(&self, start: Cursor, stream: i32, invert: bool) -> Option<Cursor> {
        for (i, packet) in self.packets.iter().enumerate().skip(start.packet) {
            let stream_match = if invert {
                packet.stream_index != stream
            } else {
                packet.stream_index == stream
            };
            if packet.key && stream_match {
                return Some(Cursor { packet: i, file: 0 });
            }
        }
        None
    }

    fn next(&self, cursor: Cursor) -> Option<Cursor> {
        if cursor.packet + 1 < self.packets.len() {
            Some(Cursor {
                packet: cursor.packet + 1,
                file: 0,
            })
        } else {
            None
        }
    }
}
