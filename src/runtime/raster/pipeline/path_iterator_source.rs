use super::{comparison,curve_source,point_source,Error,Line,LineIterator};

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Step {ReadToken,CompareToken,ReadPoint(point_source::Role),Point(point_source::Role),Quadratic,Cubic,Near(u8),PeekClose,Complete}
#[derive(Clone,Copy,Debug)]
pub struct Progress {
    pub step:Step,pub cursor_before:usize,pub cursor_after:usize,pub stack_before:usize,pub stack_after:usize,
    pub current_before:[u32;2],pub current_after:[u32;2],pub subpath_after:[u32;2],pub segment_index_after:i32,
    pub from:[u32;2],pub token:Option<u32>,pub token_word_index:usize,pub stacked:bool,
    pub comparison:Option<comparison::Request>,pub completed_comparisons:u32,pub consumed_tokens:u64,
    pub points:[Option<[u32;2]>;3],pub completed_curve:Option<curve_source::Progress>,pub completed_near:Option<curve_source::NearProgress>,
    pub raw_points:[Option<[u32;2]>;3],pub reads:[Option<point_source::Progress>;3],
    pub transforms:[Option<point_source::Progress>;3],
}
#[derive(Debug)]
pub enum Cause<E> {
    Storage(Error),Comparison {request:comparison::Request,error:E},
    InconsistentToken {token:u32,phase:comparison::TokenPhase},
    Point(point_source::Failure<E>),Curve(curve_source::Failure<E>),Near(curve_source::NearFailure<E>),
}
#[derive(Debug)]
#[must_use="SAME iterator and caller FP owner retain all source partial effects"]
pub struct Failure<E> {pub progress:Progress,pub cause:Cause<E>}
pub struct Completed {pub line:Option<Line>,pub progress:Progress}

