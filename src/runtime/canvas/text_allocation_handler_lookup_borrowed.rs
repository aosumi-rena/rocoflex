use super::Services;
pub struct BorrowedServices<'a, S: Services + ?Sized> { services: &'a mut S }
impl<S: Services + ?Sized> BorrowedServices<'_, S> {
    pub fn services(&self) -> &S { &*self.services }
}

pub fn from_actual_576624<'a, S: Services + ?Sized + 'a>(services: &'a mut S,
    owners: &'a mut S::Owners, actual_image_base: u64)
    -> super::Prepared<BorrowedServices<'a, S>> where S::Owners: 'a {
    super::Prepared::from_actual_576624(BorrowedServices { services },
        super::Input { owners, actual_image_base })
}
impl<'a, S: Services + ?Sized + 'a> Services for BorrowedServices<'a, S> where S::Owners: 'a {
    type Owners = &'a mut S::Owners;
    type Error = S::Error;
    fn call57a7a8(&mut self, owners: &mut Self::Owners, actual_ecx: u32) -> Result<(), Self::Error> {
        self.services.call57a7a8(&mut **owners, actual_ecx)
    }
    fn read_word(&mut self, owners: &mut Self::Owners, address: u64) -> Result<u64, Self::Error> {
        self.services.read_word(&mut **owners, address)
    }
    fn call57a7c4(&mut self, owners: &mut Self::Owners, actual_ecx: u32) -> Result<(), Self::Error> {
        self.services.call57a7c4(&mut **owners, actual_ecx)
    }
}
