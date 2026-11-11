fn icon_dimensions(width: i32, height: i32) -> (i32, i32) {
    let strip = width.min(height);
    (strip, width.wrapping_sub(strip))
}

fn icon_interval(width: i32, strip: i32, right_start: i32, mode: u8) -> [i32; 2] {
    if mode == 0 { [0, strip] } else { [right_start, width] }
}

fn x_before_icon(x: i32, start: i32) -> bool { x < start }

fn admits_after_x(x: i32, y: i32, end: i32, height: i32) -> bool {
    !(y < 0 || x >= end || y >= height)
}

pub fn icon_admission(width: i32, height: i32, mode: u8, point: [i32; 2]) -> bool {
    let (strip, right_start) = icon_dimensions(width, height);
    let [start, end] = icon_interval(width, strip, right_start, mode);
    if x_before_icon(point[0], start) {
        return false;
    }
    admits_after_x(point[0], point[1], end, height)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Width, Height, Mode, EventX, EventY, Collapsed, Callable, Table, Target, Ignored, Required }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    ComponentI32 { pc: u32, offset: u32 },
    ComponentByte { pc: u32, offset: u32 },
    EventI32 { pc: u32, offset: u32 },
    Callable { pc: u32, offset: u32 },
    Table { pc: u32 },
    Target { pc: u32, slot: u32 },
}

pub trait Services {
    type Error;
    type Callable;
    type Table;
    type Target;
    fn component_i32(&mut self, pc: u32, offset: u32) -> Result<i32, Self::Error>;
    fn component_byte(&mut self, pc: u32, offset: u32) -> Result<u8, Self::Error>;
    fn event_i32(&mut self, pc: u32, offset: u32) -> Result<i32, Self::Error>;

    fn callable(&mut self, pc: u32, offset: u32) -> Result<Option<Self::Callable>, Self::Error>;
    fn table(&mut self, pc: u32, callable: &Self::Callable) -> Result<Self::Table, Self::Error>;
    fn target(&mut self, pc: u32, table: &Self::Table, slot: u32) -> Result<Self::Target, Self::Error>;
}

#[derive(Debug, Default)]
pub struct Reads {
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub mode: Option<u8>,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub collapsed: Option<u8>,
    pub icon_width: Option<i32>,
    pub right_start: Option<i32>,
    pub icon_start: Option<i32>,
    pub icon_end: Option<i32>,
    pub selected_field: Option<u32>,
    pub callable_was_read: bool,
}

pub struct Prepared<'a, S: Services> {
    services: &'a mut S,
    phase: Phase,
    reads: Reads,
    attempted: Option<Access>,
    callable: Option<S::Callable>,
    table: Option<S::Table>,
    target: Option<S::Target>,
}

