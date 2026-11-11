#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum ReturnKind {Pooled,Unpooled}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum ReturnStage {Callback,Install,Consistency,Delete,Free,Complete}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct ReturnProgress {pub kind:ReturnKind,pub stage:ReturnStage,pub consistency_index:usize,pub failed:bool}
pub struct ReturnWork {progress:ReturnProgress}
impl ReturnWork {

    pub fn after_admission(kind:ReturnKind)->Self {
        let stage=match kind {ReturnKind::Pooled=>ReturnStage::Callback,ReturnKind::Unpooled=>ReturnStage::Delete};
        Self{progress:ReturnProgress{kind,stage,consistency_index:0,failed:false}}
    }
    pub fn progress(&self)->ReturnProgress {self.progress}
}
pub trait ReturnServices {
    type Error;

    fn pooled_callback_and_publish_current_raw(&mut self)->Result<(),Self::Error>;

    fn admit_current_and_install_saved_slot(&mut self)->Result<(),Self::Error>;
    fn bitmap_words(&self)->&[u64;16];
    fn cached_owner_present(&self,index:usize)->bool;

    fn delete_unpooled_vector(&mut self)->Result<(),Self::Error>;
    fn free_unpooled_pointer_cell(&mut self)->Result<(),Self::Error>;

    fn finish_return(&mut self);
}
pub enum ReturnError<E> {PriorFailure,AlreadyComplete,Service(E),Consistency{index:usize}}

pub struct ReturnComplete {kind:ReturnKind}
impl ReturnComplete {pub fn kind(&self)->ReturnKind {self.kind}}
pub enum ReturnAdvance {Paused,Complete(ReturnComplete)}

pub fn advance_return<S:ReturnServices>(work:&mut ReturnWork,services:&mut S,mut budget:usize)
    ->Result<ReturnAdvance,ReturnError<S::Error>> {
    if work.progress.failed {return Err(ReturnError::PriorFailure);}
    if work.progress.stage==ReturnStage::Complete {return Err(ReturnError::AlreadyComplete);}
    loop {
        if budget==0 {return Ok(ReturnAdvance::Paused);}
        let p=&mut work.progress;
        if p.stage==ReturnStage::Consistency {
            let index=p.consistency_index;
            let available=services.bitmap_words()[index/64]&(1_u64<<(index&63))!=0;
            if available!=services.cached_owner_present(index) {
                p.failed=true;return Err(ReturnError::Consistency{index});
            }
            p.consistency_index+=1;
            budget-=1;
            if p.consistency_index==1024 {
                services.finish_return();p.stage=ReturnStage::Complete;
                return Ok(ReturnAdvance::Complete(ReturnComplete{kind:p.kind}));
            }
            continue;
        }
        let result=match p.stage {
            ReturnStage::Callback=>services.pooled_callback_and_publish_current_raw(),
            ReturnStage::Install=>services.admit_current_and_install_saved_slot(),
            ReturnStage::Delete=>services.delete_unpooled_vector(),
            ReturnStage::Free=>services.free_unpooled_pointer_cell(),
            ReturnStage::Consistency|ReturnStage::Complete=>unreachable!(),
        };
        if let Err(error)=result {p.failed=true;return Err(ReturnError::Service(error));}
        budget-=1;
        p.stage=match p.stage {
            ReturnStage::Callback=>ReturnStage::Install,
            ReturnStage::Install=>ReturnStage::Consistency,
            ReturnStage::Delete=>ReturnStage::Free,
            ReturnStage::Free=>ReturnStage::Complete,
            ReturnStage::Consistency|ReturnStage::Complete=>unreachable!(),
        };
        if p.stage==ReturnStage::Complete {
            services.finish_return();
            return Ok(ReturnAdvance::Complete(ReturnComplete{kind:p.kind}));
        }
    }
}
