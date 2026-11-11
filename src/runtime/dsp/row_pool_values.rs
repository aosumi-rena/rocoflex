#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum RowGrowthError {ExistingExtent,InsertionIndex,SourceLengthLimit,AllocationExtent,NullOwnership,AliasedOwnership}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum RowAllocationRoute {Small55d9a0,Aligned32Via55d9a0}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct SourceRowGrowthPlan {old_count:usize,old_capacity:usize,index:usize,new_capacity:usize}
impl SourceRowGrowthPlan {
    pub fn bind(old_count:usize,old_capacity:usize,index:usize)->Result<Self,RowGrowthError> {
        if old_count>old_capacity {return Err(RowGrowthError::ExistingExtent);}
        if index>old_count {return Err(RowGrowthError::InsertionIndex);}
        let limit=0x1fff_ffff_ffff_ffffu64;
        if old_capacity as u64>limit||old_count as u64>=limit {return Err(RowGrowthError::SourceLengthLimit);}
        let grown=(old_capacity as u64).saturating_add((old_capacity/2) as u64).min(limit);
        let new_capacity=grown.max(old_count as u64+1);

        if new_capacity>=0x1fff_ffff_ffff_fffc {return Err(RowGrowthError::SourceLengthLimit);}
        Ok(Self{old_count,old_capacity,index,new_capacity:new_capacity as usize})
    }
    pub fn new_count(self)->usize {self.old_count+1}
    pub fn new_capacity(self)->usize {self.new_capacity}
    pub fn inserted_index(self)->usize {self.index}
    pub fn allocation_route(self)->RowAllocationRoute {
        if self.new_capacity>=512 {RowAllocationRoute::Aligned32Via55d9a0}else{RowAllocationRoute::Small55d9a0}
    }
    pub fn allocator_requested_bytes(self)->usize {self.new_capacity*8+if self.new_capacity>=512 {39}else{0}}
    pub fn old_deallocation_requires_header(self)->bool {self.old_capacity>=512}

    pub fn route_explicit_allocation(self,old:&[u64],donor:u64,mut allocated:Vec<u64>)->Result<Box<[u64]>,RowGrowthError> {
        if old.len()!=self.old_capacity||allocated.len()!=self.new_capacity {return Err(RowGrowthError::AllocationExtent);}
        allocated[self.index]=donor;
        allocated[..self.index].copy_from_slice(&old[..self.index]);
        allocated[self.index+1..self.new_count()].copy_from_slice(&old[self.index..self.old_count]);
        Ok(allocated.into_boxed_slice())
    }
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum PoolPutPlan{Retire,AppendExisting,Grow(SourceRowGrowthPlan)}
pub fn pool_cached_take_index(count:usize)->Option<usize>{count.checked_sub(1)}
pub fn pool_put_plan(count:usize,capacity:usize,maximum:u64)->Result<PoolPutPlan,RowGrowthError>{
    if maximum!=0&&count as u64>=maximum{return Ok(PoolPutPlan::Retire);}
    if count<capacity{return Ok(PoolPutPlan::AppendExisting);}
    SourceRowGrowthPlan::bind(count,capacity,count).map(PoolPutPlan::Grow)
}
