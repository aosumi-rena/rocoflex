pub const SIN_BITS: [u32; 9] = [
    0x0, 0x3f3504f3, 0x3f800000, 0x3f3504f3, 0xb3bbbd2e, 0xbf3504f5, 0xbf800000, 0xbf3504ef,
    0x343bbd2e,
];
pub const COS_BITS: [u32; 9] = [
    0x3f800000, 0x3f3504f3, 0xb33bbd2e, 0xbf3504f3, 0xbf800000, 0xbf3504f1, 0x324cde2e, 0x3f3504f7,
    0x3f800000,
];
pub fn vertices(xy: [f32; 2]) -> [[f32; 2]; 9] {
    std::array::from_fn(|i| {
        [
            f32::from_bits(COS_BITS[i]) * 0.05f32 + xy[0],
            f32::from_bits(SIN_BITS[i]) * 0.05f32 + xy[1],
        ]
    })
}
