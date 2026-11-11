#[derive(Clone, Copy, Debug)]
pub struct Arithmetic {

    pub mxcsr: u32,
    pub separate_operations: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Admission, Validate, Fold, FoldFinish, Below, BelowTest, BelowShift, Bottom,
    AboveMinimum, AboveStart, Above, AboveTest, AboveShift, Origin, Done,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    FloatingPointContract,
    EmptySourceDivision,
    CountConversion,
    NonfiniteInput,
    NonfiniteArithmetic,
    SourceLoopStalled,
    IntegerConversionBoundary,
    MissingDraw,
    AlreadyStopped,
}

#[derive(Clone, Copy, Debug)]
pub struct DrawBoundary {
    pub x: i32,
    pub baseline_y: i32,
    pub maximum_line_width: i32,
    pub justification: i32,
    pub leading: f32,
    pub anchor_x: f32,
    pub candidate_y: f32,
}

pub struct Prepared {
    points: Vec<[f32; 2]>,
    measured_width: f32,
    canvas_height: i32,
    arithmetic: Arithmetic,
    phase: Phase,
    halted: bool,
    cursor: usize,
    sum_x: f32,
    sum_y: f32,
    maximum_y: f32,
    half_width: f32,
    mean_x: f32,
    mean_y: f32,
    candidate_y: f32,
    horizontal: f32,
    test_y: f32,
    draw: Option<DrawBoundary>,
}

pub struct Stopped { pub pending: Prepared, pub fault: Fault }
pub struct Finished { pub owner: Prepared, pub draw: DrawBoundary }
pub enum Poll { Pending(Prepared), Ready(Finished), Failed(Stopped) }

#[derive(Clone, Copy, Debug)]
pub struct DisplayMetrics { pub measured_width: f32, pub canvas_height: i32 }

pub fn projected_trajectory(points: Vec<[f32; 2]>, metrics: DisplayMetrics,
    arithmetic: Arithmetic) -> Result<DrawBoundary, Stopped>
{
    Prepared::display_trajectory(points, metrics, arithmetic).run().map(|finished| finished.draw)
}

pub struct StandaloneStopped {
    pub projected: [f32; 2],
    pub arithmetic: Arithmetic,
    pub fault: Fault,
}

pub fn projected_standalone(projected: [f32; 2], arithmetic: Arithmetic)
    -> Result<DrawBoundary, StandaloneStopped>
{
    standalone_graph(projected, arithmetic)
        .map_err(|fault| StandaloneStopped { projected, arithmetic, fault })
}

impl Prepared {

    pub fn display_trajectory(points: Vec<[f32; 2]>, metrics: DisplayMetrics,
        arithmetic: Arithmetic) -> Self
    {
        Self::from_projected(points, metrics.measured_width, metrics.canvas_height, arithmetic)
    }

    pub fn trajectory(points: Vec<[f32; 2]>, measured_width: f32,
        canvas_height: i32, arithmetic: Arithmetic) -> Self
    {
        Self::from_projected(points, measured_width, canvas_height, arithmetic)
    }
    fn from_projected(points: Vec<[f32; 2]>, measured_width: f32,
        canvas_height: i32, arithmetic: Arithmetic) -> Self
    {
        Self { points, measured_width, canvas_height, arithmetic,
            phase: Phase::Admission, halted: false, cursor: 0,
            sum_x: 0.0, sum_y: 0.0, maximum_y: 0.0, half_width: 0.0,
            mean_x: 0.0, mean_y: 0.0, candidate_y: 0.0,
            horizontal: 0.0, test_y: 0.0, draw: None }
    }
    pub fn phase(&self) -> Phase { self.phase }
    pub fn points(&self) -> &[[f32; 2]] { &self.points }
    pub fn partial_anchor(&self) -> [f32; 2] { [self.mean_x, self.candidate_y] }
    pub fn arithmetic(&self) -> Arithmetic { self.arithmetic }

