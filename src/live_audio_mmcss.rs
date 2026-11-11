use super::{Snapshot,lock};
use std::sync::{Arc,Mutex};
pub use crate::audio_scheduling::SchedulingSnapshot;

pub(super) struct Registration {
    inner:crate::audio_scheduling::Registration,
    shared:Arc<Mutex<Snapshot>>,
}
impl Registration {
    pub fn new(shared:Arc<Mutex<Snapshot>>)->Self {
        let inner=crate::audio_scheduling::Registration::new();
        lock(&shared).scheduling.worker_thread_id=inner.snapshot().worker_thread_id;
        Self {inner,shared}
    }
    pub fn register(&mut self) {
        {
            let mut status=lock(&self.shared);
            if status.scheduling.registration_attempted {return;}
            status.scheduling.registration_attempted=true;
        }
        self.inner.register();
        let registered=self.inner.snapshot();
        let mut status=lock(&self.shared);
        status.scheduling.task_index_returned=registered.task_index_returned;
        status.scheduling.registration_hresult=registered.registration_hresult;
        status.scheduling.registration_succeeded=registered.registration_succeeded;

    }
}
impl Drop for Registration {
    fn drop(&mut self) {
        if !self.inner.snapshot().registration_succeeded {return;}
        self.inner.revert();let retired=self.inner.snapshot();
        let mut status=lock(&self.shared);
        status.scheduling.revert_hresult=retired.revert_hresult;
        status.scheduling.revert_succeeded=retired.revert_succeeded;
    }
}
