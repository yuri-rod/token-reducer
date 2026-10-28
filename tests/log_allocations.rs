use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use token_reducer::compressors::logs::compress_logs;

struct TrackedAllocator;
static LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);

// This integration-test binary has one test, so no other tests allocate while measured.
#[global_allocator]
static ALLOCATOR: TrackedAllocator = TrackedAllocator;

unsafe impl GlobalAlloc for TrackedAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = System.alloc(layout);
        if !pointer.is_null() {
            LIVE_BYTES.fetch_add(layout.size(), Ordering::SeqCst);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        System.dealloc(pointer, layout);
        LIVE_BYTES.fetch_sub(layout.size(), Ordering::SeqCst);
    }
}

#[test]
fn repeated_stack_frame_reductions_release_their_allocations() {
    let input =
        "Error: failure\n  at node:internal/main\n  at app::work\n  at std::panicking::begin\n";
    let expected = "Error: failure\n    [... 1 internal runtime stack frames omitted ...]\n  at app::work\n    [... 1 internal runtime stack frames omitted ...]";

    // Initialize the shared ANSI regex and its caches before measuring retained bytes.
    for _ in 0..4 {
        assert_eq!(compress_logs(input), expected);
    }
    let before = LIVE_BYTES.load(Ordering::SeqCst);
    for _ in 0..32 {
        assert_eq!(compress_logs(input), expected);
    }
    let after = LIVE_BYTES.load(Ordering::SeqCst);
    assert_eq!(
        after, before,
        "compression retained allocations after returning"
    );
}
