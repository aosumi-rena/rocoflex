const EXP_HIGH: [u64; 32] = [
    0x3ff0000000000000, 0x3ff059b0d0000000, 0x3ff0b55860000000, 0x3ff11301d0000000,
    0x3ff172b830000000, 0x3ff1d48730000000, 0x3ff2387a60000000, 0x3ff29e9df0000000,
    0x3ff306fe00000000, 0x3ff371a730000000, 0x3ff3dea640000000, 0x3ff44e0860000000,
    0x3ff4bfdad0000000, 0x3ff5342b50000000, 0x3ff5ab07d0000000, 0x3ff6247eb0000000,
    0x3ff6a09e60000000, 0x3ff71f75e0000000, 0x3ff7a11470000000, 0x3ff8258990000000,
    0x3ff8ace540000000, 0x3ff93737b0000000, 0x3ff9c49180000000, 0x3ffa5503b0000000,
    0x3ffae89f90000000, 0x3ffb7f76f0000000, 0x3ffc199bd0000000, 0x3ffcb720d0000000,
    0x3ffd5818d0000000, 0x3ffdfc9730000000, 0x3ffea4afa0000000, 0x3fff507650000000,
];

const EXP_LOW: [u64; 32] = [
    0x0000000000000000, 0x3e48ac2ba1d73e2a, 0x3e69f3121ec53172, 0x3df25b50a4ebbf1b,
    0x3e68faa2f5b9bef9, 0x3e368b9aa7805b80, 0x3e6ceac470cd83f6, 0x3e547f7b84b09745,
    0x3e64636e2a5bd1ab, 0x3e5ceaa72a9c5154, 0x3e682468446b6824, 0x3e18624b40c4dbd0,
    0x3e54d8a89c750e5e, 0x3e5a753e077c2a0f, 0x3e6a90a852b19260, 0x3e0d2ac258f87d03,
    0x3e59fcef32422cbf, 0x3e61d8bee7ba46e2, 0x3e4f580c36bea881, 0x3e62999c25159f11,
    0x3e415506dadd3e2a, 0x3e29b8bc9e8a0388, 0x3e451f8480e3e236, 0x3e41f12ae45a1224,
    0x3e62b5a75abd0e6a, 0x3e47daf237553d84, 0x3e6b0aa538444196, 0x3e69df20d22a0798,
    0x3e69f7490e4bb40b, 0x3e4bdcdaf5cb4656, 0x3e452486cc2c7b9d, 0x3e66dc8a80ce9f09,
];

fn split_positive_exp(argument: f64) -> (i32, f64, f64) {
    let mut integer_argument = argument * f64::from_bits(0x40471547652b82fe);
    integer_argument += 0.5;
    let integer = integer_argument.trunc() as i32;
    let index = (integer & 31) as usize;
    let exponent = (integer - index as i32) / 32;
    let high = f64::from_bits(EXP_HIGH[index]);
    let low = f64::from_bits(EXP_LOW[index]);
    let mut remainder_high = f64::from(integer);
    remainder_high *= f64::from_bits(0x3f962e42fe000000);
    remainder_high = argument - remainder_high;
    remainder_high *= 1.0;
    let mut remainder_low = f64::from(-integer);
    remainder_low *= f64::from_bits(0x3dcf473de6af278e);
    remainder_low *= 1.0;
    let mut remainder = remainder_low + remainder_high;
    let mut polynomial = remainder * f64::from_bits(0x3f56c1728d739765);
    polynomial += f64::from_bits(0x3f811115b7aa905e);
    polynomial *= remainder;
    polynomial += f64::from_bits(0x3fa5555555545d4e);
    polynomial *= remainder;
    polynomial += f64::from_bits(0x3fc5555555548f7c);
    polynomial *= remainder;
    remainder *= remainder;
    polynomial += 0.5;
    polynomial *= remainder;
    let table_sum = low + high;
    polynomial += remainder_low;
    polynomial += remainder_high;
    polynomial *= table_sum;
    polynomial += low;
    (exponent, high, polynomial)
}

pub fn finite_tanh(input: f64) -> Result<f64, &'static str> {
    if !input.is_finite() {
        return Err("finite hyperbolic scalar input required");
    }
    let magnitude_bits = input.to_bits() & 0x7fff_ffff_ffff_ffff;
    if magnitude_bits < 0x3e30000000000000 {
        return Ok(input);
    }
    let negative = input.is_sign_negative();
    let mut magnitude = f64::from_bits(magnitude_bits);
    if magnitude > 20.0 {
        magnitude = 1.0;
    } else if magnitude <= 1.0 {
        let square = magnitude * magnitude;
        let cube = square * magnitude;
        let coefficients = if magnitude < f64::from_bits(0x3feccccccccccccd) {
            [0x3e4e82d10d09af81, 0xbf2a387bfaf479c9, 0x3f2b68b3cecad284,
             0x3f94a3d4cc7a88a9, 0x3f920629b90302ae, 0x3fd86cd01c4ab94c,
             0x3fd189b6e8007758, 0x3fea4e925c00b304]
        } else {
            [0x3e48cc5a847e6cec, 0xbf25b483c69835dd, 0x3f26af77f334e750,
             0x3f912335baec487f, 0x3f8defad6e212118, 0x3fd44d1459bcc5f6,
             0x3fcd28597c5ae288, 0x3fe5de431d442afd]
        };
        let first = square * f64::from_bits(coefficients[0]);
        let mut numerator = f64::from_bits(coefficients[1]) - first;
        let mut denominator = square * f64::from_bits(coefficients[2]);
        numerator *= square;
        denominator += f64::from_bits(coefficients[3]);
        numerator -= f64::from_bits(coefficients[4]);
        denominator *= square;
        numerator *= square;
        denominator += f64::from_bits(coefficients[5]);
        numerator -= f64::from_bits(coefficients[6]);
        denominator *= square;
        denominator += f64::from_bits(coefficients[7]);
        numerator *= cube;
        numerator /= denominator;
        magnitude += numerator;
    } else {
        let doubled = magnitude + magnitude;
        let (exponent, high, low) = split_positive_exp(doubled);
        let half = exponent / 2;
        let remainder = exponent - half;
        let first_scale = f64::from_bits(((half + 1023) as u64) << 52);
        let second_scale = f64::from_bits(((remainder + 1023) as u64) << 52);
        let mut denominator = low + high;
        denominator *= first_scale;
        denominator *= second_scale;
        denominator += 1.0;
        magnitude = 1.0 - 2.0 / denominator;
    }
    if negative { magnitude = f64::from_bits(magnitude.to_bits() ^ 0x8000_0000_0000_0000); }
    Ok(magnitude)
}
