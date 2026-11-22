/// Namespace is a prefix added to the key to allow logical partitioning
/// of a single physical storage into isolated datasets.
///
/// By design, the namespace sorts before the user key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Namespace(u32);

impl Namespace {
    /// By default, it is the namespace a write or read uses when the caller doesn’t name one.
    pub const DEFAULT: Self = Self(0);

    /// Initialise the namespace with a u32 int.
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Retrieve the namespace value.
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl Default for Namespace {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// UserKey represent the key value set by the user for their input key-value entry.
///
/// It holds raw bytes and not test, and any byte sequence is allowed.
/// It is the second part of the InternalKey, after namespace.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserKey(Vec<u8>);

impl UserKey {
    /// Initialise a new Userkey.
    ///
    /// It takes ownership of the buffer it is given, with no copy and no validation.
    pub fn new(key: Vec<u8>) -> Self {
        Self(key)
    }

    /// Return UserKey as bytes.
    ///
    /// It lends the bytes without copying, and the key stays usable afterwards.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// SequenceNumber is a 64-bit counter stamped on every write.
/// The higher number is the newer write, wherever it is stored.
///
/// Every put and delete receives its own sequence number, and no two writes
/// ever share one.
///
/// Two values are reserved and never belong to a real write:
/// [`SequenceNumber::ZERO`] and [`SequenceNumber::MAX`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SequenceNumber(u64);

impl SequenceNumber {
    /// The state before any write: "nothing has been written yet".
    ///
    /// No write carries this value. The first write to an empty store carries
    /// `ZERO.next()`, which is 1.
    pub const ZERO: Self = Self(0);

    /// The highest possible value, used to mean "newer than every write".
    pub const MAX: Self = Self(u64::MAX);

    /// Initialise the sequence number with a u64 int.
    ///
    /// To produce the sequence number for a new write, use
    /// [`next`](Self::next) instead.
    pub const fn new(seq: u64) -> Self {
        Self(seq)
    }

    /// Retrieve the sequence number.
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Returns the sequence number that follows this one.
    ///
    /// Returns `None` when the counter is exhausted: when the next value
    /// would be the reserved [`MAX`](Self::MAX), or when this value is
    /// already `MAX`. The caller decides how to report that.
    pub const fn next(self) -> Option<SequenceNumber> {
        if self.0 >= Self::MAX.0 - 1 {
            return None;
        }

        Some(Self(self.0 + 1))
    }
}

impl Default for SequenceNumber {
    fn default() -> Self {
        Self::ZERO
    }
}
