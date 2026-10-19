//! Scratch storage for nested message lengths during encoding.
//!
//! This module is public for use by `prost-derive`, but is an implementation
//! detail. The `size-cache` feature enables the cached public encoding paths
//! and requires `std`. It is enabled by default.
//!
//! # Size and write passes
//!
//! A nested message's length prefix normally requires another size traversal
//! while writing its parent. Repeating this at each level revisits descendants
//! in deep trees. Instead, the size pass reserves a `u32` slot for each message
//! before visiting its children, then fills that slot with the computed length.
//! The write pass consumes slots in the same preorder and uses the stored
//! lengths for message prefixes. Every public encoding call recomputes the
//! lengths; only scratch capacity is reused across calls.
//!
//! The two passes must visit fields and entries in the same order. Slots cover
//! required, optional, repeated, boxed, and oneof messages, groups, and synthetic
//! entries for maps with message values. Within cached trees, nonempty packed
//! variable-width integer and enum fields also record their payload lengths,
//! avoiding a second length scan during writing. Fixed-width and bool fields
//! use their existing routines. Scalar maps retain their existing encoding.
//!
//! # Selecting the cached path
//!
//! At each public encoding entry, generated `Message::use_size_cache` methods
//! inspect only direct field presence. Optional message and group fields check
//! `is_some`; repeated fields and message maps check `!is_empty`; required fields
//! return true; oneofs inspect the active variant. Scalar-only schemas return
//! false. A false result skips thread-local storage entirely, including for
//! flat messages with packed fields. Presence checks do not traverse children
//! or compute encoded lengths.
//!
//! The predicate is checked only at the public entry. Leaves inside a cached
//! tree still reserve and consume slots. Built-in primitive `Message` wrappers
//! skip the cached entry path, and `Box` forwards the predicate. Handwritten
//! implementations default to an opaque slot containing `encoded_len()` and
//! delegate writing to `encode_raw()`. Such subtrees remain compatible but do
//! not gain recursive length caching without implementing the cached methods.
//!
//! # Scratch lifetime and reentrancy
//!
//! An encoding operation borrows a thread-local `RefCell<SizeBuffer>` and clears
//! its lengths without releasing capacity. Recursive generated calls receive
//! explicit buffer or cursor references and do not access thread-local storage.
//! If custom `Message` or `BufMut` code reenters a public encoder on the same
//! thread, a failed `try_borrow_mut` selects a separate local buffer, which may
//! allocate. A local buffer is also used if the thread-local key has already
//! been destroyed. Panic unwinding releases the borrow; the next operation
//! clears unfinished entries. The cache adds no unsafe code.
//!
//! Encoding is synchronous, so the borrow does not cross an await or thread
//! migration, and encoding remains available during thread-local destruction.
//!
//! # Memory and fallback behavior
//!
//! Logical scratch usage is one `u32` per message, per synthetic message-map
//! entry, and per nonempty packed variable-width integer or enum field in a
//! cached tree. For example, 1,023 messages each containing one packed uint64
//! field use 8,184 bytes of logical scratch. Allocated capacity may be larger
//! and has no retention limit; each thread keeps its largest grown capacity
//! until exit. Warming that capacity removes subsequent scratch allocations
//! for messages that fit it.
//!
//! If any recorded length exceeds `u32::MAX`, the entire write pass falls back
//! to `encode_raw()`, preserving the existing `usize` API without truncation.
//! The cache does not change message layouts, standalone `encoded_len()`, or
//! wire encoding. It removes repeated size traversal for generated message
//! trees, but scratch bookkeeping can cost more than it saves for shallow trees.
//!
//! Unit tests cover flat-message bypass and overflow fallback. The integration
//! tests in `prost/tests/size_cache.rs` compare all public encoders with raw
//! encoding and cover size boundaries, mutations, maps, oneofs, groups, boxes,
//! custom reentry, panic recovery, and independent threads.

use crate::encoding::{encode_key, encode_varint, encoded_len_varint, key_len, WireType};
use crate::Message;
use alloc::vec::Vec;
use bytes::BufMut;

