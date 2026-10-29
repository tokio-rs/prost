#![cfg(all(feature = "size-cache", feature = "derive"))]

use prost::{
    bytes::{Buf, BufMut},
    encoding::{DecodeContext, WireType},
    DecodeError, Message,
};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, PartialEq, Message)]
struct Node {
    // Deliberately declare fields out of tag order.
    #[prost(message, repeated, tag = "4")]
    children: Vec<Node>,
    #[prost(bytes = "vec", tag = "1")]
    payload: Vec<u8>,
    #[prost(message, optional, boxed, tag = "2")]
    next: Option<Box<Node>>,
    #[prost(uint64, repeated, packed = "true", tag = "3")]
    values: Vec<u64>,
}

#[derive(Clone, PartialEq, prost::Oneof)]
enum Choice {
    #[prost(message, tag = "8")]
    Node(Box<Node>),
    #[prost(string, tag = "3")]
    Text(String),
}

#[derive(Clone, PartialEq, Message)]
struct Envelope {
    #[prost(map = "string, message", tag = "1")]
    hash: HashMap<String, Node>,
    #[prost(btree_map = "int32, message", tag = "2")]
    tree: BTreeMap<i32, Node>,
    #[prost(oneof = "Choice", tags = "3, 8")]
    choice: Option<Choice>,
    #[prost(message, required, tag = "4")]
    required: Node,
    #[prost(group, optional, tag = "5")]
    group: Option<Node>,
    #[prost(group, repeated, tag = "6")]
    groups: Vec<Node>,
    #[prost(map = "int32, string", tag = "7")]
    scalars: HashMap<i32, String>,
    #[prost(message, optional, tag = "9")]
    wrapper: Option<u64>,
}

fn tree(depth: usize, width: usize, payload: usize) -> Node {
    Node {
        payload: vec![17; payload],
        values: vec![0, 127, 128, 16_383, 16_384, u64::MAX],
        children: if depth > 1 {
            (0..width)
                .map(|_| tree(depth - 1, width, payload))
                .collect()
        } else {
            vec![]
        },
        next: None,
    }
}

fn check<M: Message + Default + PartialEq + std::fmt::Debug>(msg: &M) {
    let mut original = Vec::new();
    msg.encode_raw(&mut original); // Original uncached path remains the oracle.
    assert_eq!(original.len(), msg.encoded_len());
    assert_eq!(original, msg.encode_to_vec());
    let mut reused = vec![42];
    msg.encode(&mut reused).unwrap();
    assert_eq!(&reused[1..], original);
    let mut delimited = Vec::new();
    prost::encoding::encode_varint(original.len() as u64, &mut delimited);
    delimited.extend_from_slice(&original);
    assert_eq!(delimited, msg.encode_length_delimited_to_vec());
    let mut encoded = Vec::new();
    msg.encode_length_delimited(&mut encoded).unwrap();
    assert_eq!(encoded, delimited);
    assert_eq!(M::decode(original.as_slice()).unwrap(), *msg);
    assert_eq!(
        M::decode_length_delimited(delimited.as_slice()).unwrap(),
        *msg
    );
    if !original.is_empty() {
        let mut short = vec![91; original.len() - 1];
        assert!(msg.encode(&mut short.as_mut_slice()).is_err());
        assert!(short.iter().all(|&byte| byte == 91));
    }
    let mut short = vec![91; delimited.len() - 1];
    assert!(msg
        .encode_length_delimited(&mut short.as_mut_slice())
        .is_err());
    assert!(short.iter().all(|&byte| byte == 91));
}

#[test]
fn trees_boundaries_and_mutations() {
    check(&Node::default());
    for payload in [0, 1, 127, 128, 16_383, 16_384] {
        check(&tree(3, 3, payload));
    }
    let mut deep = tree(64, 1, 3);
    check(&deep);
    let mut child = &mut deep;
    while !child.children.is_empty() {
        child = &mut child.children[0];
    }
    child.payload.resize(16_384, 42);
    check(&deep);
    deep.children.clear();
    check(&deep);
    for _ in 0..32 {
        deep = Node {
            next: Some(Box::new(deep)),
            ..Default::default()
        };
    }
    check(&Box::new(deep));
}

#[derive(Clone, PartialEq, Message)]
struct PackedNumbers {
    #[prost(uint64, repeated, tag = "1")]
    u64s: Vec<u64>,
    #[prost(sint32, repeated, tag = "2")]
    s32s: Vec<i32>,
    #[prost(int32, repeated, tag = "3")]
    i32s: Vec<i32>,
    #[prost(fixed64, repeated, tag = "4")]
    fixed: Vec<u64>,
    #[prost(uint32, repeated, tag = "5")]
    u32s: Vec<u32>,
    #[prost(sint64, repeated, tag = "6")]
    s64s: Vec<i64>,
    #[prost(int64, repeated, tag = "7")]
    i64s: Vec<i64>,
    #[prost(enumeration = "NumberKind", repeated, tag = "8")]
    enums: Vec<i32>,
    #[prost(bool, repeated, tag = "9")]
    bools: Vec<bool>,
    #[prost(uint64, repeated, packed = "false", tag = "10")]
    unpacked: Vec<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, prost::Enumeration)]
