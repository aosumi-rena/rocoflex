use super::super::image::SoftwareMapDescriptor;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Map { Source, Target }
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Address {pub map:Map,pub byte_offset:i64}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Budget { Unlimited, Accesses(usize) }
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Cause { Budget, Address, Bounds, ReadOnly, DescriptorField, DescriptorRange }
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Operation {
    DescriptorBase(Map),DescriptorDword {map:Map,offset:u32},
    Load {address:Address,width:u8},Store {address:Address,width:u8},
    DescriptorLoad {offset:usize,width:u8},DescriptorStore {offset:usize,width:u8},
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Attempt {pub source_rva:u32,pub operation:Operation}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Fault {pub attempt:Attempt,pub cause:Cause}
#[derive(Clone,Copy,Debug)]
pub struct Progress {pub attempted:Option<Attempt>,pub completed:usize,pub remaining:Budget,pub stopped:Option<Fault>}
struct Accesses {progress:Progress}
impl Accesses {
    fn new(remaining:Budget)->Self {
        Self {progress:Progress {attempted:None,completed:0,remaining,stopped:None}}
    }
    fn begin(&mut self,source_rva:u32,operation:Operation)->Result<Attempt,Fault> {

        if let Some(fault)=self.progress.stopped {return Err(fault);}
        let attempt=Attempt {source_rva,operation};self.progress.attempted=Some(attempt);
        match self.progress.remaining {
            Budget::Accesses(0)=>Err(self.stop(attempt,Cause::Budget)),
            Budget::Accesses(n)=>{self.progress.remaining=Budget::Accesses(n-1);Ok(attempt)},
            Budget::Unlimited=>Ok(attempt),
        }
    }
    fn stop(&mut self,attempt:Attempt,cause:Cause)->Fault {

        let fault=Fault {attempt,cause};
        self.progress.stopped=Some(fault);fault
    }
    fn complete<T>(&mut self,attempt:Attempt,result:Result<T,Cause>)->Result<T,Fault> {
        match result {
            Ok(value)=>{self.progress.completed=self.progress.completed.wrapping_add(1);Ok(value)},
            Err(cause)=>Err(self.stop(attempt,cause)),
        }
    }
}

#[derive(Clone,Copy,Debug)]
pub struct DescriptorValues {format:i32,row_stride:i32,pixel_stride:i32,width:i32,height:i32}
impl DescriptorValues {
    pub fn from_map(descriptor:&SoftwareMapDescriptor,dimensions:[i32;2])->Result<Self,Cause> {
        Ok(Self {format:descriptor.format,
            row_stride:i32::try_from(descriptor.row_stride).map_err(|_|Cause::DescriptorRange)?,
            pixel_stride:i32::try_from(descriptor.pixel_stride).map_err(|_|Cause::DescriptorRange)?,
            width:dimensions[0],height:dimensions[1]})
    }
    fn dword(self,offset:u32)->Result<u32,Cause> {
        let value=match offset {
            0x10=>self.format,0x14=>self.row_stride,0x18=>self.pixel_stride,
            0x1c=>self.width,0x20=>self.height,_=>return Err(Cause::DescriptorField),
        };
        Ok(value as u32)
    }
}

#[must_use="same borrowed target retains all earlier writes and stopped progress"]
pub struct PixelMaps<'a> {
    source:&'a [u8],target:&'a mut [u8],source_descriptor:DescriptorValues,target_descriptor:DescriptorValues,
    accesses:Accesses,
}
impl<'a> PixelMaps<'a> {
    pub fn bind(source:&'a [u8],target:&'a mut [u8],source_descriptor:DescriptorValues,
        target_descriptor:DescriptorValues,budget:Budget)->Self {
        Self {source,target,source_descriptor,target_descriptor,accesses:Accesses::new(budget)}
    }
    pub fn progress(&self)->Progress {self.accesses.progress}
    pub fn descriptor_base(&mut self,rva:u32,map:Map)->Result<Address,Fault> {
        let attempt=self.accesses.begin(rva,Operation::DescriptorBase(map))?;
        self.accesses.complete(attempt,Ok(Address {map,byte_offset:0}))
    }
    pub fn descriptor_dword(&mut self,rva:u32,map:Map,offset:u32)->Result<u32,Fault> {
        let attempt=self.accesses.begin(rva,Operation::DescriptorDword {map,offset})?;
        let descriptor=match map {Map::Source=>self.source_descriptor,Map::Target=>self.target_descriptor};
        self.accesses.complete(attempt,descriptor.dword(offset))
    }
    pub fn load_byte(&mut self,rva:u32,address:Address)->Result<u8,Fault> {
        let attempt=self.accesses.begin(rva,Operation::Load {address,width:1})?;
        let bytes=match address.map {Map::Source=>self.source,Map::Target=>&*self.target};
        let result=offset(address.byte_offset).and_then(|at|read_byte(bytes,at));
        self.accesses.complete(attempt,result)
    }
    pub fn load_dword(&mut self,rva:u32,address:Address)->Result<u32,Fault> {
        let attempt=self.accesses.begin(rva,Operation::Load {address,width:4})?;
        let bytes=match address.map {Map::Source=>self.source,Map::Target=>&*self.target};
        let result=offset(address.byte_offset).and_then(|at|read_dword(bytes,at));
        self.accesses.complete(attempt,result)
    }
    pub fn store_byte(&mut self,rva:u32,address:Address,value:u8)->Result<(),Fault> {
        let attempt=self.accesses.begin(rva,Operation::Store {address,width:1})?;
        let result=match address.map {
            Map::Source=>Err(Cause::ReadOnly),
            Map::Target=>offset(address.byte_offset).and_then(|at|write_byte(self.target,at,value)),
        };
        self.accesses.complete(attempt,result)
    }
    pub fn store_dword(&mut self,rva:u32,address:Address,value:u32)->Result<(),Fault> {
        let attempt=self.accesses.begin(rva,Operation::Store {address,width:4})?;
        let result=match address.map {
            Map::Source=>Err(Cause::ReadOnly),
            Map::Target=>offset(address.byte_offset).and_then(|at|write_dword(self.target,at,value)),
        };
        self.accesses.complete(attempt,result)
    }

