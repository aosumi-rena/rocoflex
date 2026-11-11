use super::{path_allocation,sample,collapse_path_rows_source,
    EdgeTable,Error,PathRowsCause,PathRowsFailure,PathRowsProgress,Row};
use super::super::path::{self,Line,LineIterator,point_source,iterator_source};
type Never=core::convert::Infallible;
#[path="edge_table_scan.rs"] pub mod scan;
pub use scan::ScanServices;
use scan::{Endpoint,Site,F32Operation,F64Operation,Predicate,IntegerSource,Integer32Operation,Integer64Operation,
    F64Comparison,ComparisonFlags,CountSqrtCrtCall,CountSqrtBranch};
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Stage {Inputs,Allocation,Initialize,Construct,NextLine,Quantize,Clip,LineMath,Step,Sample,Insert,Advance,Collapse,ReleaseIterator,Complete}
#[derive(Clone,Copy,Debug)]
pub struct Progress {
    pub stage:Stage,pub initialized_rows:usize,pub line_ordinal:usize,pub inserted_samples:usize,
    pub fixed_y:Option<[i64;2]>,pub relative_y:Option<[i64;2]>,pub current_y:Option<i64>,pub end_y:Option<i64>,
    pub delta:Option<[u32;2]>,pub slope:Option<u64>,pub source_step:Option<i32>,pub step:Option<i64>,

    pub source_step_division:Option<i32>,
    pub midpoint:Option<i64>,pub sample:Option<i64>,pub row:Option<usize>,pub entry:Option<[i32;2]>,
    pub quantized_y:[Option<i64>;2],
    pub count_guard:Option<ComparisonFlags>,pub count_sqrt_branch:Option<CountSqrtBranch>,

    pub numeric_request:Option<scan::Request>,pub numeric_result:Option<scan::Value>,pub completed_numeric:u64,

    pub numeric_values:[Option<scan::Value>;Site::VALUE_COUNT],
    pub construction:Option<point_source::IdentityProgress>,
    pub iterator:Option<iterator_source::Progress>,pub iterator_inflight:bool,
    pub insertion:Option<sample::Progress>,pub collapse:Option<PathRowsProgress>,
    pub iterator_released:bool,
}
pub enum Cause<E=core::convert::Infallible> {
    Table(Error),Numeric(scan::Failure<E>),Construction(point_source::ConstructionFailure<E>),
    Iterator(iterator_source::Failure<E>),Sample(sample::Failure),Collapse(PathRowsFailure),
}
impl Cause<Never> {
    pub fn into_error(self)->Error {match self {
        Self::Table(error)=>error,
        Self::Numeric(failure)=>match failure.error {},
        Self::Construction(failure)=>Error::Path(point_source::legacy_construction_error(failure)),
        Self::Iterator(failure)=>Error::Path(iterator_source::legacy_error(failure)),
        Self::Sample(failure)=>failure.error,
        Self::Collapse(failure)=>match failure.cause {PathRowsCause::Table(error)=>error,PathRowsCause::Sort(_)=>Error::Size},
    }}
}
pub struct Completed<'p> {
    pub path:&'p path::Path,pub bounds:[i32;4],pub affine:[f32;6],pub table:EdgeTable,pub progress:Progress,
}
#[must_use="retain SAME supplied Path, advanced table/iterator/line and stopped sample; no replay"]
pub struct Failure<'p,E=core::convert::Infallible> {

    pub pending_line:Option<Line>,pub walker:Option<LineIterator<'p>>,
    pub table:Option<EdgeTable>,pub initial_rows:Vec<Option<Row>>,
    pub path:&'p path::Path,pub bounds:[i32;4],pub affine:[f32;6],
    pub progress:Progress,pub cause:Cause<E>,
}

