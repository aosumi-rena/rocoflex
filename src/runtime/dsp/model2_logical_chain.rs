use std::collections::TryReserveError;
use crate::conditional_gate::{Batch,GateError,Projection};
use crate::moda7_projection::{Moda7Projection,RequiredServices as ActivationServices};
use crate::modl0_projection::{Modl0Projection,RequiredServices as LinearServices};
use crate::modm0_logical_sequence::{self as sequence,CachePolicy,LogicalSequence,RequiredReset};

pub enum LaterChild<A:ActivationServices,L:LinearServices>{
    Moda7(Moda7Projection<A>),Modl0(Modl0Projection<L>),
}
impl<A:ActivationServices,L:LinearServices> Projection for LaterChild<A,L>{
    fn reset(&mut self)->Result<(),GateError>{
        match self{Self::Moda7(child)=>child.reset(),Self::Modl0(child)=>child.reset()}
    }
    fn forward(&mut self,input:&Batch)->Result<Batch,GateError>{
        match self{Self::Moda7(child)=>child.forward(input),Self::Modl0(child)=>child.forward(input)}
    }
}

pub struct Parts<F:LinearServices,A:ActivationServices,L:LinearServices,R>{
    pub first:Modl0Projection<F>,pub middle:Moda7Projection<A>,pub last:Modl0Projection<L>,

    pub intermediates:Vec<Batch>,pub reset:R,
    pub cache_policy:CachePolicy,pub needs_reset:bool,
}
pub enum Refusal<F:LinearServices,A:ActivationServices,L:LinearServices,R>{

    Reservation{parts:Parts<F,A,L,R>,reason:TryReserveError},

    Sequence(sequence::Refusal<Modl0Projection<F>,LaterChild<A,L>,R>),
}

pub fn bind_initialized<F:LinearServices,A:ActivationServices,L:LinearServices,R>(parts:Parts<F,A,L,R>)
    ->Result<LogicalSequence<Modl0Projection<F>,LaterChild<A,L>,R>,Refusal<F,A,L,R>>
where R:RequiredReset<Modl0Projection<F>,LaterChild<A,L>>{
    let mut following=Vec::new();
    if let Err(reason)=following.try_reserve_exact(2){return Err(Refusal::Reservation{parts,reason});}
    let Parts{first,middle,last,intermediates,reset,cache_policy,needs_reset}=parts;
    following.push(LaterChild::Moda7(middle));
    following.push(LaterChild::Modl0(last));
    LogicalSequence::from_explicit_parts(sequence::Parts{first,following,intermediates,reset,cache_policy,needs_reset})
        .map_err(Refusal::Sequence)
}
