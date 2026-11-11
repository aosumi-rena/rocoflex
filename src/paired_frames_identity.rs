use std::sync::Arc;

pub struct PoolIdentity { tag: Arc<()> }
impl PoolIdentity {

    pub fn new_value_owner() -> Self { Self { tag: Arc::new(()) } }
}

pub struct CheckoutOrigin { tag: Arc<()> }
impl CheckoutOrigin {
    pub(super) fn from_owner(owner: &PoolIdentity) -> Self { Self { tag: Arc::clone(&owner.tag) } }
    pub fn matches(&self, owner: &PoolIdentity) -> bool { Arc::ptr_eq(&self.tag, &owner.tag) }
}