enum NumberKind {
    Zero = 0,
    Negative = -1,
    Large = 16384,
}

#[derive(Clone, PartialEq, Message)]
struct PackedEnvelope {
    #[prost(message, repeated, tag = "1")]
    values: Vec<PackedNumbers>,
    #[prost(uint64, repeated, tag = "2")]
    middle: Vec<u64>,
    #[prost(message, optional, tag = "3")]
    tail: Option<PackedNumbers>,
}

proptest::proptest! {
    #[test]
    fn packed_types_empty_arrays_and_mutations(
        unsigned in proptest::collection::vec(proptest::num::u64::ANY, 0..128),
        signed in proptest::collection::vec(proptest::num::i64::ANY, 0..128),
    ) {
        let leaf = PackedNumbers {
            u64s: unsigned.clone(),
            s32s: signed.iter().map(|&n| n as i32).collect(),
            i32s: signed.iter().map(|&n| n as i32).collect(),
            fixed: unsigned.clone(),
            u32s: unsigned.iter().map(|&n| n as u32).collect(),
            s64s: signed.clone(),
            i64s: signed.clone(),
            // Include unknown enum values, which must preserve their bits.
            enums: signed.iter().map(|&n| n as i32).collect(),
            bools: unsigned.iter().map(|n| n & 1 != 0).collect(),
            unpacked: unsigned,
        };
        assert!(!leaf.use_size_cache());
        check(&leaf);
        let mut parent = PackedEnvelope {
            values: vec![PackedNumbers::default(), leaf.clone(), PackedNumbers::default()],
            middle: vec![0, 127, 128, 16383, 16384, u64::MAX],
            tail: Some(leaf),
        };
        check(&parent);
        parent.values[1] = PackedNumbers::default();
        parent.middle.clear();
        parent.tail.as_mut().unwrap().u64s.resize(129, u64::MAX);
        check(&parent);
        parent.values.clear();
        parent.tail = None;
        assert!(!parent.use_size_cache());
        check(&parent);
    }
}

#[test]
fn maps_oneofs_groups_and_default_values() {
    let mut envelope = Envelope {
        hash: [
            (String::new(), Node::default()),
            ("empty-value".into(), Node::default()),
            ("nested".into(), tree(3, 2, 4)),
        ]
        .into(),
        tree: [
            (0, tree(3, 2, 7)),
            (1, Node::default()),
            (-1, tree(2, 3, 0)),
        ]
        .into(),
        required: tree(3, 2, 17),
        group: Some(tree(2, 3, 0)),
        groups: vec![Node::default(), tree(3, 2, 127)],
        scalars: [(0, String::new()), (1, "hello".into())].into(),
        wrapper: Some(0),
        choice: None,
    };
    for choice in [
        None,
        Some(Choice::Text(String::new())),
        Some(Choice::Text("foo".into())),
        Some(Choice::Node(Box::default())),
        Some(Choice::Node(Box::new(tree(3, 2, 128)))),
    ] {
        envelope.choice = choice;
        check(&envelope);
    }
    envelope.hash.insert(String::new(), tree(4, 2, 7));
    envelope.tree.remove(&0);
    envelope.wrapper = Some(u64::MAX);
    check(&envelope);
}

// Handwritten implementations retain source compatibility through the default
// cache methods. This one deliberately starts a second public encode operation.
#[derive(Debug, Default, PartialEq)]
struct Custom(Node);
impl Message for Custom {
    fn encode_raw(&self, buf: &mut impl BufMut) {
        buf.put_slice(&self.0.encode_to_vec());
    }
    fn encoded_len(&self) -> usize {
        self.0.encoded_len()
    }
    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        self.0.merge_field(tag, wire_type, buf, ctx)
    }
    fn clear(&mut self) {
        self.0.clear();
    }
}

