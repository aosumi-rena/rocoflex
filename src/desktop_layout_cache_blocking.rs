#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct CpuidRegisters {pub eax:u32,pub ebx:u32,pub ecx:u32,pub edx:u32}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct CpuidRecord {pub leaf:u32,pub subleaf:u32,pub registers:CpuidRegisters}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum CacheRoute { IntelLeaf4, AmdExtended, GenericLeaf4, LegacyLeaf2 }
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct CacheBytes {pub l1:i64,pub l2:i64,pub l3:i64}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct CacheSelection {pub bytes:CacheBytes,pub detected_signed_bytes:[i32;3],pub fallback_mask:u8,pub route:CacheRoute}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Product { Parent2d, Global4d }
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Blocking {pub kc:usize,pub mc:usize,pub nc:usize}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Error { MissingCpuid{leaf:u32,subleaf:u32},DuplicateCpuid,InvalidDepth,InvalidCacheFields,SourceDivisionByZero }

fn get(records:&[CpuidRecord],leaf:u32,subleaf:u32)->Result<CpuidRegisters,Error>{
    let mut rows=records.iter().filter(|r|r.leaf==leaf&&r.subleaf==subleaf);
    let row=rows.next().ok_or(Error::MissingCpuid{leaf,subleaf})?;
    if rows.next().is_some(){return Err(Error::DuplicateCpuid);}Ok(row.registers)
}

pub fn select_cache(records:&[CpuidRecord])->Result<CacheSelection,Error>{
    let vendor=get(records,0,0)?;
    let intel=vendor.ebx==0x756e6547&&vendor.edx==0x49656e69&&vendor.ecx==0x6c65746e;
    let amd=(vendor.ebx==0x68747541&&vendor.edx==0x69746e65&&vendor.ecx==0x444d4163)
        ||(vendor.ebx==0x69444d41&&vendor.edx==0x74656273&&vendor.ecx==0x21726574);
    let mut detected=[0_i32;3];
    let route;
    if amd {
        route=CacheRoute::AmdExtended;
        let a=get(records,0x80000005,0)?;let b=get(records,0x80000006,0)?;
        detected[0]=((a.ecx as i32>>14) as u32&0xfffffc00) as i32;
        detected[1]=((b.ecx as i32>>6) as u32&0xfffffc00) as i32;
        detected[2]=(b.edx.wrapping_mul(2)&0x1ff80000) as i32;
    }else if intel || vendor.ebx as i32>=4 {

        route=if intel{CacheRoute::IntelLeaf4}else{CacheRoute::GenericLeaf4};
        for index in 0..16 {
            let c=get(records,4,index)?;
            if c.eax&0xd==1 {
                let level=(c.eax>>5)&7;
                if (1..=3).contains(&level) {
                    let size=(c.ebx>>22).wrapping_add(1)
                        .wrapping_mul(((c.ebx>>12)&0x3ff).wrapping_add(1))
                        .wrapping_mul((c.ebx&0xfff).wrapping_add(1))
                        .wrapping_mul(c.ecx.wrapping_add(1));
                    detected[level as usize-1]=size as i32;
                }
            }
            if c.eax&0xf==0 {break;}
        }
    }else{
        route=CacheRoute::LegacyLeaf2;
        let c=get(records,2,0)?;let mut raw=[0_u8;16];
        for (i,w) in [c.eax,c.ebx,c.ecx,c.edx].into_iter().enumerate(){raw[i*4..i*4+4].copy_from_slice(&w.to_le_bytes());}
        let(mut l1,mut l2,mut l3,mut flag)=(0_u32,0_u32,0_u32,false);

        for &d in &raw[2..] {match d {
            0x0a|0x66=>l1=8,0x0c|0x10|0x15|0x60|0x67=>l1=16,
            0x0e=>l1=24,0x2c|0x30|0x68=>l1=32,
            0x1a=>l2=96,0x39|0x3b|0x41|0x79|0x81=>l2=128,0x3a=>l2=192,
            0x3c|0x42|0x7a|0x7e|0x82=>l2=256,0x3d=>l2=384,
            0x3e|0x43|0x7b|0x7f|0x80|0x83|0x86=>l2=512,
            0x44|0x78|0x7c|0x84|0x87=>l2=1024,0x45|0x7d|0x85=>l2=2048,
            0x48=>l2=3072,0x4e=>l2=6144,0x40=>l2=0,
            0x22=>l3=512,0x23=>l3=1024,0x25|0x88=>l3=2048,0x8d=>l3=3072,
            0x29|0x46|0x89=>l3=4096,0x4a=>l3=6144,
            0x47|0x4b|0x8a=>l3=8192,0x4c=>l3=12288,0x4d=>l3=16384,
            0x49=>{l3=4096;if l2==0{l2=l3;flag=true;}},_=>{}
        }}
        if flag && l2==l3 {l3=0;}
        detected=[l1.wrapping_shl(10) as i32,l2.wrapping_shl(10) as i32,l3.wrapping_shl(10) as i32];
    }

    let fallback=[32768_i64,262144,2097152];let mut values=[0;3];let mut mask=0;
    for i in 0..3 {if detected[i]>0{values[i]=i64::from(detected[i]);}else{values[i]=fallback[i];mask|=1<<i;}}
    Ok(CacheSelection{bytes:CacheBytes{l1:values[0],l2:values[1],l3:values[2]},detected_signed_bytes:detected,fallback_mask:mask,route})
}