#[derive(Default)]
pub struct SizeBuffer {
    lengths: Vec<u32>,
    overflow: bool,
}

impl SizeBuffer {
    #[inline]
    pub fn begin(&mut self) -> usize {
        let slot = self.lengths.len();
        self.lengths.push(0);
        slot
    }
    #[inline]
    pub fn finish(&mut self, slot: usize, len: usize) -> usize {
        match u32::try_from(len) {
            Ok(len) => self.lengths[slot] = len,
            Err(_) => self.overflow = true,
        }
        len
    }
    #[inline]
    pub fn record(&mut self, len: usize) -> usize {
        let slot = self.begin();
        self.finish(slot, len)
    }
    #[cfg(feature = "size-cache")]
    fn clear(&mut self) {
        self.lengths.clear();
        self.overflow = false;
    }
}

pub struct SizeCursor<'a> {
    remaining: &'a [u32],
}
impl SizeCursor<'_> {
    #[inline]
    pub fn peek(&self) -> usize {
        self.remaining[0] as usize
    }
    #[inline]
    pub fn next_size(&mut self) -> usize {
        let (len, tail) = self.remaining.split_first().expect("size cache exhausted");
        self.remaining = tail;
        *len as usize
    }
}

pub mod message {
    use super::*;
    #[inline]
    pub fn encoded_len<M: Message>(tag: u32, msg: &M, sizes: &mut SizeBuffer) -> usize {
        let len = msg.encoded_len_cached(sizes);
        key_len(tag) + encoded_len_varint(len as u64) + len
    }
    pub fn encoded_len_repeated<M: Message>(
        tag: u32,
        values: &[M],
        sizes: &mut SizeBuffer,
    ) -> usize {
        values.iter().map(|msg| encoded_len(tag, msg, sizes)).sum()
    }
    #[inline]
    pub fn encode<M: Message>(
        tag: u32,
        msg: &M,
        buf: &mut impl BufMut,
        sizes: &mut SizeCursor<'_>,
    ) {
        encode_key(tag, WireType::LengthDelimited, buf);
        encode_varint(sizes.peek() as u64, buf);
        msg.encode_raw_cached(buf, sizes);
    }
}

pub mod group {
    use super::*;
    pub fn encoded_len<M: Message>(tag: u32, msg: &M, sizes: &mut SizeBuffer) -> usize {
        2 * key_len(tag) + msg.encoded_len_cached(sizes)
    }
    pub fn encoded_len_repeated<M: Message>(
        tag: u32,
        values: &[M],
        sizes: &mut SizeBuffer,
    ) -> usize {
        values.iter().map(|msg| encoded_len(tag, msg, sizes)).sum()
    }
    pub fn encode<M: Message>(
        tag: u32,
        msg: &M,
        buf: &mut impl BufMut,
        sizes: &mut SizeCursor<'_>,
    ) {
        encode_key(tag, WireType::StartGroup, buf);
        msg.encode_raw_cached(buf, sizes);
        encode_key(tag, WireType::EndGroup, buf);
    }
}

#[cfg(feature = "size-cache")]
std::thread_local! {
    static SCRATCH: std::cell::RefCell<SizeBuffer> = const {
        std::cell::RefCell::new(SizeBuffer { lengths: Vec::new(), overflow: false })
    };
}

#[cfg(feature = "size-cache")]
fn with_sizes<R>(f: impl FnOnce(&mut SizeBuffer) -> R) -> R {
    #[cfg(test)]
    tests::TLS_ENTRIES.with(|count| count.set(count.get() + 1));
    // Take the callback only after both TLS access and borrowing succeed.
    // Reentry and TLS destruction share the same local-buffer fallback.
    let mut f = Some(f);
    let result = SCRATCH.try_with(|slot| {
        slot.try_borrow_mut().ok().map(|mut sizes| {
            sizes.clear();
            f.take().unwrap()(&mut sizes)
        })
    });
    match result {
        Ok(Some(result)) => result,
        Ok(None) | Err(_) => f.unwrap()(&mut SizeBuffer::default()),
    }
}