#[test]
fn handwritten_message_reentry_and_threads() {
    check(&Custom(tree(4, 2, 127)));
    let threads: Vec<_> = (0..4)
        .map(|n| {
            std::thread::spawn(move || {
                for i in 0..20 {
                    check(&tree(4, 2, n * 100 + i));
                }
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
}

#[test]
fn panic_releases_tls_borrow() {
    #[derive(Debug)]
    struct Panic;
    impl Message for Panic {
        fn encode_raw(&self, _: &mut impl BufMut) {
            panic!("intentional encode panic");
        }
        fn encoded_len(&self) -> usize {
            0
        }
        fn merge_field(
            &mut self,
            _: u32,
            _: WireType,
            _: &mut impl Buf,
            _: DecodeContext,
        ) -> Result<(), DecodeError> {
            unreachable!()
        }
        fn clear(&mut self) {}
    }
    assert!(std::panic::catch_unwind(|| Panic.encode_to_vec()).is_err());
    check(&tree(4, 2, 3));
}

#[test]
fn public_encoders_work_during_thread_local_destruction() {
    use std::cell::RefCell;
    use std::sync::mpsc;

    struct EncodeOnDrop(mpsc::Sender<bool>);
    impl Drop for EncodeOnDrop {
        fn drop(&mut self) {
            // Catch a failure here so the test reports it instead of aborting
            // the process on an unwinding thread-local destructor.
            let success = std::panic::catch_unwind(|| check(&tree(3, 2, 127))).is_ok();
            let _ = self.0.send(success);
        }
    }

    std::thread_local! {
        static ENCODE_ON_DROP: RefCell<Option<EncodeOnDrop>> = const { RefCell::new(None) };
    }

    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || {
        // Initialize our key first, so the encoder's scratch is destroyed
        // before this key on platforms that run TLS destructors in reverse order.
        ENCODE_ON_DROP.with(|slot| *slot.borrow_mut() = Some(EncodeOnDrop(send)));
        check(&tree(2, 2, 128));
    })
    .join()
    .unwrap();
    assert!(
        receive.recv().unwrap(),
        "encoding during TLS destruction failed"
    );
}

#[test]
fn deeply_nested_leaf_size_is_computed_once_per_encode() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    #[derive(Debug, Default, PartialEq)]
    struct Leaf;
    impl Message for Leaf {
        fn encode_raw(&self, _: &mut impl BufMut) {}
        fn encoded_len(&self) -> usize {
            CALLS.fetch_add(1, Ordering::Relaxed);
            0
        }
        fn merge_field(
            &mut self,
            _: u32,
            _: WireType,
            _: &mut impl Buf,
            _: DecodeContext,
        ) -> Result<(), DecodeError> {
            unreachable!()
        }
        fn clear(&mut self) {}
    }
    #[derive(PartialEq, Message)]
    struct Layer {
        #[prost(message, optional, boxed, tag = "1")]
        next: Option<Box<Layer>>,
        #[prost(message, repeated, tag = "2")]
        leaves: Vec<Leaf>,
    }
    let mut msg = Layer {
        leaves: (0..32).map(|_| Leaf).collect(),
        next: None,
    };
    for _ in 0..63 {
        msg = Layer {
            next: Some(Box::new(msg)),
            leaves: vec![],
        };
    }
    msg.encode_to_vec();
    assert_eq!(CALLS.swap(0, Ordering::Relaxed), 32);
    msg.encode_to_vec();
    assert_eq!(CALLS.swap(0, Ordering::Relaxed), 32);
    let mut old = Vec::with_capacity(msg.encoded_len());
    msg.encode_raw(&mut old);
    assert_eq!(CALLS.load(Ordering::Relaxed), 32 * 65);
}

#[test]
fn cache_selection_tracks_actual_field_presence() {
    #[derive(Clone, PartialEq, Message)]
    struct Switch {
        #[prost(message, optional, boxed, tag = "1")]
        optional: Option<Box<Node>>,
        #[prost(message, repeated, tag = "2")]
        repeated: Vec<Node>,
        #[prost(oneof = "Choice", tags = "3, 8")]
        choice: Option<Choice>,
        #[prost(map = "string, message", tag = "4")]
        hash: HashMap<String, Node>,
        #[prost(btree_map = "int32, message", tag = "5")]
        btree: BTreeMap<i32, Node>,
        #[prost(group, optional, tag = "6")]
        group: Option<Node>,
        #[prost(group, repeated, tag = "7")]
        groups: Vec<Node>,
    }
    fn selected(msg: &Switch, expected: bool) {
        assert_eq!(msg.use_size_cache(), expected);
        assert_eq!(Message::use_size_cache(&Box::new(msg.clone())), expected);
        check(msg);
    }
    let mut msg = Switch::default();
    selected(&msg, false);
    msg.choice = Some(Choice::Text("scalar oneof".into()));
    selected(&msg, false);
    msg.choice = Some(Choice::Node(Box::default()));
    selected(&msg, true);
    msg.clear();
    msg.optional = Some(Box::default());
    selected(&msg, true);
    msg.clear();
    msg.repeated.push(Node::default());
    selected(&msg, true);
    msg.clear();
    msg.hash.insert(String::new(), Node::default());
    selected(&msg, true);
    msg.clear();
    msg.btree.insert(0, Node::default());
    selected(&msg, true);
    msg.clear();
    msg.group = Some(Node::default());
    selected(&msg, true);
    msg.clear();
    msg.groups.push(Node::default());
    selected(&msg, true);
    msg.clear();
    selected(&msg, false);
    assert!(Envelope::default().use_size_cache()); // Required message is present.
    assert!(Custom::default().use_size_cache()); // Unknown implementation.

    #[derive(Clone, PartialEq, Message)]
    struct Flat {
        #[prost(bytes = "vec", tag = "1")]
        payload: Vec<u8>,
        #[prost(uint64, repeated, packed = "true", tag = "2")]
        values: Vec<u64>,
        #[prost(map = "string, string", tag = "3")]
        scalars: HashMap<String, String>,
    }
    let flat = Flat {
        payload: vec![1; 65_536],
        values: vec![u64::MAX; 128],
        scalars: [("key".into(), "value".into())].into(),
    };
    assert!(!flat.use_size_cache());
    check(&flat);
}