fn balanced(original:i64,limit:i64,packet:i64,minus_one:bool)->Result<i64,Error>{
    if limit==0{return Err(Error::SourceDivisionByZero);}
    let q=original/limit;let r=original%limit;
    Ok(if r==0{limit}else{limit-((limit-r-i64::from(minus_one))/(packet*(q+1)))*packet})
}

pub fn blocking(product:Product,depth:usize,cache:CacheBytes)->Result<Blocking,Error>{
    if depth==0 || depth>i32::MAX as usize{return Err(Error::InvalidDepth);}
    if [cache.l1,cache.l2,cache.l3].iter().any(|&v|v<=0 || v>i32::MAX as i64){return Err(Error::InvalidCacheFields);}
    let original_k=depth as i64;let m=match product{Product::Parent2d=>2,Product::Global4d=>4};let n=m;
    let(mut k,mut mc,mut nc)=(original_k,m,n);
    if k.max(m).max(n)<48 {return Ok(Blocking{kc:k as usize,mc:mc as usize,nc:nc as usize});}
    let remaining_l1=cache.l1-128;
    let max_k=((remaining_l1/48)&!7_i64).max(1);
    if k>max_k {k=balanced(k,max_k,8,true)?;}
    let free_l1=remaining_l1-k*4*m;
    let from_l1=if free_l1<k*16 {0x480000/(max_k*16)}else{free_l1/(k*4)};
    let max_n=from_l1.min(0x180000/(k*8))&!3_i64;
    if n>max_n {
        nc=balanced(n,max_n,4,false)?;
    }else if original_k==k {
        let(mut max_m,mut effective_cache)=(m,cache.l1);
        if k*n*4>=0x401 {
            effective_cache=0x180000;
            if cache.l3!=0 && k*n*4<=0x8000 {
                max_m=m.min(0x240);effective_cache=cache.l2;
            }
        }
        let mut cap=max_m.min(effective_cache/(k*12));
        if cap>=9 {cap&=!7_i64;}
        if cap!=0 {mc=balanced(m,cap,8,false)?;}
    }
    Ok(Blocking{kc:k as usize,mc:mc as usize,nc:nc as usize})
}

pub fn blocking_from_cpuid(product:Product,depth:usize,records:&[CpuidRecord])->Result<(CacheSelection,Blocking),Error>{
    let selected=select_cache(records)?;let plan=blocking(product,depth,selected.bytes)?;Ok((selected,plan))
}

