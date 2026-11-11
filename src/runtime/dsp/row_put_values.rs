use crate::row_pool_values::{PoolPutPlan,SourceRowGrowthPlan};

pub trait PutServices {
    type Output;
    type Error;
    fn retire_donor_if_nonnull(&mut self)->Result<Self::Output,Self::Error>;
    fn append_existing(&mut self)->Result<Self::Output,Self::Error>;
    fn grow_and_append(&mut self,plan:SourceRowGrowthPlan)->Result<Self::Output,Self::Error>;
}

pub fn apply_put_plan<S:PutServices>(plan:PoolPutPlan,services:&mut S)->Result<S::Output,S::Error>{
    match plan {
        PoolPutPlan::Retire=>services.retire_donor_if_nonnull(),
        PoolPutPlan::AppendExisting=>services.append_existing(),
        PoolPutPlan::Grow(plan)=>services.grow_and_append(plan),
    }
}
