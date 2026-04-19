//! Logic for reassembling fragmented network messages.

use std::collections::{HashMap, hash_map};
use std::time::{Duration, Instant};

use crate::events::FragmentedEvent;
use crate::types::ClientId;

/// Buffers fragments for a single larger message from a specific client.
#[derive(Debug, Clone)]
struct FragmentBuffer {
    /// When the first fragment of this message was received.
    start_time: Instant,
    /// Total number of fragments expected.
    total_fragments: u16,
    /// Fragments received so far.
    fragments: Vec<Option<Vec<u8>>>,
    /// Number of fragments currently present in the buffer.
    count: u16,
}

impl FragmentBuffer {
    fn new(total_fragments: u16) -> Option<Self> {
        if total_fragments == 0 || total_fragments > crate::MAX_TOTAL_FRAGMENTS {
            return None;
        }

        Some(Self {
            start_time: Instant::now(),
            total_fragments,
            fragments: vec![None; total_fragments as usize],
            count: 0,
        })
    }

    fn add(&mut self, index: u16, payload: Vec<u8>) -> Option<Vec<u8>> {
        let idx = index as usize;
        if idx >= self.fragments.len() {
            return None;
        }

        if self.fragments[idx].is_none() {
            self.fragments[idx] = Some(payload);
            self.count += 1;
        }

        if self.count == self.total_fragments {
            let mut full_payload = Vec::new();
            for frag in self.fragments.drain(..) {
                full_payload.extend(frag.unwrap());
            }
            Some(full_payload)
        } else {
            None
        }
    }

    fn is_stale(&self, timeout: Duration) -> bool {
        self.start_time.elapsed() > timeout
    }
}

/// A stateful reassembler that tracks fragmented messages from multiple clients.
#[derive(Debug, Default, Clone)]
pub struct Reassembler {
    /// `message_id` -> buffer
    buffers: HashMap<(ClientId, u32), FragmentBuffer>,
    /// How long to keep fragments before discarding.
    timeout: Duration,
}

impl Reassembler {
    /// Creates a new reassembler with a default timeout of 5 seconds.
    #[must_use]
    pub fn new() -> Self {
        Self {
            buffers: HashMap::new(),
            timeout: Duration::from_secs(5),
        }
    }

    /// Sets a custom timeout for message reassembly.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Ingests a fragment into the reassembler.
    ///
    /// Returns the full reassembled message if this was the last fragment,
    /// otherwise returns `None`.
    pub fn ingest(&mut self, client_id: ClientId, event: FragmentedEvent) -> Option<Vec<u8>> {
        // Security check: ensure total_fragments is valid from untrusted input
        if event.total_fragments == 0 || event.total_fragments > crate::MAX_TOTAL_FRAGMENTS {
            tracing::warn!(
                "Rejecting fragment with invalid total_fragments: {}",
                event.total_fragments
            );
            return None;
        }

        let key = (client_id, event.message_id);

        let buffer = match self.buffers.entry(key) {
            hash_map::Entry::Occupied(e) => e.into_mut(),
            hash_map::Entry::Vacant(e) => match FragmentBuffer::new(event.total_fragments) {
                Some(buf) => e.insert(buf),
                None => return None,
            },
        };

        // Safety check: ensure total_fragments matches what we original expected for this message_id
        if buffer.total_fragments != event.total_fragments {
            tracing::warn!(
                "Fragment mismatch for message_id {}: expected {}, got {}",
                event.message_id,
                buffer.total_fragments,
                event.total_fragments
            );
            return None;
        }

        let result = buffer.add(event.fragment_index, event.payload);

        if result.is_some() {
            self.buffers.remove(&key);
        }

        result
    }

    /// Discards messages that have haven't been completed within the timeout.
    pub fn prune(&mut self) {
        self.buffers
            .retain(|_, buffer| !buffer.is_stale(self.timeout));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reassembly_ordered() {
        let mut reassembler = Reassembler::new();
        let cid = ClientId(1);
        let mid = 100;

        let f1 = FragmentedEvent {
            message_id: mid,
            fragment_index: 0,
            total_fragments: 2,
            payload: vec![1, 2],
        };
        let f2 = FragmentedEvent {
            message_id: mid,
            fragment_index: 1,
            total_fragments: 2,
            payload: vec![3, 4],
        };

        assert!(reassembler.ingest(cid, f1).is_none());
        let result = reassembler.ingest(cid, f2).unwrap();
        assert_eq!(result, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_reassembly_out_of_order() {
        let mut reassembler = Reassembler::new();
        let cid = ClientId(1);
        let mid = 101;

        let f1 = FragmentedEvent {
            message_id: mid,
            fragment_index: 0,
            total_fragments: 3,
            payload: vec![1],
        };
        let f2 = FragmentedEvent {
            message_id: mid,
            fragment_index: 1,
            total_fragments: 3,
            payload: vec![2],
        };
        let f3 = FragmentedEvent {
            message_id: mid,
            fragment_index: 2,
            total_fragments: 3,
            payload: vec![3],
        };

        assert!(reassembler.ingest(cid, f3).is_none());
        assert!(reassembler.ingest(cid, f1).is_none());
        let result = reassembler.ingest(cid, f2).unwrap();
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_cleanup() {
        let mut reassembler = Reassembler::new().with_timeout(Duration::from_millis(10));
        let cid = ClientId(1);
        let mid = 102;

        reassembler.ingest(
            cid,
            FragmentedEvent {
                message_id: mid,
                fragment_index: 0,
                total_fragments: 2,
                payload: vec![1],
            },
        );

        std::thread::sleep(Duration::from_millis(20));
        reassembler.prune();
        assert!(reassembler.buffers.is_empty());
    }
}
