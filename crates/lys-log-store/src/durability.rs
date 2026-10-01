//! Keep the durable operation observable without replacing it in tests.

#[cfg(any(test, feature = "flush-counts"))]
std::thread_local! {
    static FLUSHES: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
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
    file.sync_all()
}
