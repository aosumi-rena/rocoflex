use std::{marker::PhantomData,rc::Rc};
use windows::{core::w,Win32::{Foundation::HANDLE,System::Threading::{
    AvRevertMmThreadCharacteristics,AvSetMmThreadCharacteristicsW,GetCurrentThreadId,
}}};

#[derive(Clone,Copy,Debug)]
pub struct SchedulingSnapshot {
    pub worker_thread_id:Option<u32>,pub requested_task:&'static str,
    pub task_index_input:u32,pub task_index_returned:Option<u32>,
    pub registration_attempted:bool,

    pub registration_hresult:Option<i32>,pub registration_succeeded:bool,
    pub revert_hresult:Option<i32>,pub revert_succeeded:Option<bool>,
}
impl Default for SchedulingSnapshot {
    fn default()->Self {
        Self {worker_thread_id:None,requested_task:"Audio",task_index_input:0,
            task_index_returned:None,registration_attempted:false,registration_hresult:None,
            registration_succeeded:false,revert_hresult:None,revert_succeeded:None}
    }
}

pub(crate) trait Api {
    type Handle;
    fn thread_id(&self)->u32;
    fn register(&mut self,index:&mut u32)->std::result::Result<Self::Handle,i32>;
    fn revert(&mut self,handle:Self::Handle)->std::result::Result<(),i32>;
}
pub(crate) struct WindowsAudio;
impl Api for WindowsAudio {
    type Handle=HANDLE;
    fn thread_id(&self)->u32 {unsafe {GetCurrentThreadId()}}
    fn register(&mut self,index:&mut u32)->std::result::Result<HANDLE,i32> {

        unsafe {AvSetMmThreadCharacteristicsW(w!("Audio"),index)}.map_err(|e|e.code().0)
    }
    fn revert(&mut self,handle:HANDLE)->std::result::Result<(),i32> {
        unsafe {AvRevertMmThreadCharacteristics(handle)}.map_err(|e|e.code().0)
    }
}

pub(crate) struct Registration<A:Api=WindowsAudio> {
    api:A,handle:Option<A::Handle>,status:SchedulingSnapshot,_same_thread:PhantomData<Rc<()>>,
}
impl Registration<WindowsAudio> {
    pub(crate) fn new()->Self {Self::with_api(WindowsAudio)}
}
impl<A:Api> Registration<A> {
    pub(crate) fn with_api(api:A)->Self {
        let mut status=SchedulingSnapshot::default();status.worker_thread_id=Some(api.thread_id());
        Self {api,handle:None,status,_same_thread:PhantomData}
    }
    pub(crate) fn snapshot(&self)->SchedulingSnapshot {self.status}
    pub(crate) fn register(&mut self) {
        if self.status.registration_attempted {return;}
        self.status.registration_attempted=true;
        let mut index=0;
        let result=self.api.register(&mut index);
        self.status.task_index_returned=Some(index);
        match result {
            Ok(handle)=>{self.handle=Some(handle);self.status.registration_hresult=Some(0);self.status.registration_succeeded=true;},
            Err(error)=>{self.status.registration_hresult=Some(error);},
        }
    }

    pub(crate) fn revert(&mut self) {
        let Some(handle)=self.handle.take() else {return;};
        match self.api.revert(handle) {
            Ok(())=>{self.status.revert_hresult=Some(0);self.status.revert_succeeded=Some(true);},
            Err(error)=>{self.status.revert_hresult=Some(error);self.status.revert_succeeded=Some(false);},
        }
    }
}
impl<A:Api> Drop for Registration<A> {fn drop(&mut self) {self.revert();}}

