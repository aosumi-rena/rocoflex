use super as tail;
use super::super as lookup;

pub struct Borrowed<'a, S: tail::Services> where S::Owners: 'a { services: &'a mut S }
pub fn from_actual_tail<'a, S: tail::Services>(services: &'a mut S, owners: &'a mut S::Owners,
    actual: tail::ActualTail) -> Result<tail::Prepared<Borrowed<'a, S>>, tail::Rejected<Borrowed<'a, S>>> where S::Owners: 'a {
    tail::Prepared::from_actual_indirect_tail(tail::Input { services: Borrowed { services }, owners, actual })
}
impl<'a, S: tail::Services> lookup::Services for Borrowed<'a, S> where S::Owners: 'a {
    type Owners = &'a mut S::Owners; type Error = S::Error;
    fn call57a7a8(&mut self, owners: &mut Self::Owners, actual_ecx: u32) -> Result<(), Self::Error> {
        self.services.call57a7a8(&mut **owners, actual_ecx)
    }
    fn read_word(&mut self, owners: &mut Self::Owners, actual_address: u64) -> Result<u64, Self::Error> {
        self.services.read_word(&mut **owners, actual_address)
    }
    fn call57a7c4(&mut self, owners: &mut Self::Owners, actual_ecx: u32) -> Result<(), Self::Error> {
        self.services.call57a7c4(&mut **owners, actual_ecx)
    }
}
impl<'a, S: tail::Services> tail::Services for Borrowed<'a, S> where S::Owners: 'a {
    type IndexedTailOwner = S::IndexedTailOwner;
    fn read_indexed_target(&mut self, owners: &mut Self::Owners, pc: u32, slot: u64) -> Result<u64, Self::Error> {
        self.services.read_indexed_target(&mut **owners, pc, slot)
    }
    fn indexed_whole_tail(&mut self, owners: &mut Self::Owners, request: tail::Request,
        held: &mut Option<tail::ReturnedTail<Self::IndexedTailOwner>>) -> Result<(), Self::Error> {
        self.services.indexed_whole_tail(&mut **owners, request, held)
    }
}
