pub const FALSE_THRESHOLDS: [f64; 11] = [
    f64::from_bits(0x404bc5b573eab368),
    f64::from_bits(0x40545ad288ce703b),
    f64::from_bits(0x405815b573eab368),
    f64::from_bits(0x405bc86a7ef9db23),
    f64::from_bits(0x405f5dc779a6b50b),
    f64::from_bits(0x4061779ce075f6fd),
    f64::from_bits(0x406341b3d07c84b6),
    f64::from_bits(0x406506b923a29c78),
    f64::from_bits(0x4066c371758e2196),
    f64::from_bits(0x4068722339c0ebee),
    f64::from_bits(0x406a21f3b645a1cb),
];
pub const FALSE_PROFILES: [[f64;3]; 12] = [
    [f64::from_bits(0x401085fcb7b18eef),f64::from_bits(0x3ff2cddb556a5c72),f64::from_bits(0x404bc5b573eab368)],
    [f64::from_bits(0x4011b254e90f03f2),f64::from_bits(0x3ffcceccb1642668),f64::from_bits(0x40545ad288ce703b)],
    [f64::from_bits(0x40150bccbecd60ad),f64::from_bits(0x3ffd03683af2b424),f64::from_bits(0x405815b573eab368)],
    [f64::from_bits(0x4017c60420cfcc1e),f64::from_bits(0x3ffdf7e26482816d),f64::from_bits(0x405bc86a7ef9db23)],
    [f64::from_bits(0x401b52ce7dec8ef6),f64::from_bits(0x3ffdade5d476d161),f64::from_bits(0x405f5dc779a6b50b)],
    [f64::from_bits(0x401ea8cd5852491d),f64::from_bits(0x3ffdb489aca8786e),f64::from_bits(0x4061779ce075f6fd)],
    [f64::from_bits(0x40206862063bb94a),f64::from_bits(0x3ffed4cbcfae4068),f64::from_bits(0x406341b3d07c84b6)],
    [f64::from_bits(0x402192b21b42cc26),f64::from_bits(0x3fffa3f74d200001),f64::from_bits(0x406506b923a29c78)],
    [f64::from_bits(0x4023306289d5453c),f64::from_bits(0x3fff8cae136db238),f64::from_bits(0x4066c371758e2196)],
    [f64::from_bits(0x4025085a52d953e5),f64::from_bits(0x3fff34ddbdeac478),f64::from_bits(0x4068722339c0ebee)],
    [f64::from_bits(0x402479fb72ad0975),f64::from_bits(0x40014981ecf4dc15),f64::from_bits(0x406a21f3b645a1cb)],
    [f64::from_bits(0x4025fec63787f2cd),f64::from_bits(0x40013aae9be1a21c),f64::from_bits(0x406bd0083126e979)],
];
pub const TRUE_THRESHOLDS: [f64; 9] = [
    f64::from_bits(0x4050a47c84b5dcc6),
    f64::from_bits(0x40559e2d0e560419),
    f64::from_bits(0x405a87a5e353f7cf),
    f64::from_bits(0x405f4ec3c9eecbfb),
    f64::from_bits(0x40620b19ce075f70),
    f64::from_bits(0x40646d1de69ad42c),
    f64::from_bits(0x4066a23886594af5),
    f64::from_bits(0x4068e9374bc6a7f0),
    f64::from_bits(0x406b15f7ced91687),
];
pub const TRUE_PROFILES: [[f64;3]; 10] = [
    [f64::from_bits(0x400fe92d92e139df),f64::from_bits(0x3ff964527a0a235f),f64::from_bits(0x4050a47c84b5dcc6)],
    [f64::from_bits(0x4012cb6ffe00657a),f64::from_bits(0x3ffcf00dbf221a67),f64::from_bits(0x40559e2d0e560419)],
    [f64::from_bits(0x401665e532a6ad62),f64::from_bits(0x3ffe49cb7f651882),f64::from_bits(0x405a87a5e353f7cf)],
    [f64::from_bits(0x4019736f2c5e1d24),f64::from_bits(0x3fffe9c74c9c398c),f64::from_bits(0x405f4ec3c9eecbfb)],
    [f64::from_bits(0x401db84624653b33),f64::from_bits(0x3fffd4563d6722f9),f64::from_bits(0x40620b19ce075f70)],
    [f64::from_bits(0x4020222fe2ec9085),f64::from_bits(0x4000be187d175f5b),f64::from_bits(0x40646d1de69ad42c)],
    [f64::from_bits(0x40217e7664cccd04),f64::from_bits(0x40014fede697c72c),f64::from_bits(0x4066a23886594af5)],
    [f64::from_bits(0x40242a28dcd418d9),f64::from_bits(0x4000b7a3c0bb0a24),f64::from_bits(0x4068e9374bc6a7f0)],
    [f64::from_bits(0x4025d82d4cb71ff9),f64::from_bits(0x4000fa3346fe2eba),f64::from_bits(0x406b15f7ced91687)],
    [f64::from_bits(0x403533a77c83f996),f64::from_bits(0x3ff2f8883ea0967b),f64::from_bits(0x406d3d66cf41f213)],
];
pub const FRACTIONAL_COEFFICIENT_000: f64 = f64::from_bits(0x4049000000000000);
pub const FRACTIONAL_COEFFICIENT_001: f64 = f64::from_bits(0x401999999999999a);
pub const FRACTIONAL_COEFFICIENT_002: f64 = f64::from_bits(0x4075e00000000000);
pub const SCALAR_COEFFICIENT_008: f64 = f64::from_bits(0x3ff0000000000000);
pub const FRACTIONAL_COEFFICIENT_003: f64 = f64::from_bits(0x4086280000000000);
pub const SOURCE_NEAR_ZERO: f64 = f64::from_bits(0x3d50000000000000);
