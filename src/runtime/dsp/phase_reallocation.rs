use std::collections::TryReserveError;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum PhaseAllocationRoute {Small55d9a0,Aligned4e10}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum PhaseAllocationError {ExistingExtent,NotReallocation,SourceLengthLimit,AllocationExtent}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct PhaseReallocationPlan {old_count:usize,old_capacity:usize,new_count:usize,new_capacity:usize,route:PhaseAllocationRoute}
impl PhaseReallocationPlan {
    pub fn bind(old_count:usize,old_capacity:usize,new_count:usize)->Result<Self,PhaseAllocationError> {
        if old_count>old_capacity {return Err(PhaseAllocationError::ExistingExtent);}
        if new_count<=old_capacity {return Err(PhaseAllocationError::NotReallocation);}
        let limit=0x3fff_ffff_ffff_ffffu64;
        if old_capacity as u64>limit||new_count as u64>limit {return Err(PhaseAllocationError::SourceLengthLimit);}

        let grown=(old_capacity as u64).saturating_add((old_capacity/2) as u64).min(limit);
        let new_capacity=grown.max(new_count as u64) as usize;
        let route=if new_capacity>=1024 {PhaseAllocationRoute::Aligned4e10}else{PhaseAllocationRoute::Small55d9a0};
        Ok(Self{old_count,old_capacity,new_count,new_capacity,route})
    }
    pub fn new_count(&self)->usize {self.new_count}
    pub fn new_capacity(&self)->usize {self.new_capacity}
    pub fn allocation_route(&self)->PhaseAllocationRoute {self.route}

    pub fn allocate_with_explicit_spare_word(&self,spare_word:u32)->Result<Vec<u32>,TryReserveError> {
        let mut allocation=Vec::new();
        allocation.try_reserve_exact(self.new_capacity)?;
        allocation.resize(self.new_capacity,spare_word);Ok(allocation)
    }

    pub fn apply_to_explicit_allocation(&self,old_backing:&[u32],new_backing:Vec<u32>)
        ->Result<Box<[u32]>,PhaseAllocationError> {
        self.apply_retaining(old_backing,new_backing).map_err(|(cause,_)|cause)
    }

    pub fn apply_retaining(&self,old_backing:&[u32],mut new_backing:Vec<u32>)
        ->Result<Box<[u32]>,(PhaseAllocationError,Vec<u32>)> {
        if old_backing.len()!=self.old_capacity||new_backing.len()!=self.new_capacity {
            return Err((PhaseAllocationError::AllocationExtent,new_backing));
        }

        new_backing[self.old_count..self.new_count].fill(0);
        new_backing[..self.old_count].copy_from_slice(&old_backing[..self.old_count]);
        Ok(new_backing.into_boxed_slice())
    }
}