    pub fn into_backings(self)->(&'a [u8],&'a mut [u8],Progress) {
        (self.source,self.target,self.accesses.progress)
    }
}

#[must_use="borrowed descriptor contains any preceding partial source effects"]
pub struct DescriptorBytes<'a> {bytes:&'a mut [u8],accesses:Accesses}
impl<'a> DescriptorBytes<'a> {
    pub fn bind(bytes:&'a mut [u8],budget:Budget)->Self {Self {bytes,accesses:Accesses::new(budget)}}
    pub fn progress(&self)->Progress {self.accesses.progress}
    pub fn load_dword(&mut self,rva:u32,offset:usize)->Result<u32,Fault> {
        let attempt=self.accesses.begin(rva,Operation::DescriptorLoad {offset,width:4})?;
        let result=read_dword(self.bytes,offset);self.accesses.complete(attempt,result)
    }
    pub fn store_dword(&mut self,rva:u32,offset:usize,value:u32)->Result<(),Fault> {
        let attempt=self.accesses.begin(rva,Operation::DescriptorStore {offset,width:4})?;
        let result=write_dword(self.bytes,offset,value);self.accesses.complete(attempt,result)
    }
    pub fn load_qword(&mut self,rva:u32,offset:usize)->Result<u64,Fault> {
        let attempt=self.accesses.begin(rva,Operation::DescriptorLoad {offset,width:8})?;
        let result=range(offset,8,self.bytes.len()).map(|r| {
            let b=&self.bytes[r];u64::from_le_bytes([b[0],b[1],b[2],b[3],b[4],b[5],b[6],b[7]])
        });
        self.accesses.complete(attempt,result)
    }
    pub fn store_qword(&mut self,rva:u32,offset:usize,value:u64)->Result<(),Fault> {
        let attempt=self.accesses.begin(rva,Operation::DescriptorStore {offset,width:8})?;
        let result=range(offset,8,self.bytes.len()).map(|r|self.bytes[r].copy_from_slice(&value.to_le_bytes()));
        self.accesses.complete(attempt,result)
    }
    pub fn into_backing(self)->(&'a mut [u8],Progress) {(self.bytes,self.accesses.progress)}
}

fn offset(value:i64)->Result<usize,Cause> {usize::try_from(value).map_err(|_|Cause::Address)}
fn range(at:usize,width:usize,extent:usize)->Result<std::ops::Range<usize>,Cause> {
    let end=at.checked_add(width).ok_or(Cause::Address)?;
    if end>extent {return Err(Cause::Bounds);}Ok(at..end)
}
fn read_byte(bytes:&[u8],at:usize)->Result<u8,Cause> {bytes.get(at).copied().ok_or(Cause::Bounds)}
fn write_byte(bytes:&mut [u8],at:usize,value:u8)->Result<(),Cause> {
    *bytes.get_mut(at).ok_or(Cause::Bounds)? = value;Ok(())
}

pub(super) fn read_dword(bytes:&[u8],at:usize)->Result<u32,Cause> {
    let b=&bytes[range(at,4,bytes.len())?];Ok(u32::from_le_bytes([b[0],b[1],b[2],b[3]]))
}
pub(super) fn write_dword(bytes:&mut [u8],at:usize,value:u32)->Result<(),Cause> {
    let r=range(at,4,bytes.len())?;bytes[r].copy_from_slice(&value.to_le_bytes());Ok(())
}
