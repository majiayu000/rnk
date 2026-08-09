use std::cell::Cell;

thread_local! {
    static ATTEMPTS: Cell<usize> = const { Cell::new(0) };
}

pub(super) fn record_attempt() {
    ATTEMPTS.with(|attempts| attempts.set(attempts.get() + 1));
}

pub(in crate::layout::engine) fn take_attempts() -> usize {
    ATTEMPTS.with(|attempts| attempts.replace(0))
}
