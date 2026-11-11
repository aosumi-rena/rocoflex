use super::Services;

pub struct BorrowedServices<'a, S: Services + ?Sized> { services: &'a mut S }
impl<S: Services + ?Sized> BorrowedServices<'_, S> {
    pub fn services(&self) -> &S { &*self.services }
}

pub fn from_actual_record<'a, S: Services + ?Sized>(services: &'a mut S,
    actual_rcx_slot: u64, actual_source_image_base: u64)
    -> super::Prepared<BorrowedServices<'a, S>> {
    super::Prepared::at_1a7310(BorrowedServices { services }, actual_rcx_slot, actual_source_image_base)
}

pub fn from_actual_static_callback<'a, S: Services + ?Sized>(services: &'a mut S,
    actual_source_image_base: u64) -> super::Prepared<BorrowedServices<'a, S>> {
    super::Prepared::at_1cf040(BorrowedServices { services }, actual_source_image_base)
}
impl<S: Services + ?Sized> Services for BorrowedServices<'_, S> {
    type Error = S::Error;
    fn read_u64(&mut self, address: u64) -> Result<u64, Self::Error> {
        self.services.read_u64(address)
    }
    fn lock_xadd_minus_one(&mut self, header: u64, held: &mut Option<u32>) -> Result<(), Self::Error> {
        self.services.lock_xadd_minus_one(header, held)
    }
    fn tail_free55da4c(&mut self, saved_header: u64) -> Result<(), Self::Error> {
        self.services.tail_free55da4c(saved_header)
    }
}