    pub fn run(mut self) -> Result<Finished, Stopped> {
        if self.halted { return Err(Stopped { pending: self, fault: Fault::AlreadyStopped }); }
        while self.phase != Phase::Done {
            if let Err(fault) = self.step_one() {
                self.halted = true;
                return Err(Stopped { pending: self, fault });
            }
        }
        self.finish()
    }

    pub fn poll(self, work_budget: usize) -> Poll { self.poll_with_work(work_budget).0 }

    pub fn poll_with_work(mut self, work_budget: usize) -> (Poll, usize) {
        if self.halted {
            return (Poll::Failed(Stopped { pending: self, fault: Fault::AlreadyStopped }), 0);
        }
        let mut remaining = work_budget;
        while self.phase != Phase::Done && remaining != 0 {
            remaining -= 1;
            if let Err(fault) = self.step_one() {
                self.halted = true;
                return (Poll::Failed(Stopped { pending: self, fault }), work_budget - remaining);
            }
        }
        let spent = work_budget - remaining;
        let result = if self.phase == Phase::Done {
            match self.finish() { Ok(done) => Poll::Ready(done), Err(stopped) => Poll::Failed(stopped) }
        } else { Poll::Pending(self) };
        (result, spent)
    }

    fn finish(self) -> Result<Finished, Stopped> {
        match self.draw {
            Some(draw) => Ok(Finished { owner: self, draw }),
            None => Err(Stopped { pending: self, fault: Fault::MissingDraw }),
        }
    }

    fn step_one(&mut self) -> Result<(), Fault> {
        match self.phase {
            Phase::Admission => {
                if self.arithmetic.mxcsr & 0xffc0 != 0x1f80 || !self.arithmetic.separate_operations {
                    return Err(Fault::FloatingPointContract);
                }
                if self.points.is_empty() { return Err(Fault::EmptySourceDivision); }
                if self.points.len() > i64::MAX as usize { return Err(Fault::CountConversion); }
                if !self.measured_width.is_finite() { return Err(Fault::NonfiniteInput); }
                self.cursor = 0;
                self.phase = Phase::Validate;
            }
            Phase::Validate => {
                if self.cursor == self.points.len() {

                    self.half_width = self.measured_width * 0.5;
                    self.cursor = 0;
                    self.phase = Phase::Fold;
                } else {
                    let [x, y] = self.points[self.cursor];
                    if !x.is_finite() || !y.is_finite() { return Err(Fault::NonfiniteInput); }
                    self.cursor += 1;
                }
            }
            Phase::Fold => {
                if self.cursor == self.points.len() { self.phase = Phase::FoldFinish; }
                else {
                    let [x, y] = self.points[self.cursor];
                    self.sum_x = self.sum_x + x;
                    self.sum_y = self.sum_y + y;

                    self.maximum_y = if y > self.maximum_y { y } else { self.maximum_y };
                    self.cursor += 1;
                    if !self.sum_x.is_finite() || !self.sum_y.is_finite() {
                        return Err(Fault::NonfiniteArithmetic);
                    }
                }
            }
            Phase::FoldFinish => {
                self.candidate_y = self.maximum_y + 8.0;

                self.half_width = if 50.0 < self.half_width { 50.0 } else { self.half_width };
                let count = self.points.len() as i64 as f32;
                self.mean_x = self.sum_x / count;
                self.mean_y = self.sum_y / count;
                self.horizontal = self.half_width + 4.0;
                self.phase = Phase::Below;
            }
            Phase::Below => {
                if self.candidate_y > self.mean_y {
                    self.cursor = 0;
                    self.phase = Phase::BelowTest;
                } else { self.phase = Phase::Bottom; }
            }
            Phase::BelowTest => {
                if self.cursor == self.points.len() { self.phase = Phase::BelowShift; }
                else if self.near_sample_at(self.cursor, self.candidate_y) { self.phase = Phase::Bottom; }
                else { self.cursor += 1; }
            }
            Phase::BelowShift => {
                self.shift_candidate(-1.0)?;
                self.phase = Phase::Below;
            }
            Phase::Bottom => {

                let bottom = self.canvas_height.wrapping_sub(30) as f32;
                if self.candidate_y > bottom {
                    self.cursor = 0;
                    self.phase = Phase::AboveMinimum;
                } else { self.phase = Phase::Origin; }
            }
            Phase::AboveMinimum => {
                if self.cursor == self.points.len() { self.phase = Phase::AboveStart; }
                else {
                    let y = self.points[self.cursor][1];

                    self.candidate_y = if y < self.candidate_y { y } else { self.candidate_y };
                    self.cursor += 1;
                }
            }
            Phase::AboveStart => {
                self.candidate_y = self.candidate_y + (-38.0);
                self.phase = Phase::Above;
            }
            Phase::Above => {
                if self.mean_y > self.candidate_y {
                    self.test_y = self.candidate_y + 30.0;
                    self.cursor = 0;
                    self.phase = Phase::AboveTest;
                } else { self.phase = Phase::Origin; }
            }
            Phase::AboveTest => {
                if self.cursor == self.points.len() { self.phase = Phase::AboveShift; }
                else if self.near_sample_at(self.cursor, self.test_y) { self.phase = Phase::Origin; }
                else { self.cursor += 1; }
            }
            Phase::AboveShift => {
                self.shift_candidate(1.0)?;
                self.phase = Phase::Above;
            }
            Phase::Origin => {
                let baseline = self.candidate_y + 16.0;
                let x = self.mean_x + (-50.0);
                self.draw = Some(DrawBoundary { x: checked_cvtt(x)?, baseline_y: checked_cvtt(baseline)?,
                    maximum_line_width: 100, justification: 0x24, leading: 0.0,
                    anchor_x: self.mean_x, candidate_y: self.candidate_y });
                self.phase = Phase::Done;
            }
            Phase::Done => {}
        }
        Ok(())
    }

