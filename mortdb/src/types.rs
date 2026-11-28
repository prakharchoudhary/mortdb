//! Core types shared by every component of the engine.
//!
//! The memtable, the write-ahead log, the table files, the iterators and
//! compaction all describe a row the same way: an [`InternalKey`] saying
//! which key was written and when, and an [`EntryKind`] saying what the write
//! did. The ordering of [`InternalKey`] defined here is the one ordering all
//! of those components rely on.

use std::cmp::Ordering;

/// Namespace is a prefix added to the key to allow logical partitioning
/// of a single physical storage into isolated datasets.
///
/// By design, the namespace sorts before the user key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Namespace(u32);

impl Namespace {
    /// By default, it is the namespace a write or read uses when the caller doesn't name one.
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

/// UserKey represents the key bytes the user supplied for an entry.
///
/// It holds raw bytes and not text, and any byte sequence is allowed.
/// Keys sort byte by byte from the left, and a key that is a prefix of
/// another sorts first.
///
/// It is the second part of the [`InternalKey`], after the namespace.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserKey(Vec<u8>);

impl UserKey {
    /// Initialise a new UserKey.
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

    /// Consume the UserKey and hand back the buffer it holds.
    ///
    /// The bytes are moved out, not copied. Use this when the caller needs
    /// to own the bytes and is finished with the key; use
    /// [`as_bytes`](Self::as_bytes) to only read them.
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
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

/// InternalKey represents the actual identifier record for a key-value pair being stored in the engine.
///
/// The composition is as follows: ([`Namespace`], [`UserKey`], [`SequenceNumber`])
///
/// Internal keys sort by namespace, then user key ascending, then sequence
/// number descending. All versions of one user key therefore sit together,
/// newest first.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InternalKey {
    namespace: Namespace,
    user_key: UserKey,
    seq_num: SequenceNumber,
}

impl InternalKey {
    /// Builds a new key from 3 parts: Namespace, UserKey and SequenceNumber.
    ///
    /// It takes ownership of the user key, with no copy.
    pub fn new(namespace: Namespace, user_key: UserKey, seq_num: SequenceNumber) -> Self {
        Self {
            namespace,
            user_key,
            seq_num,
        }
    }

    /// Returns the namespace this key belongs to.
    pub fn namespace(&self) -> Namespace {
        self.namespace
    }

    /// Lends the UserKey assigned to the key, without copying it.
    pub fn user_key(&self) -> &UserKey {
        &self.user_key
    }

    /// Returns the sequence number of the write that produced the key.
    pub fn seq_num(&self) -> SequenceNumber {
        self.seq_num
    }

    /// Consume the key and hand back its three parts.
    ///
    /// The user key is moved out, not copied. This is the reverse of
    /// [`new`](Self::new): the parts come back in the same order.
    pub fn into_parts(self) -> (Namespace, UserKey, SequenceNumber) {
        (self.namespace, self.user_key, self.seq_num)
    }
}

impl Ord for InternalKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.namespace
            .cmp(&other.namespace)
            .then_with(|| self.user_key.cmp(&other.user_key))
            // Reversed because higher (newer) sequence number should appear first.
            .then_with(|| other.seq_num.cmp(&self.seq_num))
    }
}

impl PartialOrd for InternalKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// What a single write did to its key: stored a value, or deleted the key.
///
/// This is the content side of a row. The key side is [`InternalKey`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryKind {
    /// The write stored these bytes under the key.
    Value(Vec<u8>),

    /// The write deleted the key.
    Tombstone,
}

impl EntryKind {
    /// Lends the stored bytes, or returns `None` for a tombstone.
    pub fn value(&self) -> Option<&[u8]> {
        match self {
            EntryKind::Value(bytes) => Some(bytes),
            EntryKind::Tombstone => None,
        }
    }