pub enum Fault<E> { Service(E), Incomplete(Phase) }
pub struct Stopped<'a, S: Services> { pending: Prepared<'a, S>, fault: Fault<S::Error> }
pub struct Ignored<'a, S: Services> { pending: Prepared<'a, S> }
pub struct Required<'a, S: Services> { pending: Prepared<'a, S> }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Boundary { TailCallback54b9a2, NullCallableCall58f9a8 }
pub enum Poll<'a, S: Services> {
    Pending(Prepared<'a, S>),
    Ignored(Ignored<'a, S>),
    Required(Required<'a, S>),
    Failed(Stopped<'a, S>),
}

impl<'a, S: Services> Prepared<'a, S> {

    pub fn from_explicit_services(services: &'a mut S) -> Self {
        Self { services, phase: Phase::Width, reads: Reads::default(), attempted: None,
            callable: None, table: None, target: None }
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn reads(&self) -> &Reads { &self.reads }
    pub fn attempted(&self) -> Option<Access> { self.attempted }
    pub fn services(&self) -> &S { self.services }
    pub fn callable(&self) -> Option<&S::Callable> { self.callable.as_ref() }
    pub fn table(&self) -> Option<&S::Table> { self.table.as_ref() }
    pub fn target(&self) -> Option<&S::Target> { self.target.as_ref() }

    pub fn poll(mut self, budget: usize) -> Poll<'a, S> {
        for _ in 0..budget {
            match self.phase {
                Phase::Ignored => return Poll::Ignored(Ignored { pending: self }),
                Phase::Required => return Poll::Required(Required { pending: self }),
                _ => {}
            }
            if let Err(fault) = self.step() {
                return Poll::Failed(Stopped { pending: self, fault });
            }
        }
        match self.phase {
            Phase::Ignored => Poll::Ignored(Ignored { pending: self }),
            Phase::Required => Poll::Required(Required { pending: self }),
            _ => Poll::Pending(self),
        }
    }

    fn need<T: Copy>(&self, value: Option<T>) -> Result<T, Fault<S::Error>> {
        value.ok_or(Fault::Incomplete(self.phase))
    }

    fn step(&mut self) -> Result<(), Fault<S::Error>> {
        match self.phase {
            Phase::Width => {
                self.attempted = Some(Access::ComponentI32 { pc: 0x54b944, offset: 0x40 });
                self.reads.width = Some(self.services.component_i32(0x54b944, 0x40).map_err(Fault::Service)?);
                self.phase = Phase::Height;
            }
            Phase::Height => {
                self.attempted = Some(Access::ComponentI32 { pc: 0x54b948, offset: 0x44 });
                let height = self.services.component_i32(0x54b948, 0x44).map_err(Fault::Service)?;
                self.reads.height = Some(height);
                let width = self.need(self.reads.width)?;
                let (strip, right_start) = icon_dimensions(width, height);
                self.reads.icon_width = Some(strip);
                self.reads.right_start = Some(right_start);
                self.reads.icon_start = Some(0);
                self.reads.icon_end = Some(strip);
                self.phase = Phase::Mode;
            }
            Phase::Mode => {
                self.attempted = Some(Access::ComponentByte { pc: 0x54b95e, offset: 0x1e0 });
                let mode = self.services.component_byte(0x54b95e, 0x1e0).map_err(Fault::Service)?;
                self.reads.mode = Some(mode);
                let [start, end] = icon_interval(self.need(self.reads.width)?,
                    self.need(self.reads.icon_width)?, self.need(self.reads.right_start)?, mode);
                self.reads.icon_start = Some(start);
                self.reads.icon_end = Some(end);
                self.phase = Phase::EventX;
            }
            Phase::EventX => {
                self.attempted = Some(Access::EventI32 { pc: 0x54b96d, offset: 8 });
                let x = self.services.event_i32(0x54b96d, 8).map_err(Fault::Service)?;
                self.reads.x = Some(x);
                self.phase = if x_before_icon(x, self.need(self.reads.icon_start)?) {
                    Phase::Ignored
                } else { Phase::EventY };
            }
            Phase::EventY => {
                self.attempted = Some(Access::EventI32 { pc: 0x54b976, offset: 0xc });
                let y = self.services.event_i32(0x54b976, 0xc).map_err(Fault::Service)?;
                self.reads.y = Some(y);
                self.phase = if admits_after_x(self.need(self.reads.x)?, y,
                    self.need(self.reads.icon_end)?, self.need(self.reads.height)?) {
                    Phase::Collapsed
                } else { Phase::Ignored };
            }
            Phase::Collapsed => {
                self.attempted = Some(Access::ComponentByte { pc: 0x54b986, offset: 0x1e1 });
                let collapsed = self.services.component_byte(0x54b986, 0x1e1).map_err(Fault::Service)?;
                self.reads.collapsed = Some(collapsed);
                self.reads.selected_field = Some(if collapsed == 0 { 0x190 } else { 0x150 });
                self.phase = Phase::Callable;
            }
            Phase::Callable => {
                let offset = self.need(self.reads.selected_field)?;
                let pc = if offset == 0x150 { 0x54b98f } else { 0x54b9ac };
                self.attempted = Some(Access::Callable { pc, offset });
                self.callable = self.services.callable(pc, offset).map_err(Fault::Service)?;
                self.reads.callable_was_read = true;
                self.phase = if self.callable.is_some() { Phase::Table } else { Phase::Required };
            }
            Phase::Table => {
                self.attempted = Some(Access::Table { pc: 0x54b99b });
                let callable = self.callable.as_ref().ok_or(Fault::Incomplete(self.phase))?;
                self.table = Some(self.services.table(0x54b99b, callable).map_err(Fault::Service)?);
                self.phase = Phase::Target;
            }
            Phase::Target => {
                self.attempted = Some(Access::Target { pc: 0x54b9a2, slot: 0x10 });
                let table = self.table.as_ref().ok_or(Fault::Incomplete(self.phase))?;
                self.target = Some(self.services.target(0x54b9a2, table, 0x10).map_err(Fault::Service)?);
                self.phase = Phase::Required;
            }
            Phase::Ignored | Phase::Required => {}
        }
        Ok(())
    }
}

impl<'a, S: Services> Required<'a, S> {
    pub fn pending(&self) -> &Prepared<'a, S> { &self.pending }
    pub fn boundary(&self) -> Boundary {
        if self.pending.callable.is_some() { Boundary::TailCallback54b9a2 }
        else { Boundary::NullCallableCall58f9a8 }
    }
}
impl<'a, S: Services> Stopped<'a, S> {
    pub fn pending(&self) -> &Prepared<'a, S> { &self.pending }
    pub fn fault(&self) -> &Fault<S::Error> { &self.fault }
}
impl<'a, S: Services> Ignored<'a, S> {
    pub fn pending(&self) -> &Prepared<'a, S> { &self.pending }

    pub fn into_services(self) -> &'a mut S { self.pending.services }
}
