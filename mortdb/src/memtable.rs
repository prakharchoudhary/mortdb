//! The in-memory sorted buffer that receives every write before it is
//! flushed to a table file.
use std::collections::BTreeMap;

use crate::{EntryKind, InternalKey};

/// This is the fixed memory overhead cost for each entry, in addition to the actual key and value bytes.
const ENTRY_OVERHEAD_BYTES: usize = size_of::<InternalKey>() + size_of::<EntryKind>();

/// A sorted in-memory buffer of recent writes.
///
/// Every write is under a unique [`InternalKey`] such that nothing overwritten,
/// and table keeps growing until it is flushed out.
#[derive(Debug)]
pub struct MemTable {
    entries: BTreeMap<InternalKey, EntryKind>,
    size_bytes: usize,
    max_size_bytes: usize,
}

impl MemTable {
    /// Creates an empty table that reports full at `max_size_bytes`.
    pub fn new(max_size_bytes: usize) -> Self {
        tracing::debug!(max_size_bytes, "memtable created");

        Self {
            entries: BTreeMap::new(),
            size_bytes: 0,
            max_size_bytes,
        }
    }

    /// Looks up the newest version of a key as of a sequence number.
    pub fn get(&self, key: &InternalKey) -> Option<&EntryKind> {
        let entry = match self.entries.range(key..).next() {
            Some((found, entry))
                if found.namespace() == key.namespace() && found.user_key() == key.user_key() =>
            {
                Some(entry)
            }
            _ => None,
        };

        let outcome = match entry {
            Some(EntryKind::Value(_)) => "value",
            Some(EntryKind::Tombstone) => "tombstone",
            None => "miss",
        };
        tracing::trace!(
            namespace = key.namespace().get(),
            seq_num = key.seq_num().get(),
            key_len = key.user_key().as_bytes().len(),
            outcome,
            "memtable get"
        );

        entry
    }

    /// Stores `value` under a `key`.
    pub fn put(&mut self, key: InternalKey, value: &[u8]) {
        self.insert(key, EntryKind::Value(value.to_vec()));
    }

    /// Marks a `key` as logically deleted.
    pub fn delete(&mut self, key: InternalKey) {
        self.insert(key, EntryKind::Tombstone);
    }

    /// Reports the current size of the memtable.
    pub fn size_bytes(&self) -> usize {
        self.size_bytes
    }

    /// Returns `true` if the size of memtable is larger than configured `max_size_bytes`.
    pub fn is_full(&self) -> bool {
        self.size_bytes >= self.max_size_bytes
    }

    fn insert(&mut self, key: InternalKey, entry: EntryKind) {
        let value_len = entry.value().map_or(0, <[u8]>::len);
        self.size_bytes += ENTRY_OVERHEAD_BYTES + key.user_key().as_bytes().len() + value_len;

        // Read the log fields now: the insert below takes ownership of the key.
        let namespace = key.namespace().get();
        let seq_num = key.seq_num().get();
        let key_len = key.user_key().as_bytes().len();
        let op = if entry.is_tombstone() {
            "delete"
        } else {
            "put"
        };

        let prev = self.entries.insert(key, entry);

        tracing::trace!(
            namespace,
            seq_num,
            key_len,
            value_len,
            op,
            size_bytes = self.size_bytes,
            "memtable write"
        );

        if prev.is_some() {
            tracing::error!(
                namespace,
                seq_num,
                key_len,
                "two writes share one internal key; the older write was replaced"
            );
            debug_assert!(
                false,
                "two writes share one internal key; sequence numbers must be unique"
            );
        }
    }
}
