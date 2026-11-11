#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResetStage { Module40, Module48, Module56, Module64, EmptyOwners, Complete }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResetProgress { pub stage: ResetStage, pub failed: bool }
pub struct ResetWork { progress: ResetProgress }
impl ResetWork {
    pub fn after_admission() -> Self {
        Self { progress: ResetProgress { stage: ResetStage::Module40, failed: false } }
    }
    pub fn progress(&self) -> ResetProgress { self.progress }
}
pub trait ResetServices {
    type Error;
    fn reset_module40(&mut self) -> Result<(), Self::Error>;
    fn reset_module48(&mut self) -> Result<(), Self::Error>;
    fn reset_module56(&mut self) -> Result<(), Self::Error>;
    fn reset_module64(&mut self) -> Result<(), Self::Error>;
    fn finish_empty_owners(&mut self) -> Result<(), Self::Error>;
}
pub enum ResetError<E> { PriorFailure, AlreadyComplete, Service(E) }

pub struct ResetComplete { _private: () }
pub enum ResetAdvance { Paused, Complete(ResetComplete) }
pub fn advance<S: ResetServices>(work: &mut ResetWork, services: &mut S, mut budget: usize)
    -> Result<ResetAdvance, ResetError<S::Error>>
{
    if work.progress.failed { return Err(ResetError::PriorFailure); }
    if work.progress.stage == ResetStage::Complete { return Err(ResetError::AlreadyComplete); }
    loop {
        if budget == 0 { return Ok(ResetAdvance::Paused); }

        work.progress.failed = true;
        let result = match work.progress.stage {
            ResetStage::Module40 => services.reset_module40(),
            ResetStage::Module48 => services.reset_module48(),
            ResetStage::Module56 => services.reset_module56(),
            ResetStage::Module64 => services.reset_module64(),
            ResetStage::EmptyOwners => services.finish_empty_owners(),
            ResetStage::Complete => unreachable!(),
        };
        if let Err(error) = result {
            return Err(ResetError::Service(error));
        }
        work.progress.failed = false;
        budget -= 1;
        work.progress.stage = match work.progress.stage {
            ResetStage::Module40 => ResetStage::Module48,
            ResetStage::Module48 => ResetStage::Module56,
            ResetStage::Module56 => ResetStage::Module64,
            ResetStage::Module64 => ResetStage::EmptyOwners,
            ResetStage::EmptyOwners => ResetStage::Complete,
            ResetStage::Complete => unreachable!(),
        };
        if work.progress.stage == ResetStage::Complete {
            return Ok(ResetAdvance::Complete(ResetComplete { _private: () }));
        }
    }
}