#[must_use="retain actual input/numeric owners with this terminal failed graph"]
pub struct TerminalFailure<E> {
    pending_line:Option<Line>,walker:Option<path::TerminalIterator>,
    table:Option<EdgeTable>,initial_rows:Vec<Option<Row>>,
    bounds:[i32;4],affine:[f32;6],progress:Progress,cause:Cause<E>,
}
impl<E> TerminalFailure<E> {
    pub fn pending_line(&self)->Option<&Line> {self.pending_line.as_ref()}
    pub fn walker(&self)->Option<&path::TerminalIterator> {self.walker.as_ref()}
    pub fn table(&self)->Option<&EdgeTable> {self.table.as_ref()}
    pub fn initial_rows(&self)->&[Option<Row>] {&self.initial_rows}
    pub fn bounds(&self)->&[i32;4] {&self.bounds}
    pub fn affine(&self)->&[f32;6] {&self.affine}
    pub fn progress(&self)->&Progress {&self.progress}
    pub fn cause(&self)->&Cause<E> {&self.cause}
}
impl<E> Failure<'_,E> {
    pub fn freeze_terminal(self)->TerminalFailure<E> {
        let Self {pending_line,walker,table,initial_rows,path:_,bounds,affine,progress,cause}=self;
        TerminalFailure {pending_line,walker:walker.map(LineIterator::freeze_terminal),
            table,initial_rows,bounds,affine,progress,cause}
    }
}
impl Failure<'_,Never> {

    pub fn into_error(self)->Error {
        let Self {path:_,bounds:_,affine:_,table,initial_rows,walker,pending_line,progress:_,cause}=self;
        let error=cause.into_error();drop(pending_line);drop(walker);drop(table);drop(initial_rows);error
    }
}
pub fn run_existing_host_source(path:&path::Path,bounds:[i32;4],affine:[f32;6])
    ->Result<Completed<'_>,Failure<'_>>
{
    run_with_services(path,bounds,affine.map(f32::to_bits),&mut path::ellipse_source::ExistingHostValueFp)
}

pub fn run_with_services<'p,S,E>(path:&'p path::Path,bounds:[i32;4],affine_words:[u32;6],services:&mut S)
    ->Result<Completed<'p>,Failure<'p,E>>
