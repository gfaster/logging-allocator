use std::alloc::{GlobalAlloc, Layout, System};
use std::backtrace::Backtrace;
use std::cell::Cell;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(feature = "warn")]
const WARNING_THRESHOLD: usize = 1_000_000;

/// A wrapper allocator that logs messages on allocation.
pub struct LoggingAllocator<A = System> {
    enabled: AtomicBool,
    allocator: A,
}

impl LoggingAllocator<System> {
    pub const fn new(enabled: bool) -> Self {
        LoggingAllocator::with_allocator(System, enabled)
    }
}

impl<A> LoggingAllocator<A> {
    pub const fn with_allocator(allocator: A, enabled: bool) -> Self {
        LoggingAllocator {
            enabled: AtomicBool::new(enabled),
            allocator,
        }
    }

    pub fn enable_logging(&self) {
        self.enabled.store(true, Ordering::SeqCst)
    }

    pub fn disable_logging(&self) {
        self.enabled.store(false, Ordering::SeqCst)
    }

    pub fn logging_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }
}

/// Execute a closure without logging on allocations.
pub fn run_guarded<F>(f: F)
where
    F: FnOnce(),
{
    thread_local! {
        static GUARD: Cell<bool> = Cell::new(false);
    }

    GUARD.with(|guard| {
        if !guard.replace(true) {
            f();
            guard.set(false)
        }
    })
}

unsafe impl<A> GlobalAlloc for LoggingAllocator<A>
where
    A: GlobalAlloc,
{
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        #[cfg(feature = "warn")]
        {
            if layout.size() > WARNING_THRESHOLD {
                eprintln!("large allocation at {:?}", backtrace::Backtrace::new());
            }
        }
        let ptr = self.allocator.alloc(layout);
        if self.logging_enabled() {
            run_guarded(|| {
                eprintln!("alloc {}", Fmt(ptr, layout.size(), layout.align(), true));
            });
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.allocator.dealloc(ptr, layout);
        if self.logging_enabled() {
            run_guarded(|| eprintln!("dealloc {}", Fmt(ptr, layout.size(), layout.align(), true),));
        }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = self.allocator.alloc_zeroed(layout);
        if self.logging_enabled() {
            run_guarded(|| {
                eprintln!("alloc_zeroed {}", Fmt(ptr, layout.size(), layout.align(), true));
            });
        }
        ptr
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        #[cfg(feature = "warn")]
        {
            if new_size > WARNING_THRESHOLD {
                eprintln!("large reallocation at {:?}", backtrace::Backtrace::new());
            }
        }
        let new_ptr = self.allocator.realloc(ptr, layout, new_size);
        if self.logging_enabled() {
            run_guarded(|| {
                eprintln!(
                    "realloc {} to {}",
                    Fmt(ptr, layout.size(), layout.align(), false),
                    Fmt(new_ptr, new_size, layout.align(), true)
                );
            });
        }
        new_ptr
    }
}

struct Fmt(*mut u8, usize, usize, bool);

impl fmt::Display for Fmt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.3 {
            write!(
                f,
                "[address={:p}, size={}, align={}] at:\n{:}",
                self.0, self.1, self.2, Backtrace::capture()
            )
        } else {
            write!(
                f,
                "[address={:p}, size={}, align={}]",
                self.0, self.1, self.2
            )
        }
    }
}