#[cfg(feature = "size-cache")]
fn write<M: Message>(msg: &M, buf: &mut impl BufMut, sizes: &SizeBuffer) {
    if sizes.overflow {
        // Preserve the existing usize-sized API rather than truncating to u32.
        msg.encode_raw(buf);
    } else {
        let mut cursor = SizeCursor {
            remaining: &sizes.lengths,
        };
        msg.encode_raw_cached(buf, &mut cursor);
        assert!(
            cursor.remaining.is_empty(),
            "size cache was not fully consumed"
        );
    }
}

#[cfg(feature = "size-cache")]
pub(crate) fn encode<M: Message>(
    msg: &M,
    buf: &mut impl BufMut,
    delimited: bool,
) -> Result<(), crate::EncodeError> {
    with_sizes(|sizes| {
        let len = msg.encoded_len_cached(sizes);
        let prefix = if delimited {
            encoded_len_varint(len as u64)
        } else {
            0
        };
        let required = len.checked_add(prefix).expect("message size overflow");
        let remaining = buf.remaining_mut();
        if required > remaining {
            return Err(crate::EncodeError::new(required, remaining));
        }
        if delimited {
            encode_varint(len as u64, buf);
        }
        write(msg, buf, sizes);
        Ok(())
    })
}

#[cfg(feature = "size-cache")]
pub(crate) fn encode_to_vec<M: Message>(msg: &M, delimited: bool) -> Vec<u8> {
    with_sizes(|sizes| {
        let len = msg.encoded_len_cached(sizes);
        let prefix = if delimited {
            encoded_len_varint(len as u64)
        } else {
            0
        };
        let mut buf = Vec::with_capacity(len.checked_add(prefix).expect("message size overflow"));
        if delimited {
            encode_varint(len as u64, &mut buf);
        }
        write(msg, &mut buf, sizes);
        buf
    })
}

#[cfg(all(test, feature = "size-cache"))]
mod tests {
    use super::*;

    std::thread_local! {
        pub(super) static TLS_ENTRIES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }

    #[cfg(feature = "derive")]
    #[test]
    fn flat_public_encoders_never_enter_tls() {
        #[derive(PartialEq, crate::Message)]
        #[prost(prost_path = "crate")]
        struct Flat {
            #[prost(bytes = "vec", tag = "1")]
            payload: Vec<u8>,
        }
        #[derive(PartialEq, crate::Message)]
        #[prost(prost_path = "crate")]
        struct Parent {
            #[prost(message, repeated, tag = "1")]
            children: Vec<Flat>,
        }
        fn check_entries(msg: &impl Message, expected: usize) {
            TLS_ENTRIES.with(|count| count.set(0));
            let _ = msg.encode_to_vec();
            let _ = msg.encode_length_delimited_to_vec();
            msg.encode(&mut Vec::new()).unwrap();
            msg.encode_length_delimited(&mut Vec::new()).unwrap();
            TLS_ENTRIES.with(|count| assert_eq!(count.get(), expected));
        }
        let leaf = Flat {
            payload: vec![1; 65_536],
        };
        check_entries(&leaf, 0);
        check_entries(&Box::new(Flat::default()), 0);
        check_entries(&7u64, 0);
        check_entries(&(), 0);
        let mut parent = Parent::default();
        check_entries(&parent, 0);
        parent.children.push(leaf);
        check_entries(&parent, 4);
        parent.children.clear();
        check_entries(&parent, 0);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn oversized_lengths_fall_back_and_capacity_is_reused() {
        let mut sizes = SizeBuffer::default();
        sizes.record(u32::MAX as usize);
        assert!(!sizes.overflow);
        sizes.record(u32::MAX as usize + 1);
        assert!(sizes.overflow);
        // An empty message normally consumes exactly one cache slot. With
        // overflow flagged, the original raw writer must bypass this cache.
        write(&(), &mut Vec::new(), &sizes);
        let capacity = sizes.lengths.capacity();
        sizes.clear();
        assert!(!sizes.overflow);
        assert!(sizes.lengths.is_empty());
        assert_eq!(sizes.lengths.capacity(), capacity);
    }
}
