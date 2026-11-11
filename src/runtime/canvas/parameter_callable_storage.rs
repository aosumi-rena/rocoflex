use super::{Entry, History};
use super::super::super::super::super as function_owner;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReturnRequest {
    pub entry: Entry, pub ret_pc: u32, pub entry_rsp: u64,
    pub source_word: u64, pub destination_word: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActualReturn { pub ret_pc: u32, pub entry_rsp: u64, pub caller_rsp: u64, pub target: u64, pub rax: u64 }
pub trait Services: function_owner::Services {

    fn selected_write_word(&mut self, source: &mut Self::Function, parameter: &mut Self::Parameter,
        frame: &mut Self::Frame, address: u64, word: u64) -> Result<(), Self::Error>;
    fn selected_read_dword(&mut self, source: &mut Self::Function, parameter: &mut Self::Parameter,
        frame: &mut Self::Frame, address: u64) -> Result<u32, Self::Error>;
    fn selected_write_dword(&mut self, source: &mut Self::Function, parameter: &mut Self::Parameter,
        frame: &mut Self::Frame, address: u64, bits: u32) -> Result<(), Self::Error>;

    fn selected_actual_return(&mut self, source: &mut Self::Function, parameter: &mut Self::Parameter,
        frame: &mut Self::Frame, request: ReturnRequest, body: History,
        held: &mut Option<function_owner::CloneReturn<Self::CloneOwner>>,
        returned: &mut Option<ActualReturn>) -> Result<(), Self::Error>;
}

pub trait NativeEntryServices: Services {
    fn selected_prefix_instruction(&mut self, source: &mut Self::Function, parameter: &mut Self::Parameter,
        frame: &mut Self::Frame, request: super::native_entry::Request,
        returned: &mut Option<super::native_entry::ActualInstruction>) -> Result<(), Self::Error>;
}
pub struct Storage<'a, S: Services> where S::Function: 'a, S::Parameter: 'a, S::Frame: 'a {
    pub source: &'a mut S::Function, pub parameter: &'a mut S::Parameter, pub frame: &'a mut S::Frame,
}
pub struct Memory<'a, S: Services> { pub services: &'a mut S }
impl<'a, S: Services> super::Memory for Memory<'a, S> where S::Function: 'a, S::Parameter: 'a, S::Frame: 'a {
    type Storage = Storage<'a, S>;
    type Error = S::Error;
    fn write_word(&mut self, storage: &mut Self::Storage, address: u64, word: u64) -> Result<(), Self::Error> {
        self.services.selected_write_word(&mut *storage.source, &mut *storage.parameter, &mut *storage.frame, address, word)
    }
    fn read_dword(&mut self, storage: &mut Self::Storage, address: u64) -> Result<u32, Self::Error> {
        self.services.selected_read_dword(&mut *storage.source, &mut *storage.parameter, &mut *storage.frame, address)
    }
    fn write_dword(&mut self, storage: &mut Self::Storage, address: u64, bits: u32) -> Result<(), Self::Error> {
        self.services.selected_write_dword(&mut *storage.source, &mut *storage.parameter, &mut *storage.frame, address, bits)
    }
}
impl<'a, S: NativeEntryServices> super::native_entry::Services for Memory<'a, S>
    where S::Function: 'a, S::Parameter: 'a, S::Frame: 'a {
    fn actual_prefix_instruction(&mut self, storage: &mut Self::Storage, request: super::native_entry::Request,
        returned: &mut Option<super::native_entry::ActualInstruction>) -> Result<(), Self::Error> {
        self.services.selected_prefix_instruction(&mut *storage.source, &mut *storage.parameter,
            &mut *storage.frame, request, returned)
    }
}
