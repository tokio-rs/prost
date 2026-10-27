use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

use prost::Message;

const DEFAULT_MESSAGE_COUNT: usize = 4_096;

struct CountingAllocator;

thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
}

static ALLOCATION_CALLS: AtomicUsize = AtomicUsize::new(0);
static REALLOCATION_CALLS: AtomicUsize = AtomicUsize::new(0);
static REQUESTED_ALLOCATION_BYTES: AtomicUsize = AtomicUsize::new(0);

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count_allocation(layout.size(), false);
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count_allocation(layout.size(), false);
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count_allocation(new_size, true);
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

// A reallocation is counted at its new requested size. This measures requested
// allocation traffic, not whether a particular allocator moved the allocation.
fn count_allocation(bytes: usize, is_reallocation: bool) {
    COUNTING.with(|counting| {
        if counting.get() {
            ALLOCATION_CALLS.fetch_add(1, Ordering::Relaxed);
            if is_reallocation {
                REALLOCATION_CALLS.fetch_add(1, Ordering::Relaxed);
            }
            REQUESTED_ALLOCATION_BYTES.fetch_add(bytes, Ordering::Relaxed);
        }
    });
}

#[derive(Clone, PartialEq, Message)]
struct Nested {
    #[prost(uint32, tag = "1")]
    value: u32,
}

#[derive(Clone, PartialEq, Message)]
struct RepeatedMessages {
    #[prost(message, repeated, tag = "1")]
    values: Vec<Nested>,
}

fn message_count() -> usize {
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--message-count" {
            return arguments
                .next()
                .expect("missing message count")
                .parse()
                .expect("message count must be an integer");
        }
    }

    DEFAULT_MESSAGE_COUNT
}

fn main() {
    let message_count = message_count();
    let expected = RepeatedMessages {
        values: (0..message_count)
            .map(|value| Nested {
                value: value as u32 + 1,
            })
            .collect(),
    };
    let input = expected.encode_to_vec();

    ALLOCATION_CALLS.store(0, Ordering::Relaxed);
    REALLOCATION_CALLS.store(0, Ordering::Relaxed);
    REQUESTED_ALLOCATION_BYTES.store(0, Ordering::Relaxed);
    COUNTING.with(|counting| counting.set(true));
    let decoded = RepeatedMessages::decode(std::hint::black_box(input.as_slice())).unwrap();
    COUNTING.with(|counting| counting.set(false));

    assert_eq!(std::hint::black_box(decoded), expected);
    println!(
        "allocation_calls: {}",
        ALLOCATION_CALLS.load(Ordering::Relaxed)
    );
    println!(
        "reallocation_calls: {}",
        REALLOCATION_CALLS.load(Ordering::Relaxed)
    );
    println!(
        "requested_allocation_bytes: {}",
        REQUESTED_ALLOCATION_BYTES.load(Ordering::Relaxed)
    );
}
