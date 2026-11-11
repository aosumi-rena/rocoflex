#[derive(Clone, Debug)]
pub struct FirLayout {
    pub fft_len:usize,
    pub up:usize,
    pub down:usize,
    pub up_shift:i32,
    pub down_shift:i32,
    pub input_fft_len:usize,
    pub output_fft_len:usize,
    pub latency_samples:usize,
    pub history_len:usize,
    pub block_len:usize,
    pub initial_discard:usize,
    pub fractional_latency:f64,
    pub initial_zeros:usize,
    pub down_alignment:usize,
    pub reset_remaining:usize,
}

fn power_shift(factor:usize)->i32 {
    if factor.is_power_of_two() { factor.trailing_zeros() as i32 } else { -1 }
}

impl FirLayout {
    pub fn phase0(tap_count:usize,up:usize,down:usize,input_delay:f64)->Result<Self,&'static str> {
        if tap_count==0 || tap_count%2!=1 || tap_count>32767 || up==0 || down==0 || up>255 || down>255 || !input_delay.is_finite() {
            return Err("bounded source phase0 constructor scope required");
        }
        let extent=tap_count-1;
        let bit_length=(usize::BITS-extent.leading_zeros()).max(1);
        let fft_len=2usize<<bit_length;
        let up_shift=power_shift(up);
        let down_shift=power_shift(down);
        let latency_samples=extent/2;
        let mut history_len=if up_shift>=0 { (tap_count+up-2)/up } else { extent };
        let expanded_history=if up_shift>=0 { history_len*up } else { history_len };
        if expanded_history>=fft_len { return Err("invalid source block geometry"); }
        let mut block_len=fft_len-expanded_history;
        let delay=input_delay*up as f64+0.0;
        if !delay.is_finite() || delay<0.0 || delay>i32::MAX as f64 {
            return Err("finite nonnegative source i32 latency scope required");
        }
        let integer_delay=delay as i32;
        let fractional_latency=(delay-integer_delay as f64)/down as f64;
        let mut initial_discard=block_len+integer_delay as usize+latency_samples;
        let mut initial_zeros=0;
        let mut down_alignment=0;

        if down_shift>=0 && down>=2 && (up_shift<0 || up==1) {
            down_alignment=block_len & (down-1);
            history_len+=down_alignment;
            block_len-=down_alignment;
            initial_discard-=down_alignment;
            let remainder=integer_delay as usize & (down-1);
            if remainder!=0 { initial_zeros=down-remainder; }
        }
        let input_fft_len=if up_shift>=0 { fft_len/up } else { fft_len };
        let output_fft_len=if down_shift>=0 { fft_len/down } else { fft_len };
        if input_fft_len<4 || output_fft_len<4 || block_len<=initial_zeros || initial_discard>i32::MAX as usize {
            return Err("candidate FFT/reset resource scope required");
        }
        let reset_remaining=block_len-initial_zeros;
        let input_block=if up_shift>=0 { block_len/up } else { block_len };
        if history_len+input_block!=input_fft_len { return Err("source input block/history invariant failed"); }
        Ok(Self { fft_len,up,down,up_shift,down_shift,input_fft_len,output_fft_len,
                  latency_samples,history_len,block_len,initial_discard,fractional_latency,
                  initial_zeros,down_alignment,reset_remaining })
    }
}