where S:ScanServices<Error=E>+path::comparison::PackedServices<Error=E>
    +point_source::Fp<Error=E>+path::curve_source::Fp<Error=E>
{
    let affine=affine_words.map(f32::from_bits);
    let mut progress=Progress {stage:Stage::Inputs,initialized_rows:0,line_ordinal:0,inserted_samples:0,
        fixed_y:None,relative_y:None,current_y:None,end_y:None,delta:None,slope:None,source_step:None,
        step:None,source_step_division:None,midpoint:None,sample:None,row:None,entry:None,quantized_y:[None;2],
        count_guard:None,count_sqrt_branch:None,
        numeric_request:None,numeric_result:None,completed_numeric:0,numeric_values:[None;Site::VALUE_COUNT],
        construction:None,iterator:None,iterator_inflight:false,insertion:None,collapse:None,iterator_released:false};
    let mut initial_rows=Vec::new();let mut table=None;let mut walker=None;let mut pending_line=None;
    let result=(||->Result<(),Cause<E>> {
        macro_rules! op {($site:expr,$kind:ident,$operation:expr,$method:ident,$value:ident)=>{{
            let site=$site;let operation=$operation;
            perform(&mut progress,scan::Request {site,operation:scan::Operation::$kind(operation)},
                ||services.$method(site,operation),scan::Value::$value)?
        }};}
        if bounds[2]<=0||bounds[3]<0||path.words.len()>i32::MAX as usize {return Err(Cause::Table(Error::Domain));}
        progress.stage=Stage::Allocation;
        let count=op!(Site::CountWide,IntegerToF64,IntegerSource::Signed32(path.words.len() as i32),integer_wide,F64);
        let flags=op!(Site::CountSqrtGuard,F64Comparison,F64Comparison {left:count,right:0},f64_comparison,Flags);
        progress.count_guard=Some(flags);

        let branch=if flags.carry {CountSqrtBranch::Crt}else {CountSqrtBranch::Fast};
        progress.count_sqrt_branch=Some(branch);
        let root=match branch {
            CountSqrtBranch::Fast=>op!(Site::CountSqrt,F64,F64Operation::Sqrt {source:count},f64,F64),
            CountSqrtBranch::Crt=>op!(Site::CountSqrtCrt,CountSqrtCrt,CountSqrtCrtCall {argument:count},count_sqrt_crt,F64),
        };
        let integer=op!(Site::CountInteger,Integer32,Integer32Operation::CountResult {source:root},integer32,Integer32);
        let capacity=integer.wrapping_shl(2).max(16);
        let (stride,allocated_words)=path_allocation(bounds[3],capacity).map_err(Cause::Table)?;
        let allocated_rows=bounds[3] as usize+2;
        initial_rows.try_reserve_exact(allocated_rows).map_err(|_|Cause::Table(Error::Allocation))?;
        initial_rows.resize(allocated_rows,None);
        progress.stage=Stage::Initialize;
        for row in initial_rows.iter_mut().take(bounds[3] as usize) {
            *row=Some(Row::Edges {entries:Vec::new()});progress.initialized_rows+=1;
        }
        table=Some(EdgeTable {bounds,source_capacity_edges:capacity,source_stride_words:stride,
            source_flag_32:1,allocated_words,rows:core::mem::take(&mut initial_rows)});
        progress.stage=Stage::Construct;
        match LineIterator::new_source_with_comparisons_retaining(path,affine_words,services) {
            Ok(completed)=>{progress.construction=Some(completed.progress);walker=Some(completed.walker);},
            Err(failure)=>{progress.construction=Some(failure.progress);return Err(Cause::Construction(failure));},
        }
        let table=table.as_mut().ok_or(Cause::Table(Error::NativeOutOfBounds))?;
        let walker=walker.as_mut().ok_or(Cause::Table(Error::NativeOutOfBounds))?;
        let minimum_x=i64::from(bounds[0])<<8;
        let maximum_x=((i64::from(bounds[0])+i64::from(bounds[2]))<<8)-1;
        let origin_y=i64::from(bounds[1])<<8;let height_fixed=i64::from(bounds[3])<<8;
        loop {
            progress.stage=Stage::NextLine;
            progress.iterator_inflight=true;
            match walker.next_source_line_with_fp(services) {
                Ok(completed)=>{
                    progress.iterator=Some(completed.progress);
                    progress.iterator_inflight=false;pending_line=completed.line;
                },
                Err(failure)=>{progress.iterator=Some(failure.progress);return Err(Cause::Iterator(failure));},
            }
            let Some(line)=pending_line.as_ref() else {break;};
            progress.line_ordinal+=1;
            progress.fixed_y=None;progress.relative_y=None;progress.current_y=None;progress.end_y=None;
            progress.delta=None;progress.slope=None;progress.source_step=None;progress.step=None;
            progress.source_step_division=None;
            progress.midpoint=None;progress.sample=None;progress.row=None;progress.entry=None;
            progress.quantized_y=[None;2];Site::clear_line_values(&mut progress.numeric_values);progress.insertion=None;
            progress.stage=Stage::Quantize;
            for (endpoint,y) in [(Endpoint::From,line.from[1].to_bits()),(Endpoint::To,line.to[1].to_bits())] {

                let positive=op!(Site::YHalf(endpoint),Predicate,Predicate::OrderedLessEqual {left:0,right:y},predicate,Predicate);
                let half=if positive {0x3f000000}else {0xbf000000};
                let scaled=op!(Site::YScale(endpoint),F32,F32Operation::Arithmetic(path::ellipse_source::Operation::Mul {destination:y,source:0x43800000}),f32,F32);
                let biased=op!(Site::YBias(endpoint),F32,F32Operation::Arithmetic(path::ellipse_source::Operation::Add {destination:scaled,source:half}),f32,F32);
                let integer=op!(Site::YInteger(endpoint),Integer64,Integer64Operation::FromF32 {source:biased},integer64,Integer64);
                progress.quantized_y[endpoint.index()]=Some(integer);
            }
            let y0=progress.quantized_y[0].ok_or(Cause::Table(Error::NativeOutOfBounds))?;
            let y1=progress.quantized_y[1].ok_or(Cause::Table(Error::NativeOutOfBounds))?;
            progress.fixed_y=Some([y0,y1]);
            if y0==y1 {continue;}
            progress.stage=Stage::Clip;
            let relative0=y0.wrapping_sub(origin_y);let relative1=y1.wrapping_sub(origin_y);
            progress.relative_y=Some([relative0,relative1]);
            let mut current_y=relative0.min(relative1).max(0);
            let end_y=relative0.max(relative1).min(height_fixed);progress.current_y=Some(current_y);progress.end_y=Some(end_y);
            if current_y>=end_y {continue;}
            progress.stage=Stage::LineMath;
            let delta_x=op!(Site::DeltaX,F32,F32Operation::Arithmetic(path::ellipse_source::Operation::Sub {destination:line.to[0].to_bits(),source:line.from[0].to_bits()}),f32,F32);
            let start_x=op!(Site::StartXScale,F32,F32Operation::Arithmetic(path::ellipse_source::Operation::Mul {destination:line.from[0].to_bits(),source:0x43800000}),f32,F32);
            let delta_y=op!(Site::DeltaY,F32,F32Operation::Arithmetic(path::ellipse_source::Operation::Sub {destination:line.to[1].to_bits(),source:line.from[1].to_bits()}),f32,F32);
            progress.delta=Some([delta_x,delta_y]);
            let slope32=op!(Site::Slope,F32,F32Operation::Divide {destination:delta_x,source:delta_y},f32,F32);
            let slope=op!(Site::SlopeWide,F32ToF64,slope32,widen,F64);progress.slope=Some(slope);
            let absolute_slope=slope&0x7fffffffffffffff;
            let divisor=op!(Site::SlopeInteger,Integer32,Integer32Operation::Slope {source:absolute_slope},integer32,Integer32).wrapping_add(1);
            let source_step=256i32.checked_div(divisor).ok_or(Cause::Table(Error::Divide))?;
            progress.source_step_division=Some(source_step);

            let x0_fixed=op!(Site::StartXWide,F32ToF64,start_x,widen,F64);
            let source_step=if source_step<2 {1}else {source_step};progress.source_step=Some(source_step);
            while current_y<end_y {
                progress.stage=Stage::Step;
                progress.midpoint=None;progress.sample=None;progress.row=None;progress.entry=None;
                Site::clear_sample_values(&mut progress.numeric_values);progress.insertion=None;
                let until_row_end=256i64-(current_y&255);
                let step=end_y.wrapping_sub(current_y).min(i64::from(source_step)).min(until_row_end);progress.step=Some(step);
                if step<=0 {return Err(Cause::Table(Error::Domain));}
                progress.stage=Stage::Sample;
                let midpoint=current_y.wrapping_sub(relative0).wrapping_add(step>>1);progress.midpoint=Some(midpoint);
                let wide=op!(Site::MidpointWide,IntegerToF64,IntegerSource::Signed64(midpoint),integer_wide,F64);
                let product=op!(Site::SampleMul,F64,F64Operation::Mul {destination:wide,source:slope},f64,F64);
                let sum=op!(Site::SampleAdd,F64,F64Operation::Add {destination:product,source:x0_fixed},f64,F64);
                let sample=op!(Site::SampleInteger,Integer64,Integer64Operation::FromF64 {source:sum},integer64,Integer64);progress.sample=Some(sample);
                let upper_clamped=if maximum_x<sample {maximum_x}else {sample};
                let fixed_x=if minimum_x<=sample {upper_clamped as i32}else {minimum_x as i32};
                let level=if y0>y1 {step as i32}else {(step as i32).wrapping_neg()};
                let row=usize::try_from(current_y>>8).map_err(|_|Cause::Table(Error::NativeOutOfBounds))?;
                progress.row=Some(row);progress.entry=Some([fixed_x,level]);progress.stage=Stage::Insert;
                match sample::insert_source(table,row,[fixed_x,level]) {
                    Ok(completed)=>{progress.insertion=Some(completed);progress.inserted_samples+=1;},
                    Err(failure)=>{progress.insertion=Some(failure.progress);return Err(Cause::Sample(failure));},
                }
                progress.stage=Stage::Advance;current_y=current_y.wrapping_add(step);progress.current_y=Some(current_y);
            }
        }
        progress.stage=Stage::Collapse;
        match collapse_path_rows_source(table,path.source_flag_32) {
            Ok(completed)=>progress.collapse=Some(completed),
            Err(failure)=>{progress.collapse=Some(failure.progress);return Err(Cause::Collapse(failure));},
        }
        Ok(())
    })();
    match result {
        Ok(())=>match table {
            Some(table)=>{
                progress.stage=Stage::ReleaseIterator;drop(walker.take());progress.iterator_released=true;
                progress.stage=Stage::Complete;Ok(Completed {path,bounds,affine,table,progress})
            },
            None=>Err(Failure {path,bounds,affine,table:None,initial_rows,walker,pending_line,progress,cause:Cause::Table(Error::NativeOutOfBounds)}),
        },
        Err(cause)=>Err(Failure {path,bounds,affine,table,initial_rows,walker,pending_line,progress,cause}),
    }
}

fn perform<T:Copy,E>(progress:&mut Progress,request:scan::Request,
    operation:impl FnOnce()->Result<T,E>,as_value:fn(T)->scan::Value)->Result<T,Cause<E>>
{
    progress.numeric_request=Some(request);progress.numeric_result=None;
    let value=operation().map_err(|error|Cause::Numeric(scan::Failure {request,error}))?;
    let raw=as_value(value);progress.numeric_result=Some(raw);progress.numeric_values[request.site.index()]=Some(raw);
    progress.completed_numeric=progress.completed_numeric.wrapping_add(1);Ok(value)
}
