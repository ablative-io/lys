//! Keep the durable operation observable without replacing it in tests.

#[cfg(any(test, feature = "flush-counts"))]
std::thread_local! {
    static FLUSHES: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

#[cfg(feature = "flush-counts")]
static PROCESS_FLUSHES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Number of real flush attempts across this process, including spawned tasks.
/// A difference measures one startup only when the test owns the process.
#[cfg(feature = "flush-counts")]
pub fn process_flush_count() -> u64 {
    PROCESS_FLUSHES.load(std::sync::atomic::Ordering::Relaxed)
}

/// Number of real file and directory flush attempts on the calling thread.
/// Measure synchronous store work, or use a current-thread async runtime.
#[cfg(any(test, feature = "flush-counts"))]
pub fn flush_count() -> u64 {
    FLUSHES.get()
}

pub(crate) fn sync_all(file: &std::fs::File) -> std::io::Result<()> {
    #[cfg(any(test, feature = "flush-counts"))]
    FLUSHES.set(FLUSHES.get() + 1);
    #[cfg(feature = "flush-counts")]
    PROCESS_FLUSHES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    file.sync_all()
}