    /// Returns `true` if this write was a delete.
    pub fn is_tombstone(&self) -> bool {
        matches!(self, EntryKind::Tombstone)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::cmp::Reverse;

    /// Builds an internal key from plain parts
    fn key(namespace: u32, user_key: &[u8], seq_num: u64) -> InternalKey {
        InternalKey::new(
            Namespace::new(namespace),
            UserKey::new(user_key.to_vec()),
            SequenceNumber::new(seq_num),
        )
    }

    fn user_key(bytes: &[u8]) -> UserKey {
        UserKey::new(bytes.to_vec())
    }

    #[test]
    fn namespace_default_constant_holds_id_zero() {
        assert_eq!(Namespace::DEFAULT.get(), 0);
    }

    #[test]
    fn namespace_default_trait_matches_default_constant() {
        assert_eq!(Namespace::default(), Namespace::DEFAULT);
    }

    #[test]
    fn namespace_returns_the_id_it_was_built_from() {
        assert_eq!(Namespace::new(7).get(), 7);
    }

    #[test]
    fn namespace_with_smaller_id_sorts_first() {
        assert!(Namespace::new(1) < Namespace::new(2));
    }

    #[test]
    fn user_key_lends_back_the_bytes_it_was_built_from() {
        assert_eq!(user_key(b"user:42").as_bytes(), b"user:42");
    }

    #[test]
    fn user_key_into_bytes_returns_the_same_buffer_without_copying() {
        let bytes = b"user:42".to_vec();
        let address = bytes.as_ptr();

        let returned = UserKey::new(bytes).into_bytes();

        assert_eq!(returned, b"user:42");
        // Same heap allocation going in and coming out: nothing was copied.
        assert_eq!(returned.as_ptr(), address);
    }

    #[test]
    fn user_key_holds_bytes_that_are_not_valid_text() {
        let bytes = [0x00, 0xFF, 0x10];
        assert_eq!(user_key(&bytes).as_bytes(), bytes);
    }

    #[test]
    fn user_key_sorts_byte_by_byte() {
        assert!(user_key(b"a") < user_key(b"b"));
    }

    #[test]
    fn user_key_that_is_a_prefix_sorts_first() {
        assert!(user_key(b"ab") < user_key(b"abc"));
    }

    #[test]
    fn user_key_sorts_by_byte_value_not_alphabet() {
        // 'Z' is byte 90 and 'a' is byte 97.
        assert!(user_key(b"Z") < user_key(b"a"));
    }

    #[test]
    fn empty_user_key_sorts_before_every_non_empty_key() {
        assert!(user_key(b"") < user_key(&[0x00]));
    }

    #[test]
    fn sequence_number_reserved_constants_hold_the_extremes() {
        assert_eq!(SequenceNumber::ZERO.get(), 0);
        assert_eq!(SequenceNumber::MAX.get(), u64::MAX);
    }

    #[test]
    fn sequence_number_default_trait_is_zero() {
        assert_eq!(SequenceNumber::default(), SequenceNumber::ZERO);
    }

    #[test]
    fn sequence_number_returns_the_value_it_was_built_from() {
        assert_eq!(SequenceNumber::new(99).get(), 99);
    }

    #[test]
    fn next_from_zero_is_one() {
        assert_eq!(SequenceNumber::ZERO.next(), Some(SequenceNumber::new(1)));
    }

    #[test]
    fn next_from_an_ordinary_value_adds_one() {
        assert_eq!(
            SequenceNumber::new(41).next(),
            Some(SequenceNumber::new(42))
        );
    }

    #[test]
    fn next_from_two_below_max_gives_the_last_assignable_value() {
        assert_eq!(
            SequenceNumber::new(u64::MAX - 2).next(),
            Some(SequenceNumber::new(u64::MAX - 1))
        );
    }

    #[test]
    fn next_from_one_below_max_returns_none() {
        assert_eq!(SequenceNumber::new(u64::MAX - 1).next(), None);
    }

    #[test]
    fn next_from_max_returns_none_without_wrapping() {
        assert_eq!(SequenceNumber::MAX.next(), None);
    }

    #[test]
    fn lower_sequence_number_sorts_first_on_the_type_itself() {
        assert!(SequenceNumber::new(1) < SequenceNumber::new(3));
    }

    #[test]
    fn internal_key_returns_the_parts_it_was_built_from() {
        let k = key(5, b"a", 9);

        assert_eq!(k.namespace(), Namespace::new(5));
        assert_eq!(k.user_key(), &user_key(b"a"));
        assert_eq!(k.seq_num(), SequenceNumber::new(9));
    }

    #[test]
    fn reading_the_user_key_leaves_the_internal_key_usable() {
        let k = key(0, b"a", 1);

        let lent = k.user_key();
        assert_eq!(lent.as_bytes(), b"a");

        assert_eq!(k.seq_num(), SequenceNumber::new(1));
    }

    #[test]
    fn internal_key_into_parts_returns_the_parts_it_was_built_from() {
        let (namespace, user_key_part, seq_num) = key(5, b"a", 9).into_parts();

        assert_eq!(namespace, Namespace::new(5));
        assert_eq!(user_key_part, user_key(b"a"));
        assert_eq!(seq_num, SequenceNumber::new(9));
    }

    #[test]
    fn internal_key_into_parts_moves_the_user_key_without_copying() {
        let bytes = b"a".to_vec();
        let address = bytes.as_ptr();
        let k = InternalKey::new(
            Namespace::DEFAULT,
            UserKey::new(bytes),
            SequenceNumber::new(1),
        );

        let (_, user_key_part, _) = k.into_parts();

        // Same heap allocation going in and coming out: nothing was copied.
        let returned = user_key_part.into_bytes();
        assert_eq!(returned.as_ptr(), address);
    }

    #[test]
    fn newer_version_of_the_same_key_sorts_first() {
        assert!(key(0, b"a", 3) < key(0, b"a", 1));
    }

    #[test]
    fn user_key_outranks_sequence_number() {
        // "a" has the lower sequence number and still sorts first.
        assert!(key(0, b"a", 1) < key(0, b"b", 9));
    }

    #[test]
    fn namespace_outranks_user_key_and_sequence_number() {
        // Namespace 0 has the larger user key and the lower sequence number,
        // and still sorts first.
        assert!(key(0, b"z", 1) < key(1, b"a", 9));
    }

    #[test]
    fn identical_keys_compare_equal_and_are_equal() {
        let a = key(0, b"a", 1);
        let b = key(0, b"a", 1);

        assert_eq!(a.cmp(&b), Ordering::Equal);
        assert_eq!(a, b);
    }

    #[test]
    fn keys_differing_only_in_sequence_number_are_not_equal() {
        assert_ne!(key(0, b"a", 1), key(0, b"a", 2));
    }

    #[test]
    fn key_at_max_sequence_number_sorts_before_every_version_of_that_key() {
        let seek = InternalKey::new(Namespace::DEFAULT, user_key(b"a"), SequenceNumber::MAX);

        assert!(seek < key(0, b"a", u64::MAX - 1));
        assert!(seek < key(0, b"a", 1));
        // It still sorts after the previous user key.
        assert!(key(0, b"", 0) < seek);
    }

    #[test]
    fn sorting_puts_keys_ascending_and_versions_newest_first() {
        // Write order: put a, put b, put a, delete b.
        let mut keys = vec![
            key(0, b"a", 1),
            key(0, b"b", 2),
            key(0, b"a", 3),
            key(0, b"b", 4),
        ];

        keys.sort();

        assert_eq!(
            keys,
            vec![
                key(0, b"a", 3),
                key(0, b"a", 1),
                key(0, b"b", 4),
                key(0, b"b", 2),
            ]
        );
    }

    // ---------------------------------------------------------------------
    // InternalKey: property tests
    // ---------------------------------------------------------------------

    /// Generates internal keys.
    fn arb_key() -> impl Strategy<Value = InternalKey> {
        let namespace = prop_oneof![4 => 0u32..3, 1 => any::<u32>()];
        let user_key = prop::collection::vec(0u8..4, 0..4);
        let seq = prop_oneof![
            4 => 0u64..6,
            1 => any::<u64>(),
            1 => Just(u64::MAX),
        ];

        (namespace, user_key, seq).prop_map(|(ns, uk, seq)| key(ns, &uk, seq))
    }

    /// The ordering written out as a plain tuple: namespace, user key, then
    /// sequence number reversed.
    fn spec(k: &InternalKey) -> (u32, &[u8], Reverse<u64>) {
        (
            k.namespace().get(),
            k.user_key().as_bytes(),
            Reverse(k.seq_num().get()),
        )
    }

    proptest! {
        #[test]
        fn ordering_is_total(a in arb_key(), b in arb_key()) {
            let holds = [a < b, a == b, a > b];
            prop_assert_eq!(holds.iter().filter(|&&h| h).count(), 1);
        }

        #[test]
        fn ordering_is_antisymmetric(a in arb_key(), b in arb_key()) {
            prop_assert_eq!(a.cmp(&b), b.cmp(&a).reverse());
        }

        #[test]
        fn ordering_is_transitive(a in arb_key(), b in arb_key(), c in arb_key()) {
            if a <= b && b <= c {
                prop_assert!(a <= c);
            }
        }

        #[test]
        fn ordering_is_consistent_with_equality(a in arb_key(), b in arb_key()) {
            prop_assert_eq!(a.cmp(&b) == Ordering::Equal, a == b);
            prop_assert_eq!(a.partial_cmp(&b), Some(a.cmp(&b)));
        }

        #[test]
        fn ordering_matches_the_specification(a in arb_key(), b in arb_key()) {
            prop_assert_eq!(a.cmp(&b), spec(&a).cmp(&spec(&b)));
        }

        #[test]
        fn sorting_gives_the_same_result_after_shuffling(
            (original, shuffled) in prop::collection::vec(arb_key(), 0..20)
                .prop_flat_map(|keys| (Just(keys.clone()), Just(keys).prop_shuffle()))
        ) {
            let mut from_original = original;
            let mut from_shuffled = shuffled;
            from_original.sort();
            from_shuffled.sort();

            prop_assert_eq!(from_original, from_shuffled);
        }
    }

    #[test]
    fn value_entry_returns_its_bytes() {
        let entry = EntryKind::Value(b"hello".to_vec());
        assert_eq!(entry.value(), Some(&b"hello"[..]));
    }

    #[test]
    fn tombstone_entry_returns_no_value() {
        assert_eq!(EntryKind::Tombstone.value(), None);
    }

    #[test]
    fn is_tombstone_is_true_only_for_a_tombstone() {
        assert!(EntryKind::Tombstone.is_tombstone());
        assert!(!EntryKind::Value(b"x".to_vec()).is_tombstone());
    }

    #[test]
    fn empty_value_is_not_a_tombstone() {
        let entry = EntryKind::Value(Vec::new());

        assert!(!entry.is_tombstone());
        assert_eq!(entry.value(), Some(&[][..]));
    }

    #[test]
    fn entries_are_equal_only_when_kind_and_bytes_match() {
        assert_eq!(
            EntryKind::Value(b"x".to_vec()),
            EntryKind::Value(b"x".to_vec())
        );
        assert_ne!(
            EntryKind::Value(b"x".to_vec()),
            EntryKind::Value(b"y".to_vec())
        );
        assert_ne!(EntryKind::Value(Vec::new()), EntryKind::Tombstone);
    }
}
