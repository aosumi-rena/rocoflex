use super::canvas_primitives::F32Operation;

const ONE: u32 = 0x3f80_0000;
const NEGATIVE_FOUR: u32 = 0xc080_0000;
const OPACITY_SCALE: u64 = 0x406f_e000_0000_0000;
const ROUND_BIAS: u64 = 0x4338_0000_0000_0000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Input {
    pub width: i32,
    pub height: i32,
    pub current: i32,
    pub total: i32,

    pub collapsed: u8,

    pub mode: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CounterInput {
    pub collapsed: u8,
    pub current: i32,
    pub total: i32,
    pub timer_period: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction { Open, Close }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchedulerRequired {
    Call28ee90,
    Tail28f190,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransitionValues {
    pub before: CounterInput,
    pub after: CounterInput,
    pub scheduler: SchedulerRequired,
}

pub fn transition_values(input: CounterInput, direction: Direction)
    -> Option<TransitionValues>
{
    let collapsed = match direction {
        Direction::Open if input.collapsed != 0 => 0,
        Direction::Close if input.collapsed == 0 => 1,
        _ => return None,
    };
    Some(TransitionValues {
        before: input,
        after: CounterInput {
            collapsed,
            current: input.total.wrapping_sub(input.current),
            total: input.total,
            timer_period: 16,
        },
        scheduler: if input.timer_period == 0 {
            SchedulerRequired::Call28ee90
        } else {
            SchedulerRequired::Tail28f190
        },
    })
}

pub fn timer_counter_value(current: i32, total: i32) -> Option<i32> {
    if current >= total { None } else { Some(current.wrapping_add(1)) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    F32(F32Operation),
    Cvtss2sd { source: u32 },
    Mulsd { destination: u64, source: u64 },
    Addsd { destination: u64, source: u64 },
    Helper5b0580 { argument: u32 },
    Cvttss2siSigned32 { source: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    pub source_rva: u32,
    pub operation: Operation,
}

pub trait Services {
    type Error;
    fn f32(&mut self, source_rva: u32, operation: F32Operation)
        -> Result<u32, Self::Error>;
    fn cvtss2sd(&mut self, source_rva: u32, source: u32)
        -> Result<u64, Self::Error>;
    fn mulsd(&mut self, source_rva: u32, destination: u64, source: u64)
        -> Result<u64, Self::Error>;
    fn addsd(&mut self, source_rva: u32, destination: u64, source: u64)
        -> Result<u64, Self::Error>;
    fn helper5b0580(&mut self, source_rva: u32, argument: u32)
        -> Result<u32, Self::Error>;
    fn cvttss2si_signed32(&mut self, source_rva: u32, source: u32)
        -> Result<i32, Self::Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Word {
    F32(u32),
    F64(u64),
    Signed32(i32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompletedOperation {
    pub request: Request,
    pub result: Word,
}

#[derive(Debug)]
pub struct Progress {
    pub input: Input,
    pub completed: Vec<CompletedOperation>,

    pub envelope: i32,
    pub opacity_byte: Option<u8>,
    pub child_x: Option<i32>,
}

#[derive(Debug)]
pub struct Failure<E> {
    pub cause: E,
    pub attempted: Request,
    pub progress: Progress,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChildRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParentLayout {
    pub pitch: ChildRect,
    pub waypoints: ChildRect,
    pub voice_group: ChildRect,
}

pub fn parent_layout(width: i32, height: i32) -> ParentLayout {

    let limited_width = width.min(32);
    let half_gap = (32i32.wrapping_sub(limited_width) as u32) >> 1;
    let right_inset = (half_gap as i32).wrapping_add(limited_width);
    let voice_x = width.wrapping_sub(right_inset).wrapping_sub(20);

    let vertical_difference = height.wrapping_sub(80);
    let sign = ((vertical_difference as u32) >> 31) as i32;
    let voice_y = vertical_difference.wrapping_add(sign) >> 1;
    let monitor_y = height.wrapping_sub(56);
    ParentLayout {
        pitch: ChildRect { x: 12, y: monitor_y, width: 258, height: 44 },
        waypoints: ChildRect {
            x: width.wrapping_sub(132), y: monitor_y, width: 120, height: 44,
        },
        voice_group: ChildRect { x: voice_x, y: voice_y, width: 32, height: 80 },
    }
}

pub fn monitor_hit_test(
    width: i32,
    height: i32,
    mode: u8,
    point: [i32; 2],
    child: ChildRect,
) -> bool {
    let strip = width.min(height);
    let (start, end) = if mode == 0 { (0, strip) }
        else { (width.wrapping_sub(strip), width) };
    if point[1] >= 0 && point[0] >= start && point[0] < end && point[1] < height {
        return true;
    }
    point[0] >= child.x
        && point[1] >= child.y
        && point[0] < child.x.wrapping_add(child.width)
        && point[1] < child.y.wrapping_add(child.height)
}

pub fn mapped_icon_rect(mapped: ChildRect, current_height: i32, mode: u8) -> ChildRect {
    if mode == 0 {
        ChildRect { width: current_height.max(0), ..mapped }
    } else {
        let width = mapped.width.min(current_height);
        ChildRect {
            x: mapped.x.wrapping_add(mapped.width.wrapping_sub(width)),
            width,
            ..mapped
        }
    }
}

pub fn waypoint_child_rects(width: i32, height: i32) -> [ChildRect; 2] {
    fn half_toward_zero(value: i32) -> i32 {
        value.wrapping_add(((value as u32) >> 31) as i32) >> 1
    }
    let x = half_toward_zero(width.wrapping_sub(76));
    let y = half_toward_zero(height.wrapping_sub(24));
    [
        ChildRect { x, y, width: 24, height: 24 },
        ChildRect { x: x.wrapping_add(36), y, width: 24, height: 24 },
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaypointIcon { Add, Reroute }

pub fn waypoint_icon_color(
    icon: WaypointIcon,
    highlighted: bool,
    current_count: i32,
    current_ordering_counter: i32,
) -> u32 {
    if icon == WaypointIcon::Add
        && (current_count >= 99 || current_ordering_counter >= 0)
    {
        0x80b7_b8bb
    } else if highlighted {
        0xffe0_e0e2
    } else {
        0xffb7_b8bb
    }
}

#[derive(Debug)]
pub struct Completed {
    pub opacity_byte: u8,
    pub child: ChildRect,
    pub progress: Progress,
}

struct Graph<'a, S: Services> {
    services: &'a mut S,
    progress: Progress,
}

impl<S: Services> Graph<'_, S> {
    fn accept<T>(
        &mut self,
        request: Request,
        result: Result<T, S::Error>,
        word: impl FnOnce(&T) -> Word,
    ) -> Result<T, (Request, S::Error)> {
        match result {
            Ok(value) => {
                self.progress.completed.push(CompletedOperation {
                    request,
                    result: word(&value),
                });
                Ok(value)
            }
            Err(error) => Err((request, error)),
        }
    }

    fn f32(&mut self, pc: u32, operation: F32Operation)
        -> Result<u32, (Request, S::Error)>
    {
        let request = Request { source_rva: pc, operation: Operation::F32(operation) };
        let result = self.services.f32(pc, operation);
        self.accept(request, result, |v| Word::F32(*v))
    }

    fn helper(&mut self, pc: u32, argument: u32)
        -> Result<u32, (Request, S::Error)>
    {
        let request = Request {
            source_rva: pc, operation: Operation::Helper5b0580 { argument },
        };
        let result = self.services.helper5b0580(pc, argument);
        self.accept(request, result, |v| Word::F32(*v))
    }

    fn ratio_exp(&mut self, pcs: [u32; 5]) -> Result<u32, (Request, S::Error)> {
        let input = self.progress.input;
        let current = self.f32(pcs[0], F32Operation::Cvtsi2ssSigned32(input.current))?;

        if pcs[0] == 0x54b774 {
            self.progress.child_x = Some(input.width.wrapping_sub(self.progress.envelope));
        }
        let total = self.f32(pcs[1], F32Operation::Cvtsi2ssSigned32(input.total))?;
        let ratio = self.f32(pcs[2], F32Operation::Div {
            destination: current, source: total,
        })?;
        let argument = self.f32(pcs[3], F32Operation::Mul {
            destination: ratio, source: NEGATIVE_FOUR,
        })?;
        self.helper(pcs[4], argument)
    }

    fn run(&mut self) -> Result<(u8, ChildRect), (Request, S::Error)> {
        let input = self.progress.input;
        let first = self.ratio_exp([0x54b658, 0x54b660, 0x54b668, 0x54b66c, 0x54b674])?;
        let mut opacity = self.f32(0x54b684, F32Operation::Sub {
            destination: ONE, source: first,
        })?;
        if input.collapsed != 0 {
            opacity = self.f32(0x54b69b, F32Operation::Sub {
                destination: ONE, source: opacity,
            })?;
        }
        let request = Request {
            source_rva: 0x54b6a5, operation: Operation::Cvtss2sd { source: opacity },
        };
        let result = self.services.cvtss2sd(request.source_rva, opacity);
        let wide = self.accept(request, result, |v| Word::F64(*v))?;
        let request = Request {
            source_rva: 0x54b6a9,
            operation: Operation::Mulsd { destination: wide, source: OPACITY_SCALE },
        };
        let result = self.services.mulsd(request.source_rva, wide, OPACITY_SCALE);
        let scaled = self.accept(request, result, |v| Word::F64(*v))?;
        let request = Request {
            source_rva: 0x54b6b1,
            operation: Operation::Addsd { destination: scaled, source: ROUND_BIAS },
        };
        let result = self.services.addsd(request.source_rva, scaled, ROUND_BIAS);
        let biased = self.accept(request, result, |v| Word::F64(*v))?;
        let signed_low_word = biased as u32 as i32;
        let opacity_byte = !(signed_low_word.min(255).max(0) as u8);
        self.progress.opacity_byte = Some(opacity_byte);

        self.progress.envelope = self.progress.envelope.min(input.width);
        let right = input.mode != 0;
        let second = if right {
            self.ratio_exp([0x54b70d, 0x54b718, 0x54b720, 0x54b724, 0x54b72c])?
        } else {
            self.ratio_exp([0x54b774, 0x54b781, 0x54b789, 0x54b78d, 0x54b795])?
        };
        let ease = self.f32(if right { 0x54b737 } else { 0x54b7a0 },
            F32Operation::Sub { destination: ONE, source: second })?;
        let spare = self.f32(if right { 0x54b745 } else { 0x54b7ae },
            F32Operation::Cvtsi2ssSigned32(input.width.wrapping_sub(input.height)))?;
        let mut width = self.f32(if right { 0x54b749 } else { 0x54b7b2 },
            F32Operation::Mul { destination: ease, source: spare })?;
        if input.collapsed != 0 {

            width = self.f32(if right { 0x54b74f } else { 0x54b7b8 },
                F32Operation::Sub { destination: spare, source: width })?;
        }
        let request = Request {
            source_rva: if right { 0x54b756 } else { 0x54b7bf },
            operation: Operation::Cvttss2siSigned32 { source: width },
        };
        let result = self.services.cvttss2si_signed32(request.source_rva, width);
        let width = self.accept(request, result, |v| Word::Signed32(*v))?
            .min(self.progress.envelope);
        let x = if right {
            self.progress.envelope.wrapping_sub(width)
        } else {
            input.width.wrapping_sub(self.progress.envelope)
        };
        self.progress.child_x = Some(x);
        Ok((opacity_byte, ChildRect { x, y: 0, width, height: input.height }))
    }
}

pub fn prepare<S: Services>(input: Input, services: &mut S)
    -> Result<Completed, Failure<S::Error>>
{
    let mut graph = Graph {
        services,
        progress: Progress {
            input,
            completed: Vec::new(),
            envelope: input.width.wrapping_sub(input.height),
            opacity_byte: None,
            child_x: None,
        },
    };
    match graph.run() {
        Ok((opacity_byte, child)) => Ok(Completed {
            opacity_byte, child, progress: graph.progress,
        }),
        Err((attempted, cause)) => Err(Failure {
            cause, attempted, progress: graph.progress,
        }),
    }
}
