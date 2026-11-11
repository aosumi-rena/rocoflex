#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputSelector {
    period: i32,
    down: i32,
    down_shift: i32,
    discard: i32,
    phase: i32,
}

impl OutputSelector {
    pub fn from_source_state(period:i32,down:i32,discard:i32,phase:i32) -> Result<Self,&'static str> {
        if period <= 0 || period > (1<<26) || down <= 0 || down > period || discard < 0 || phase < 0 || phase >= down {
            return Err("bounded valid source output state required");
        }
        let down_shift = if (down as u32).is_power_of_two() { (down as u32).trailing_zeros() as i32 } else { -1 };
        Ok(Self { period,down,down_shift,discard,phase })
    }

    pub fn pending_discard(&self)->i32 { self.discard }
    pub fn phase(&self)->i32 { self.phase }

    pub fn append(&mut self,storage:&[f64],offset:i32,count:i32,output:&mut Vec<f64>) -> Result<usize,&'static str> {
        if count < 0 || count > self.period || offset < -self.period || offset >= self.period || offset+count > self.period {
            return Err("source single-period view scope required");
        }
        let required = if self.down_shift > 0 { (self.period+self.down-1)/self.down } else { self.period };
        if storage.len() < required as usize { return Err("source shortened/full output storage missing"); }
        let initial = output.len();
        self.append_view(storage,offset,count,output);
        Ok(output.len()-initial)
    }

    fn append_view(&mut self,storage:&[f64],mut offset:i32,mut count:i32,output:&mut Vec<f64>) {
        if offset < 0 {
            let remainder = count+offset;
            if remainder > 0 {
                self.append_view(storage,offset+self.period,-offset,output);
                offset = 0;
                count = remainder;
            } else { offset += self.period; }
        }
        if self.discard != 0 {
            let remaining = self.discard-count;
            if remaining >= 0 { self.discard=remaining;return; }
            offset += self.discard;
            count -= self.discard;
            self.discard = 0;
        }
        if self.down_shift > 0 {
            let remainder = offset & (self.down-1);
            let alignment = if remainder > 0 { self.down-remainder } else { 0 };
            count -= alignment;
            if count <= 0 { return; }
            let produced = (count+self.down-1) >> self.down_shift;
            let start = (alignment+offset) >> self.down_shift;

            output.extend_from_slice(&storage[start as usize..(start+produced) as usize]);
        } else if self.down >= 2 {
            let previous_phase = self.phase;
            let produced = (self.down-1-previous_phase+count)/self.down;
            self.phase = previous_phase-count+produced*self.down;
            for index in 0..produced {
                output.push(storage[(offset+previous_phase+index*self.down) as usize]);
            }
        } else {
            output.extend_from_slice(&storage[offset as usize..(offset+count) as usize]);
        }
    }
}
