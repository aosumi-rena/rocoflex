#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Activation{Sigmoid,Tanh}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum CallbackSite{Matrix(usize),ResetSigmoid,UpdateSigmoid,CandidateTanh}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Stage{Ready,ResetBias,Callback(CallbackSite),UpdateBias,CandidateBias,
    CandidateResetProduct,CandidateInputBias,HiddenUpdateProduct,InverseUpdate,
    CandidateUpdateProduct,HiddenAdd,Complete,Refused(CallbackSite)}
pub struct Work{pub reset:Vec<f32>,pub update:Vec<f32>,pub candidate:Vec<f32>,pub stage:Stage}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Buffer{Bias(usize),ResetSum,UpdateSum,ResetWork,UpdateWork,CandidateWork}
pub enum RowError<E>{Width{buffer:Buffer,expected:usize,actual:usize},Control{stage:Stage},
    Matrix{index:usize,reason:E},Activation{site:CallbackSite,reason:E}}
fn matrix_step<E>(stage:&mut Stage,index:usize,input:&[f32],destination:&mut[f32],
    matrix:&mut impl FnMut(usize,&[f32],&mut[f32])->Result<(),E>)->Result<(),RowError<E>>{
    let site=CallbackSite::Matrix(index);*stage=Stage::Callback(site);
    match matrix(index,input,destination){Ok(())=>Ok(()),Err(reason)=>{*stage=Stage::Refused(site);Err(RowError::Matrix{index,reason})}}
}
fn activation_step<E>(stage:&mut Stage,site:CallbackSite,kind:Activation,row:&mut[f32],
    activation:&mut impl FnMut(Activation,&mut[f32])->Result<(),E>)->Result<(),RowError<E>>{
    *stage=Stage::Callback(site);
    match activation(kind,row){Ok(())=>Ok(()),Err(reason)=>{*stage=Stage::Refused(site);Err(RowError::Activation{site,reason})}}
}

pub fn forward_row<E>(input:&[f32],hidden:&mut[f32],biases:[&[f32];6],bias_sum_r:&[f32],bias_sum_z:&[f32],work:&mut Work,
    matrix:&mut impl FnMut(usize,&[f32],&mut[f32])->Result<(),E>,
    activation:&mut impl FnMut(Activation,&mut[f32])->Result<(),E>)->Result<(),RowError<E>>{
    if !matches!(work.stage,Stage::Ready|Stage::Complete){return Err(RowError::Control{stage:work.stage});}
    let width=hidden.len();
    for(index,bias)in biases.iter().enumerate(){if bias.len()!=width{return Err(RowError::Width{buffer:Buffer::Bias(index),expected:width,actual:bias.len()});}}
    for(buffer,actual)in[(Buffer::ResetSum,bias_sum_r.len()),(Buffer::UpdateSum,bias_sum_z.len()),
        (Buffer::ResetWork,work.reset.len()),(Buffer::UpdateWork,work.update.len()),(Buffer::CandidateWork,work.candidate.len())]{
        if actual!=width{return Err(RowError::Width{buffer,expected:width,actual});}
    }
    work.stage=Stage::ResetBias;work.reset.copy_from_slice(bias_sum_r);
    matrix_step(&mut work.stage,0,input,&mut work.reset,matrix)?;
    matrix_step(&mut work.stage,3,hidden,&mut work.reset,matrix)?;
    activation_step(&mut work.stage,CallbackSite::ResetSigmoid,Activation::Sigmoid,&mut work.reset,activation)?;
    work.stage=Stage::UpdateBias;work.update.copy_from_slice(bias_sum_z);
    matrix_step(&mut work.stage,1,input,&mut work.update,matrix)?;
    matrix_step(&mut work.stage,4,hidden,&mut work.update,matrix)?;
    activation_step(&mut work.stage,CallbackSite::UpdateSigmoid,Activation::Sigmoid,&mut work.update,activation)?;
    work.stage=Stage::CandidateBias;work.candidate.copy_from_slice(biases[5]);
    matrix_step(&mut work.stage,5,hidden,&mut work.candidate,matrix)?;
    work.stage=Stage::CandidateResetProduct;
    for(value,gate)in work.candidate.iter_mut().zip(&work.reset){*value *= *gate;}
    matrix_step(&mut work.stage,2,input,&mut work.candidate,matrix)?;
    work.stage=Stage::CandidateInputBias;
    for(value,bias)in work.candidate.iter_mut().zip(biases[2]){*value += *bias;}
    activation_step(&mut work.stage,CallbackSite::CandidateTanh,Activation::Tanh,&mut work.candidate,activation)?;
    work.stage=Stage::HiddenUpdateProduct;
    for(value,gate)in hidden.iter_mut().zip(&work.update){*value *= *gate;}
    work.stage=Stage::InverseUpdate;
    for gate in &mut work.update{*gate=1.0f32-*gate;}
    work.stage=Stage::CandidateUpdateProduct;
    for(value,gate)in work.candidate.iter_mut().zip(&work.update){*value *= *gate;}
    work.stage=Stage::HiddenAdd;
    for(value,part)in hidden.iter_mut().zip(&work.candidate){*value += *part;}
    work.stage=Stage::Complete;Ok(())
}
