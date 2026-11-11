#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct CachedChoice {word:usize,bit:usize,index:usize,mask:u64}
impl CachedChoice {
    pub fn word(self)->usize {self.word}
    pub fn bit(self)->usize {self.bit}
    pub fn index(self)->usize {self.index}
    pub fn mask(self)->u64 {self.mask}
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum CheckoutStage {Select,Cached,Allocate,Create,Admit,Complete}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct CheckoutProgress {pub stage:CheckoutStage,pub choice:Option<CachedChoice>,pub failed:bool}
pub struct CheckoutWork {progress:CheckoutProgress}
impl CheckoutWork {

    pub fn after_admission()->Self {
        Self{progress:CheckoutProgress{stage:CheckoutStage::Select,choice:None,failed:false}}
    }
    pub fn progress(&self)->CheckoutProgress {self.progress}
}
pub trait CheckoutServices {
    type Error;
    fn bitmap_words(&self)->&[u64;16];

    fn take_cached_and_finish(&mut self,choice:CachedChoice)->Result<(),Self::Error>;

    fn allocate_pointer_cell8(&mut self)->Result<(),Self::Error>;

    fn create_with_required_factory(&mut self)->Result<(),Self::Error>;

    fn admit_created_and_finish(&mut self)->Result<(),Self::Error>;
}
pub enum CheckoutError<E> {PriorFailure,AlreadyComplete,Service(E)}

pub struct CheckoutComplete {choice:Option<CachedChoice>}
impl CheckoutComplete {pub fn choice(&self)->Option<CachedChoice> {self.choice}}
pub enum CheckoutAdvance {Paused,Complete(CheckoutComplete)}

pub fn advance_checkout<S:CheckoutServices>(work:&mut CheckoutWork,services:&mut S,mut budget:usize)
    ->Result<CheckoutAdvance,CheckoutError<S::Error>> {
    if work.progress.failed {return Err(CheckoutError::PriorFailure);}
    if work.progress.stage==CheckoutStage::Complete {return Err(CheckoutError::AlreadyComplete);}
    loop {
        if budget==0 {return Ok(CheckoutAdvance::Paused);}
        let p=&mut work.progress;
        if p.stage==CheckoutStage::Select {
            let words=services.bitmap_words();
            if let Some(word)=words.iter().position(|bits|*bits!=0) {
                let bit=words[word].trailing_zeros() as usize;
                p.choice=Some(CachedChoice{word,bit,index:(word<<6)|bit,mask:1_u64<<bit});
                p.stage=CheckoutStage::Cached;
            } else {p.stage=CheckoutStage::Allocate;}
        }
        let result=match p.stage {
            CheckoutStage::Cached=>services.take_cached_and_finish(p.choice.expect("selected cached bit")),
            CheckoutStage::Allocate=>services.allocate_pointer_cell8(),
            CheckoutStage::Create=>services.create_with_required_factory(),
            CheckoutStage::Admit=>services.admit_created_and_finish(),
            CheckoutStage::Select|CheckoutStage::Complete=>unreachable!(),
        };
        if let Err(error)=result {p.failed=true;return Err(CheckoutError::Service(error));}
        budget-=1;
        p.stage=match p.stage {
            CheckoutStage::Cached|CheckoutStage::Admit=>CheckoutStage::Complete,
            CheckoutStage::Allocate=>CheckoutStage::Create,
            CheckoutStage::Create=>CheckoutStage::Admit,
            CheckoutStage::Select|CheckoutStage::Complete=>unreachable!(),
        };
        if p.stage==CheckoutStage::Complete {
            return Ok(CheckoutAdvance::Complete(CheckoutComplete{choice:p.choice}));
        }
    }
}
