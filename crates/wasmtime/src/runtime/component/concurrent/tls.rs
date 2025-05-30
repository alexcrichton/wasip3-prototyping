//! TODO

use crate::runtime::vm::VMStore;
use core::cell::Cell;
use core::ptr::NonNull;

std::thread_local! {
    static STORE: Cell<Option<NonNull<dyn VMStore>>> = const { Cell::new(None) };
}

struct Reset(Option<NonNull<dyn VMStore>>);

impl Drop for Reset {
    fn drop(&mut self) {
        STORE.with(|s| s.set(self.0));
    }
}

/// TODO
pub fn set<R>(store: &mut dyn VMStore, f: impl FnOnce() -> R) -> R {
    let _reset = Reset(STORE.with(|s| s.replace(Some(store.into()))));
    f()
}

/// TODO
pub fn get<R>(f: impl FnOnce(&mut dyn VMStore) -> R) -> R {
    let reset = Reset(STORE.with(|s| s.replace(None)));
    // SAFETY: TODO
    f(unsafe { reset.0.unwrap().as_mut() })
}
