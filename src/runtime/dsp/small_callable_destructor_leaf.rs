#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum SourceSmallCallableDestructorEntry{At4ff0,At9fc0}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum SourceSmallCallableDestructorRequest{
    ReturnRequired{ret_pc:u32},
    TailFree55da4cRequired{tail_pc:u32,rcx:u64,edx:u32},
}

pub const fn source_small_callable_destructor_request(entry:SourceSmallCallableDestructorEntry,
    actual_rcx:u64,actual_edx:u32)->SourceSmallCallableDestructorRequest{
    let (ret_pc,tail_pc,size)=match entry{
        SourceSmallCallableDestructorEntry::At4ff0=>(0x4ffe,0x4ff9,16),
        SourceSmallCallableDestructorEntry::At9fc0=>(0x9fce,0x9fc9,24),
    };
    if actual_edx&0xff==0{SourceSmallCallableDestructorRequest::ReturnRequired{ret_pc}}
    else{SourceSmallCallableDestructorRequest::TailFree55da4cRequired{tail_pc,rcx:actual_rcx,edx:size}}
}
