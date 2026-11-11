const PALETTE: [u32; 12] = [
    0xff7db235, 0xff4794cb, 0xffd14f5b, 0xffd6bc43, 0xff3bb26c, 0xff9153d4, 0xffb853b3, 0xffc2a153,
    0xff6959d4, 0xffc27455, 0xff5478c7, 0xff3eb2a8,
];

pub fn next_color<'a>(colors: impl IntoIterator<Item = &'a str>) -> String {
    let mut counts = [0usize; 12];
    for color in colors {
        if let Ok(argb) = u32::from_str_radix(color.trim_start_matches('#'), 16)
            && let Some(index) = PALETTE.iter().position(|&value| value == argb)
        {
            counts[index] += 1;
        }
    }
    let mut least = 0;
    for index in 1..counts.len() {
        if counts[index] < counts[least] {
            least = index;
        }
    }
    format!("{:08x}", PALETTE[least])
}

