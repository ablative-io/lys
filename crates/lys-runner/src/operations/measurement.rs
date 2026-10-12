//! Checkpoint cost measurements for operation tests.
use std::cell::RefCell;
use std::time::Duration;
#[derive(Default)]
pub(crate) struct Costs {
    pub(crate) encode: Duration,
    pub(crate) read: Duration,
    pub(crate) decode: Duration,
}
thread_local! { static COSTS: RefCell<Option<Costs>> = const { RefCell::new(None) }; }
pub(crate) fn begin() {
    COSTS.with(|costs| *costs.borrow_mut() = Some(Costs::default()));
}
pub(crate) fn finish() -> Option<Costs> {
    COSTS.with(|costs| costs.borrow_mut().take())
}
pub(super) fn encode(elapsed: Duration) {
    COSTS.with(|costs| {
        if let Some(costs) = &mut *costs.borrow_mut() {
            costs.encode += elapsed;
        }
    });
}
pub(super) fn read(elapsed: Duration) {
    COSTS.with(|costs| {
        if let Some(costs) = &mut *costs.borrow_mut() {
            costs.read += elapsed;
        }
    });
}
pub(super) fn decode(elapsed: Duration) {
    COSTS.with(|costs| {
        if let Some(costs) = &mut *costs.borrow_mut() {
            costs.decode += elapsed;
        }
    });
}
