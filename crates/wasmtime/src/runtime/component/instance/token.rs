use crate::StoreContextMut;
use crate::runtime::vm::component::ComponentInstance;
use crate::store::StoreId;
use crate::vm::SendSyncPtr;

/// TODO
#[derive(Copy, Clone)]
pub struct InstanceToken {
    ptr: SendSyncPtr<ComponentInstance>,
    store: StoreId,
}

impl InstanceToken {
    // pub fn new<T>(store: StoreContextMut<'_, T>, instance

    /// TODO
    pub fn get_mut<'a, T>(&self, store: StoreContextMut<'a, T>) -> &'a mut ComponentInstance {
        assert_eq!(store.0.id(), self.store);
        unsafe { self.ptr.as_mut() }
    }
}
