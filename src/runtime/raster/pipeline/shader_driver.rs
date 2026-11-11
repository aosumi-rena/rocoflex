use super::{inverse,setup};
use setup::root_loop as rows;

pub trait Services:setup::finish::Services+rows::bulk::Services {}
impl<T:setup::finish::Services+rows::bulk::Services> Services for T {}

type F<S> = <S as super::Services>::Frame;
type E<S> = <S as super::Services>::Error;
type I<S> = <S as inverse::Services>::Identity;
type AR<S> = <S as setup::Services>::AllocationReturnPacket;
type AO<S> = <S as setup::Services>::AllocationOutputPacket;
type SR<S> = <S as rows::sample::Services>::SampleReturnPacket;
type SO<S> = <S as rows::sample::Services>::SampleOutputPacket;
type LoopRequired<S> = rows::Required<F<S>,I<S>,AR<S>,AO<S>>;
type SetupRequired<S> = setup::Required<F<S>,I<S>,AR<S>,AO<S>>;

#[must_use = "unimplemented exact branch retains original advanced Frame/resources"]
pub enum Required<S:Services> {
    Prefix(super::Required<F<S>>),Setup(SetupRequired<S>),Loop(LoopRequired<S>),

    BulkB(rows::bulk::Required<F<S>,I<S>,AR<S>,AO<S>>),
}
#[must_use]
pub enum Outcome<S:Services> {
    Required(Required<S>),

    Returned(setup::finish::Completed<setup::finish::StateOf<S>>),
}
#[must_use = "each original typed failure owns SAME Frame and ALL partial receipts"]
pub enum Failure<S:Services> {
    Prefix(super::Failure<F<S>,E<S>>),
    Inverse(inverse::Failure<F<S>,I<S>,<S as inverse::Services>::ReturnPacket,<S as inverse::Services>::OutputPacket,E<S>>),
    Setup(setup::Failure<F<S>,I<S>,AR<S>,AO<S>,E<S>>),
    Loop(rows::Failure<F<S>,I<S>,AR<S>,AO<S>,E<S>>),
    Sample(rows::sample::Failure<F<S>,I<S>,AR<S>,AO<S>,SR<S>,SO<S>,E<S>>),
    Bulk(rows::bulk::CallerFailure<F<S>,I<S>,AR<S>,AO<S>,SR<S>,SO<S>,E<S>>),
    Finish(setup::finish::Failure<setup::finish::StateOf<S>,E<S>>),
}

pub fn from_entry<S:Services>(host:&mut S,frame:F<S>)->Result<Outcome<S>,Failure<S>> {
    let required=super::dispatch(host,frame).map_err(Failure::Prefix)?;
    from_prefix(host,required)
}
pub fn resume<S:Services>(host:&mut S,required:Required<S>)->Result<Outcome<S>,Failure<S>> {
    match required {
        Required::Prefix(p)=>from_prefix(host,p),Required::Setup(p)=>from_setup(host,p),Required::Loop(p)=>from_loop(host,p),
        Required::BulkB(p)=>match rows::bulk::resume_b(host,p).map_err(Failure::Bulk)? {

            rows::bulk::CallOutcome::Returned(p)=>from_loop(host,p),

            rows::bulk::CallOutcome::Required(p)=>Ok(Outcome::Required(Required::BulkB(p))),
        },
    }
}
fn from_prefix<S:Services>(host:&mut S,required:super::Required<F<S>>)->Result<Outcome<S>,Failure<S>> {
    match required.next {
        super::Next::Actual211800CallAt27a60d|super::Next::Actual211800CallAt27a6aa=>{
            let after=inverse::call(host,required).map_err(Failure::Inverse)?;
            let setup=setup::continue_after_inverse(host,after).map_err(Failure::Setup)?;
            from_setup(host,setup)
        },

        _=>Ok(Outcome::Required(Required::Prefix(required))),
    }
}
fn from_setup<S:Services>(host:&mut S,required:SetupRequired<S>)->Result<Outcome<S>,Failure<S>> {
    match required.next_rva {
        0x27fbe7=>setup::finish::from_signed_height_branch(host,required)
            .map(Outcome::Returned).map_err(Failure::Finish),
        0x27a8e5|0x27ae5a=>{
            let loop_entry=rows::prefix(host,required).map_err(Failure::Loop)?;
            from_loop(host,loop_entry)
        },
        _=>Ok(Outcome::Required(Required::Setup(required))),
    }
}

pub fn from_loop<S:Services>(host:&mut S,mut required:LoopRequired<S>)->Result<Outcome<S>,Failure<S>> {

    loop {
        required=match required.next_rva {
            0x27a8e5|0x27ae5a=>rows::resume_prefix(host,required).map_err(Failure::Loop)?,
            0x27a8d5|0x27ae4a=>rows::point::advance_skipped_row(host,required).map_err(Failure::Loop)?,
            0x27a8cc|0x27ae41=>rows::point::reload_after_final_store(host,required).map_err(Failure::Loop)?,
            0x27a95d|0x27aecd|0x27aac7|0x27b01c|0x27aacc|0x27b021|0x27aae7|0x27b03c|
            0x27aaf4|0x27b049|0x27ac10|0x27b150|0x27a7c6|0x27ad53=>
                rows::point::advance(host,required).map_err(Failure::Loop)?,
            0x27a9d6|0x27ab0b|0x27ac5d|0x27a7dc|0x27af46|0x27b060|0x27b19d|0x27ad69=>
                rows::sample::call(host,required).map_err(Failure::Sample)?,
            0x27a9ff|0x27ac86|0x27af6f|0x27b1c6=>rows::blend::full_pixel(host,required).map_err(Failure::Loop)?,
            0x27ab39|0x27a808|0x27b08e|0x27ad95=>rows::blend::partial::pixel(host,required).map_err(Failure::Loop)?,
            0x27aae2|0x27b037=>match rows::bulk::call(host,required).map_err(Failure::Bulk)? {
                rows::bulk::CallOutcome::Returned(p)=>p,
                rows::bulk::CallOutcome::Required(p)=>return Ok(Outcome::Required(Required::BulkB(p))),
            },
            0x27fbdf=>return setup::finish::from_row_exit(host,required).map(Outcome::Returned).map_err(Failure::Finish),
            _=>return Ok(Outcome::Required(Required::Loop(required))),
        };
    }
}