    fn near_sample_at(&self, index: usize, y: f32) -> bool {
        let point = self.points[index];
        let dx = clear_sign(point[0] - self.mean_x);

        if self.horizontal <= dx { return false; }
        let dy = clear_sign(point[1] - y);
        8.0 > dy
    }
    fn shift_candidate(&mut self, delta: f32) -> Result<(), Fault> {
        let next = self.candidate_y + delta;
        if !next.is_finite() { return Err(Fault::NonfiniteArithmetic); }
        if next.to_bits() == self.candidate_y.to_bits() { return Err(Fault::SourceLoopStalled); }
        self.candidate_y = next;
        Ok(())
    }
}

fn clear_sign(value: f32) -> f32 { f32::from_bits(value.to_bits() & 0x7fff_ffff) }

fn checked_cvtt(value: f32) -> Result<i32, Fault> {
    if !value.is_finite() || value < -2147483648.0 || value >= 2147483648.0 {
        Err(Fault::IntegerConversionBoundary)
    } else { Ok(value as i32) }
}

pub fn standalone(projected: [f32; 2], arithmetic: Arithmetic)
    -> Result<DrawBoundary, Fault>
{
    standalone_graph(projected, arithmetic)
}

fn standalone_graph(projected: [f32; 2], arithmetic: Arithmetic)
    -> Result<DrawBoundary, Fault>
{
    if arithmetic.mxcsr & 0xffc0 != 0x1f80 || !arithmetic.separate_operations {
        return Err(Fault::FloatingPointContract);
    }
    if projected.iter().any(|v| !v.is_finite()) { return Err(Fault::NonfiniteInput); }
    let candidate_y = projected[1] + 14.0;
    let baseline = candidate_y + 16.0;
    let anchor_x = projected[0] + 0.0;
    let x = anchor_x + (-50.0);
    Ok(DrawBoundary { x: checked_cvtt(x)?, baseline_y: checked_cvtt(baseline)?,
        maximum_line_width: 100, justification: 0x24, leading: 0.0,
        anchor_x, candidate_y })
}
