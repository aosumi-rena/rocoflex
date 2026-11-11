pub trait FractionServices {
    type Error;
    fn signed_i32_to_f32_bits(&mut self,value:i32)->Result<u32,Self::Error>;
    fn addss_bits(&mut self,left:u32,right:u32)->Result<u32,Self::Error>;
    fn divss_bits(&mut self,left:u32,right:u32)->Result<u32,Self::Error>;
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum FractionStage {Convert,Add,Divide,Complete}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct FractionPartial {
    pub step:i32,pub bias:u32,pub denominator:u32,pub converted:Option<u32>,
    pub shifted:Option<u32>,pub fraction:Option<u32>,pub stage:FractionStage,
}
pub struct FractionFailure<E> {pub reason:E,pub partial:FractionPartial}
pub struct FractionComplete {pub bits:u32,pub partial:FractionPartial}

pub fn fraction<S:FractionServices>(step:i32,bias:u32,denominator:u32,services:&mut S)
    ->Result<FractionComplete,FractionFailure<S::Error>> {
    let mut p=FractionPartial{step,bias,denominator,converted:None,shifted:None,fraction:None,stage:FractionStage::Convert};
    let converted=services.signed_i32_to_f32_bits(step).map_err(|reason|FractionFailure{reason,partial:p})?;
    p.converted=Some(converted);p.stage=FractionStage::Add;
    let shifted=services.addss_bits(converted,bias).map_err(|reason|FractionFailure{reason,partial:p})?;
    p.shifted=Some(shifted);p.stage=FractionStage::Divide;
    let bits=services.divss_bits(shifted,denominator).map_err(|reason|FractionFailure{reason,partial:p})?;
    p.fraction=Some(bits);p.stage=FractionStage::Complete;Ok(FractionComplete{bits,partial:p})
}
pub trait LaneServices {
    type Error;
    fn read_next_bits(&mut self,lane:usize)->Result<u32,Self::Error>;
    fn read_previous_bits(&mut self,lane:usize)->Result<u32,Self::Error>;
    fn subss_bits(&mut self,left:u32,right:u32)->Result<u32,Self::Error>;
    fn mulss_bits(&mut self,left:u32,right:u32)->Result<u32,Self::Error>;
    fn addss_bits(&mut self,left:u32,right:u32)->Result<u32,Self::Error>;
    fn store_bits(&mut self,lane:usize,bits:u32)->Result<(),Self::Error>;
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum LaneStage {ReadNext,ReadPrevious,FiniteRead,Subtract,Multiply,Add,FiniteResult,Store,Complete}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct LanePartial {
    pub lane:usize,pub fraction:u32,pub next:Option<u32>,pub previous:Option<u32>,
    pub difference:Option<u32>,pub product:Option<u32>,pub value:Option<u32>,
    pub stage:LaneStage,pub stored:bool,
}
pub enum LaneError<E> {Service(E),NonfiniteRead,NonfiniteResult}
pub struct LaneFailure<E> {pub reason:LaneError<E>,pub partial:LanePartial}
pub struct LaneComplete {pub bits:u32,pub partial:LanePartial}
fn finite(bits:u32)->bool {bits&0x7f80_0000!=0x7f80_0000}

pub fn apply_lane<S:LaneServices>(lane:usize,fraction:u32,services:&mut S)
    ->Result<LaneComplete,LaneFailure<S::Error>> {
    let mut p=LanePartial{lane,fraction,next:None,previous:None,difference:None,product:None,value:None,
        stage:LaneStage::ReadNext,stored:false};
    let next=services.read_next_bits(lane).map_err(|reason|LaneFailure{reason:LaneError::Service(reason),partial:p})?;
    p.next=Some(next);p.stage=LaneStage::ReadPrevious;
    let previous=services.read_previous_bits(lane).map_err(|reason|LaneFailure{reason:LaneError::Service(reason),partial:p})?;
    p.previous=Some(previous);p.stage=LaneStage::FiniteRead;
    if !finite(next)||!finite(previous){return Err(LaneFailure{reason:LaneError::NonfiniteRead,partial:p});}
    p.stage=LaneStage::Subtract;
    let difference=services.subss_bits(next,previous).map_err(|reason|LaneFailure{reason:LaneError::Service(reason),partial:p})?;
    p.difference=Some(difference);p.stage=LaneStage::Multiply;
    let product=services.mulss_bits(difference,fraction).map_err(|reason|LaneFailure{reason:LaneError::Service(reason),partial:p})?;
    p.product=Some(product);p.stage=LaneStage::Add;
    let value=services.addss_bits(product,previous).map_err(|reason|LaneFailure{reason:LaneError::Service(reason),partial:p})?;
    p.value=Some(value);p.stage=LaneStage::FiniteResult;
    if !finite(difference)||!finite(product)||!finite(value){return Err(LaneFailure{reason:LaneError::NonfiniteResult,partial:p});}
    p.stage=LaneStage::Store;
    services.store_bits(lane,value).map_err(|reason|LaneFailure{reason:LaneError::Service(reason),partial:p})?;
    p.stored=true;p.stage=LaneStage::Complete;Ok(LaneComplete{bits:value,partial:p})
}

pub trait RowServices:FractionServices {
    type Target<'a>:LaneServices<Error=Self::Error> where Self:'a;
    fn copy_current_to_history(&mut self,first:bool)->Result<(),Self::Error>;
    fn copy_current_to_output(&mut self,written:usize)->Result<(),Self::Error>;
    fn bind_interpolation_target(&mut self,written:usize,step:usize)->Result<Self::Target<'_>,Self::Error>;
    fn increment_seen16(&mut self)->Result<(),Self::Error>;
}
pub struct RowBinding<R> {pub first:bool,pub width:usize,pub services:R}
pub trait EndServices {
    type Error;
    fn copy_history_to_output(&mut self,written:usize)->Result<(),Self::Error>;
}
pub trait InterpolationServices:FractionServices {
    type Row<'a>:RowServices<Error=Self::Error> where Self:'a;
    type End<'a>:EndServices<Error=Self::Error> where Self:'a;
    fn clear_first_empty(&mut self,ended:u8)->Result<(),Self::Error>;
    fn create_history_from_first(&mut self)->Result<(),Self::Error>;
    fn prepare_output336(&mut self,needed:usize)->Result<(),Self::Error>;
    fn set_output336_status(&mut self,ended:u8)->Result<(),Self::Error>;
    fn check_nonempty_arithmetic(&mut self)->Result<(),Self::Error>;

    fn bind_row(&mut self,input:usize)->Result<RowBinding<Self::Row<'_>>,Self::Error>;

    fn bind_end_copies(&mut self)->Result<Self::End<'_>,Self::Error>;
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum RowStage {FirstHistory,FirstCopies,Fraction,Target,Lanes,HistoryCopy,Seen,Complete}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct RowProgress {
    pub input:usize,pub first:bool,pub width:usize,pub stage:RowStage,
    pub copies:usize,pub step:usize,pub lane:usize,
    pub fraction:Option<FractionPartial>,pub partial_lane:Option<LanePartial>,
}
pub enum RowError<E> {Service(E),Fraction(FractionFailure<E>),Lane(LaneFailure<E>)}
fn process_row<S:RowServices>(p:&mut RowProgress,services:&mut S,factor:usize,half:usize,
    denominator:u32,written:&mut usize)->Result<(),RowError<S::Error>> {
    if p.first {
        p.stage=RowStage::FirstHistory;
        services.copy_current_to_history(true).map_err(RowError::Service)?;
        p.stage=RowStage::FirstCopies;
        for _ in 0..half {
            services.copy_current_to_output(*written).map_err(RowError::Service)?;
            *written+=1;p.copies+=1;
        }
    }else{
        let bias=if factor&1!=0{0}else{0x3f00_0000};
        for step in 0..factor {
            p.step=step;p.lane=0;p.partial_lane=None;p.stage=RowStage::Fraction;
            let value=match fraction(step as i32,bias,denominator,services){
                Ok(value)=>value,Err(error)=>{p.fraction=Some(error.partial);return Err(RowError::Fraction(error));}
            };
            p.fraction=Some(value.partial);p.stage=RowStage::Target;
            {
                let mut target=services.bind_interpolation_target(*written,step).map_err(RowError::Service)?;
                p.stage=RowStage::Lanes;
                for lane in 0..p.width {
                    p.lane=lane;
                    match apply_lane(lane,value.bits,&mut target){
                        Ok(result)=>p.partial_lane=Some(result.partial),
                        Err(error)=>{p.partial_lane=Some(error.partial);return Err(RowError::Lane(error));}
                    }
                    p.lane=lane+1;
                }
            }
            *written+=1;p.step=step+1;
        }
        p.stage=RowStage::HistoryCopy;
        services.copy_current_to_history(false).map_err(RowError::Service)?;
    }
    p.stage=RowStage::Seen;
    services.increment_seen16().map_err(RowError::Service)?;
    p.stage=RowStage::Complete;Ok(())
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum InterpolationStage {Ready,FirstEmpty,HistoryCreate,Prepare336,OutputStatus,Environment,Denominator,Rows,EndCopies,WrittenCheck,Complete}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct InterpolationProgress {
    pub stage:InterpolationStage,pub count:usize,pub initial_seen:u64,pub factor:i32,pub ended:u8,
    pub needed:Option<usize>,pub denominator:Option<u32>,pub input:usize,pub written:usize,
    pub row:Option<RowProgress>,pub end_copies:usize,pub failed:bool,
}
pub struct InterpolationWork {progress:InterpolationProgress}
impl InterpolationWork {

    pub fn after_explicit_prefix(count:usize,initial_seen:u64,factor:i32,ended:u8)->Self {
        Self{progress:InterpolationProgress{stage:InterpolationStage::Ready,count,initial_seen,factor,ended,
            needed:None,denominator:None,input:0,written:0,row:None,end_copies:0,failed:false}}
    }
    pub fn progress(&self)->InterpolationProgress {self.progress}
}
pub enum InterpolationError<E> {
    PriorFailure,AlreadyComplete,CountDomain,WrittenCount{expected:usize,written:usize},Service(E),Row(RowError<E>),
}

pub struct InterpolationComplete {written:usize,first_empty:bool}
impl InterpolationComplete {
    pub fn written(&self)->usize {self.written}
    pub fn first_empty(&self)->bool {self.first_empty}
}
pub enum InterpolationAdvance {Paused,Complete(InterpolationComplete)}

pub fn advance<S:InterpolationServices>(work:&mut InterpolationWork,services:&mut S,mut budget:usize)
    ->Result<InterpolationAdvance,InterpolationError<S::Error>> {
    if work.progress.failed{return Err(InterpolationError::PriorFailure);}
    if work.progress.stage==InterpolationStage::Complete{return Err(InterpolationError::AlreadyComplete);}
    let result=(||{
        let p=&mut work.progress;
        if p.stage==InterpolationStage::Ready {
            if p.initial_seen==0&&p.count==0 {p.stage=InterpolationStage::FirstEmpty;}
            else {
                let factor=p.factor as usize;let ceil=factor-factor/2;
                let mut needed=p.count.checked_mul(factor).ok_or(InterpolationError::CountDomain)?;
                if p.initial_seen==0{needed=needed.checked_sub(ceil).ok_or(InterpolationError::CountDomain)?;}
                if p.ended!=0{needed=needed.checked_add(ceil).ok_or(InterpolationError::CountDomain)?;}
                if needed>i32::MAX as usize{return Err(InterpolationError::CountDomain);}
                p.needed=Some(needed);
                p.stage=if p.initial_seen==0{InterpolationStage::HistoryCreate}else{InterpolationStage::Prepare336};
            }
        }
        loop {
            if p.stage==InterpolationStage::WrittenCheck {
                let expected=p.needed.expect("planned interpolation count");
                if p.written!=expected{return Err(InterpolationError::WrittenCount{expected,written:p.written});}
                p.stage=InterpolationStage::Complete;
                return Ok(InterpolationAdvance::Complete(InterpolationComplete{written:p.written,first_empty:false}));
            }
            if p.stage==InterpolationStage::Rows&&p.input==p.count {
                p.stage=if p.ended!=0{InterpolationStage::EndCopies}else{InterpolationStage::WrittenCheck};continue;
            }
            if budget==0{return Ok(InterpolationAdvance::Paused);}
            match p.stage {
                InterpolationStage::FirstEmpty=>{
                    services.clear_first_empty(p.ended).map_err(InterpolationError::Service)?;
                    p.stage=InterpolationStage::Complete;
                    return Ok(InterpolationAdvance::Complete(InterpolationComplete{written:0,first_empty:true}));
                }
                InterpolationStage::HistoryCreate=>{
                    services.create_history_from_first().map_err(InterpolationError::Service)?;
                    p.stage=InterpolationStage::Prepare336;
                }
                InterpolationStage::Prepare336=>{
                    services.prepare_output336(p.needed.expect("planned interpolation count")).map_err(InterpolationError::Service)?;
                    p.stage=InterpolationStage::OutputStatus;
                }
                InterpolationStage::OutputStatus=>{
                    services.set_output336_status(p.ended).map_err(InterpolationError::Service)?;
                    p.stage=if p.count!=0{InterpolationStage::Environment}else{InterpolationStage::Rows};
                }
                InterpolationStage::Environment=>{
                    services.check_nonempty_arithmetic().map_err(InterpolationError::Service)?;p.stage=InterpolationStage::Denominator;
                }
                InterpolationStage::Denominator=>{
                    p.denominator=Some(services.signed_i32_to_f32_bits(p.factor).map_err(InterpolationError::Service)?);
                    p.stage=InterpolationStage::Rows;
                }
                InterpolationStage::Rows=>{
                    let mut bound=services.bind_row(p.input).map_err(InterpolationError::Service)?;
                    let mut row=RowProgress{input:p.input,first:bound.first,width:bound.width,stage:RowStage::FirstHistory,
                        copies:0,step:0,lane:0,fraction:None,partial_lane:None};
                    let result=process_row(&mut row,&mut bound.services,p.factor as usize,(p.factor as usize)/2,
                        p.denominator.expect("nonempty source conversion"),&mut p.written);
                    p.row=Some(row);result.map_err(InterpolationError::Row)?;p.input+=1;
                }
                InterpolationStage::EndCopies=>{
                    let mut end=services.bind_end_copies().map_err(InterpolationError::Service)?;
                    let factor=p.factor as usize;
                    for _ in 0..factor-factor/2 {
                        end.copy_history_to_output(p.written).map_err(InterpolationError::Service)?;
                        p.written+=1;p.end_copies+=1;
                    }
                    p.stage=InterpolationStage::WrittenCheck;
                }
                InterpolationStage::Ready|InterpolationStage::WrittenCheck|InterpolationStage::Complete=>unreachable!(),
            }
            budget-=1;
        }
    })();
    if result.is_err(){work.progress.failed=true;}
    result
}