fn token_equal<P:comparison::Services>(fp:&mut P,progress:&mut Progress,phase:comparison::TokenPhase,token:u32,marker:u32)
    ->Result<bool,Cause<P::Error>>
{
    progress.step=Step::CompareToken;
    let request=comparison::Request {site:comparison::Site::IteratorToken {phase,stacked:progress.stacked,
        word_index:progress.token_word_index,marker},relation:comparison::Relation::OrderedEqual,left:token,right:marker};
    progress.comparison=Some(request);
    let result=fp.compare(request).map_err(|error|Cause::Comparison {request,error})?;
    progress.completed_comparisons=progress.completed_comparisons.wrapping_add(1);Ok(result)
}
fn near<P:curve_source::Fp>(fp:&mut P,progress:&mut Progress,pair:u8,first:u32,second:u32)
    ->Result<bool,Cause<P::Error>>
{
    progress.step=Step::Near(pair);
    let completed=curve_source::near_with_fp(comparison::CurveKind::Line,pair,first,second,fp).map_err(Cause::Near)?;
    progress.completed_near=Some(completed.progress);Ok(completed.result)
}
fn read_point<E>(walker:&mut LineIterator<'_>,stacked:bool,role:point_source::Role,
    progress:&mut Progress,slot:usize)->Result<point_source::ReadCompleted,Cause<E>>
{
    match walker.read_point_source(stacked,role) {
        Ok(completed)=>{
            progress.raw_points[slot]=Some(completed.point());progress.reads[slot]=Some(*completed.progress());Ok(completed)
        },
        Err(failure)=>{
            progress.reads[slot]=Some(failure.progress);
            Err(Cause::Point(point_source::Failure {progress:failure.progress,cause:point_source::Cause::Storage(failure.error)}))
        },
    }
}
fn transform_point<P:point_source::Fp>(walker:&mut LineIterator<'_>,read:point_source::ReadCompleted,
    role:point_source::Role,fp:&mut P,progress:&mut Progress,slot:usize)->Result<[f32;2],Cause<P::Error>>
{
    match walker.finish_point_with_fp_retaining(read,role,fp) {
        Ok(completed)=>{
            let point=completed.point();progress.transforms[slot]=Some(*completed.progress());
            progress.points[slot]=Some(point);Ok(point.map(f32::from_bits))
        },
        Err(failure)=>{progress.transforms[slot]=Some(failure.progress);Err(Cause::Point(failure))},
    }
}
impl<'a> LineIterator<'a> {
    pub fn next_source_line_with_fp<P>(&mut self,fp:&mut P)
        ->Result<Completed,Failure<<P as curve_source::Fp>::Error>>
    where P:curve_source::Fp+point_source::Fp<Error=<P as curve_source::Fp>::Error>
        +comparison::Services<Error=<P as curve_source::Fp>::Error>
    {
        let mut progress=Progress {step:Step::ReadToken,cursor_before:self.cursor,cursor_after:self.cursor,
            stack_before:self.stack.len(),stack_after:self.stack.len(),current_before:self.current.map(f32::to_bits),
            current_after:self.current.map(f32::to_bits),subpath_after:self.subpath_start.map(f32::to_bits),segment_index_after:self.segment_index,
            from:self.current.map(f32::to_bits),token:None,token_word_index:self.cursor,stacked:false,comparison:None,
            completed_comparisons:0,consumed_tokens:0,points:[None;3],completed_curve:None,completed_near:None,
            raw_points:[None;3],reads:[None;3],transforms:[None;3]};
        let result=(||->Result<Option<Line>,Cause<<P as curve_source::Fp>::Error>> {
            let mut from=self.current;
            loop {
                progress.step=Step::ReadToken;progress.from=from.map(f32::to_bits);progress.points=[None;3];
                progress.raw_points=[None;3];progress.reads=[None;3];progress.transforms=[None;3];
                let stacked=!self.stack.is_empty();progress.stacked=stacked;
                let bits=if stacked {
                    progress.token_word_index=self.stack.len()-1;
                    self.stack.pop().ok_or(Cause::Storage(Error::MalformedStream))?
                }else {
                    progress.token_word_index=self.cursor;
                    let Some(&word)=self.path.words.get(self.cursor) else {progress.step=Step::Complete;return Ok(None);};
                    self.cursor+=1;word
                };
                progress.token=Some(bits);progress.consumed_tokens=progress.consumed_tokens.wrapping_add(1);
                use comparison::TokenPhase as T;
                if token_equal(fp,&mut progress,T::InitialClose,bits,0x47c35280)? {
                    if near(fp,&mut progress,0,self.current[0].to_bits(),self.subpath_start[0].to_bits())?
                        &&near(fp,&mut progress,1,self.current[1].to_bits(),self.subpath_start[1].to_bits())? {continue;}
                    from=self.current;progress.from=from.map(f32::to_bits);self.current=self.subpath_start;progress.step=Step::Complete;
                    return Ok(Some(Line {from,to:self.current,source_closed_flag:true}));
                }

                progress.step=Step::ReadPoint(point_source::Role::Ordinary);
                let first_read=read_point::<<P as curve_source::Fp>::Error>(self,stacked,point_source::Role::Ordinary,&mut progress,0)?;
                if token_equal(fp,&mut progress,T::QuadraticAfterRead,bits,0x47c35180)? {
                    progress.step=Step::ReadPoint(point_source::Role::QuadraticEnd);
                    let end_read=read_point::<<P as curve_source::Fp>::Error>(self,stacked,point_source::Role::QuadraticEnd,&mut progress,1)?;
                    progress.step=Step::Point(point_source::Role::QuadraticControl);
                    let control=transform_point(self,first_read,point_source::Role::QuadraticControl,fp,&mut progress,0)?;
                    progress.step=Step::Point(point_source::Role::QuadraticEnd);
                    let end=transform_point(self,end_read,point_source::Role::QuadraticEnd,fp,&mut progress,1)?;
                    progress.step=Step::Quadratic;
                    progress.completed_curve=Some(self.quadratic_with_fp(from.map(f32::to_bits),control.map(f32::to_bits),end.map(f32::to_bits),fp)
                        .map_err(Cause::Curve)?);continue;
                }
                if token_equal(fp,&mut progress,T::CubicAfterRead,bits,0x47c35200)? {
                    progress.step=Step::ReadPoint(point_source::Role::CubicSecond);
                    let second_read=read_point::<<P as curve_source::Fp>::Error>(self,stacked,point_source::Role::CubicSecond,&mut progress,1)?;
                    progress.step=Step::ReadPoint(point_source::Role::CubicEnd);
                    let end_read=read_point::<<P as curve_source::Fp>::Error>(self,stacked,point_source::Role::CubicEnd,&mut progress,2)?;
                    progress.step=Step::Point(point_source::Role::CubicFirst);
                    let first=transform_point(self,first_read,point_source::Role::CubicFirst,fp,&mut progress,0)?;
                    progress.step=Step::Point(point_source::Role::CubicSecond);
                    let second=transform_point(self,second_read,point_source::Role::CubicSecond,fp,&mut progress,1)?;
                    progress.step=Step::Point(point_source::Role::CubicEnd);
                    let end=transform_point(self,end_read,point_source::Role::CubicEnd,fp,&mut progress,2)?;
                    progress.step=Step::Cubic;
                    progress.completed_curve=Some(self.cubic_with_fp(from.map(f32::to_bits),first.map(f32::to_bits),second.map(f32::to_bits),
                        end.map(f32::to_bits),fp).map_err(Cause::Curve)?);continue;
                }
                progress.step=Step::Point(point_source::Role::Ordinary);
                self.current=transform_point(self,first_read,point_source::Role::Ordinary,fp,&mut progress,0)?;
                if !token_equal(fp,&mut progress,T::LineAfterTransform,bits,0x47c35080)? {

                    if token_equal(fp,&mut progress,T::QuadraticAfterTransform,bits,0x47c35180)? {
                        return Err(Cause::InconsistentToken {token:bits,phase:T::QuadraticAfterTransform});
                    }
                    if token_equal(fp,&mut progress,T::CubicAfterTransform,bits,0x47c35200)? {
                        return Err(Cause::InconsistentToken {token:bits,phase:T::CubicAfterTransform});
                    }
                    if token_equal(fp,&mut progress,T::CloseBeforeMove,bits,0x47c35280)? {
                        return Err(Cause::InconsistentToken {token:bits,phase:T::CloseBeforeMove});
                    }

                    self.segment_index=-1;from=self.current;self.subpath_start=self.current;continue;
                }
                self.segment_index=self.segment_index.wrapping_add(1);
                let mut next_is_close=false;
                if self.stack.is_empty() {
                    if let Some(&word)=self.path.words.get(self.cursor) {
                        progress.step=Step::PeekClose;
                        let request=comparison::Request {site:comparison::Site::IteratorPeekClose {word_index:self.cursor},
                            relation:comparison::Relation::OrderedEqual,left:word,right:0x47c35280};
                        progress.comparison=Some(request);
                        next_is_close=comparison::Services::compare(fp,request).map_err(|error|Cause::Comparison {request,error})?;
                        progress.completed_comparisons=progress.completed_comparisons.wrapping_add(1);
                    }
                }
                let source_closed_flag=next_is_close
                    &&near(fp,&mut progress,2,self.current[0].to_bits(),self.subpath_start[0].to_bits())?
                    &&near(fp,&mut progress,3,self.current[1].to_bits(),self.subpath_start[1].to_bits())?;
                progress.step=Step::Complete;return Ok(Some(Line {from,to:self.current,source_closed_flag}));
            }
        })();
        progress.cursor_after=self.cursor;progress.stack_after=self.stack.len();progress.current_after=self.current.map(f32::to_bits);
        progress.subpath_after=self.subpath_start.map(f32::to_bits);progress.segment_index_after=self.segment_index;
        match result {Ok(line)=>Ok(Completed {line,progress}),Err(cause)=>Err(Failure {progress,cause})}
    }
}
pub(crate) fn legacy_error(failure:Failure<core::convert::Infallible>)->Error {
    match failure.cause {
        Cause::Storage(error)=>error,Cause::Comparison {error,..}=>match error {},
        Cause::InconsistentToken {..}=>Error::MalformedStream,
        Cause::Point(failure)=>match failure.cause {point_source::Cause::Storage(error)=>error,
            point_source::Cause::Arithmetic(error)=>match error {}},
        Cause::Curve(failure)=>curve_source::legacy_error(failure),
        Cause::Near(failure)=>match failure.cause {
            curve_source::NumericCause::Arithmetic {error,..}|curve_source::NumericCause::Comparison {error,..}
                |curve_source::NumericCause::Maximum {error,..}=>match error {},
        },
    }
}
