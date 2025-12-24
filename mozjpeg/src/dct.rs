//! Forward DCT (Discrete Cosine Transform) implementation.
//!
//! This implements the Loeffler-Ligtenberg-Moschytz algorithm for 8x8 DCT,
//! matching mozjpeg's jfdctint.c (integer slow DCT).
//!
//! The algorithm uses 12 multiplies and 32 adds per 1-D DCT.
//! A 2-D DCT is done by 1-D DCT on rows followed by 1-D DCT on columns.
//!
//! Note: The output is scaled up by a factor of 8 compared to a true DCT.
//! This scaling is removed during quantization (in the encoder pipeline).
//!
//! Reference: C. Loeffler, A. Ligtenberg and G. Moschytz,
//! "Practical Fast 1-D DCT Algorithms with 11 Multiplications",
//! Proc. ICASSP 1989, pp. 988-991.

use crate::consts::{DCTSIZE, DCTSIZE2};

// Fixed-point constants for 13-bit precision (CONST_BITS = 13)
const CONST_BITS: i32 = 13;
const PASS1_BITS: i32 = 2;

// Pre-calculated fixed-point constants: FIX(x) = (x * (1 << CONST_BITS) + 0.5)
const FIX_0_298631336: i32 = 2446;   // FIX(0.298631336)
const FIX_0_390180644: i32 = 3196;   // FIX(0.390180644)
const FIX_0_541196100: i32 = 4433;   // FIX(0.541196100)
const FIX_0_765366865: i32 = 6270;   // FIX(0.765366865)
const FIX_0_899976223: i32 = 7373;   // FIX(0.899976223)
const FIX_1_175875602: i32 = 9633;   // FIX(1.175875602)
const FIX_1_501321110: i32 = 12299;  // FIX(1.501321110)
const FIX_1_847759065: i32 = 15137;  // FIX(1.847759065)
const FIX_1_961570560: i32 = 16069;  // FIX(1.961570560)
const FIX_2_053119869: i32 = 16819;  // FIX(2.053119869)
const FIX_2_562915447: i32 = 20995;  // FIX(2.562915447)
const FIX_3_072711026: i32 = 25172;  // FIX(3.072711026)

/// DESCALE: Right-shift with rounding (used to remove fixed-point scaling)
#[inline]
fn descale(x: i32, n: i32) -> i32 {
    // Round by adding 2^(n-1) before shifting
    (x + (1 << (n - 1))) >> n
}

/// Perform forward DCT on one 8x8 block of samples.
///
/// Input: 64 sample values in row-major order (0-255 for 8-bit JPEG)
/// Output: 64 DCT coefficients in row-major order
///
/// Note: The output is scaled up by a factor of 8. This is intentional
/// and matches libjpeg/mozjpeg behavior - the scaling is removed during
/// quantization.
///
/// # Arguments
/// * `samples` - Input 8x8 block of pixel samples (typically centered around 0)
/// * `coeffs` - Output 8x8 block of DCT coefficients
pub fn forward_dct_8x8(samples: &[i16; DCTSIZE2], coeffs: &mut [i16; DCTSIZE2]) {
    // Work buffer (we modify in place across both passes)
    let mut data = [0i32; DCTSIZE2];

    // Convert input to i32 for processing
    for i in 0..DCTSIZE2 {
        data[i] = samples[i] as i32;
    }

    // Pass 1: process rows
    // Results are scaled up by sqrt(8) and by 2^PASS1_BITS
    for row in 0..DCTSIZE {
        let base = row * DCTSIZE;

        let tmp0 = data[base + 0] + data[base + 7];
        let tmp7 = data[base + 0] - data[base + 7];
        let tmp1 = data[base + 1] + data[base + 6];
        let tmp6 = data[base + 1] - data[base + 6];
        let tmp2 = data[base + 2] + data[base + 5];
        let tmp5 = data[base + 2] - data[base + 5];
        let tmp3 = data[base + 3] + data[base + 4];
        let tmp4 = data[base + 3] - data[base + 4];

        // Even part (per Loeffler figure 1)
        let tmp10 = tmp0 + tmp3;
        let tmp13 = tmp0 - tmp3;
        let tmp11 = tmp1 + tmp2;
        let tmp12 = tmp1 - tmp2;

        data[base + 0] = (tmp10 + tmp11) << PASS1_BITS;
        data[base + 4] = (tmp10 - tmp11) << PASS1_BITS;

        let z1 = (tmp12 + tmp13) * FIX_0_541196100;
        data[base + 2] = descale(z1 + tmp13 * FIX_0_765366865, CONST_BITS - PASS1_BITS);
        data[base + 6] = descale(z1 + tmp12 * (-FIX_1_847759065), CONST_BITS - PASS1_BITS);

        // Odd part (per Loeffler figure 8)
        let z1 = tmp4 + tmp7;
        let z2 = tmp5 + tmp6;
        let z3 = tmp4 + tmp6;
        let z4 = tmp5 + tmp7;
        let z5 = (z3 + z4) * FIX_1_175875602; // sqrt(2) * c3

        let tmp4 = tmp4 * FIX_0_298631336;  // sqrt(2) * (-c1+c3+c5-c7)
        let tmp5 = tmp5 * FIX_2_053119869;  // sqrt(2) * ( c1+c3-c5+c7)
        let tmp6 = tmp6 * FIX_3_072711026;  // sqrt(2) * ( c1+c3+c5-c7)
        let tmp7 = tmp7 * FIX_1_501321110;  // sqrt(2) * ( c1+c3-c5-c7)
        let z1 = z1 * (-FIX_0_899976223);   // sqrt(2) * ( c7-c3)
        let z2 = z2 * (-FIX_2_562915447);   // sqrt(2) * (-c1-c3)
        let z3 = z3 * (-FIX_1_961570560) + z5;  // sqrt(2) * (-c3-c5)
        let z4 = z4 * (-FIX_0_390180644) + z5;  // sqrt(2) * ( c5-c3)

        data[base + 7] = descale(tmp4 + z1 + z3, CONST_BITS - PASS1_BITS);
        data[base + 5] = descale(tmp5 + z2 + z4, CONST_BITS - PASS1_BITS);
        data[base + 3] = descale(tmp6 + z2 + z3, CONST_BITS - PASS1_BITS);
        data[base + 1] = descale(tmp7 + z1 + z4, CONST_BITS - PASS1_BITS);
    }

    // Pass 2: process columns
    // We remove PASS1_BITS scaling but leave results scaled by factor of 8
    for col in 0..DCTSIZE {
        let tmp0 = data[DCTSIZE * 0 + col] + data[DCTSIZE * 7 + col];
        let tmp7 = data[DCTSIZE * 0 + col] - data[DCTSIZE * 7 + col];
        let tmp1 = data[DCTSIZE * 1 + col] + data[DCTSIZE * 6 + col];
        let tmp6 = data[DCTSIZE * 1 + col] - data[DCTSIZE * 6 + col];
        let tmp2 = data[DCTSIZE * 2 + col] + data[DCTSIZE * 5 + col];
        let tmp5 = data[DCTSIZE * 2 + col] - data[DCTSIZE * 5 + col];
        let tmp3 = data[DCTSIZE * 3 + col] + data[DCTSIZE * 4 + col];
        let tmp4 = data[DCTSIZE * 3 + col] - data[DCTSIZE * 4 + col];

        // Even part
        let tmp10 = tmp0 + tmp3;
        let tmp13 = tmp0 - tmp3;
        let tmp11 = tmp1 + tmp2;
        let tmp12 = tmp1 - tmp2;

        data[DCTSIZE * 0 + col] = descale(tmp10 + tmp11, PASS1_BITS);
        data[DCTSIZE * 4 + col] = descale(tmp10 - tmp11, PASS1_BITS);

        let z1 = (tmp12 + tmp13) * FIX_0_541196100;
        data[DCTSIZE * 2 + col] = descale(z1 + tmp13 * FIX_0_765366865, CONST_BITS + PASS1_BITS);
        data[DCTSIZE * 6 + col] = descale(z1 + tmp12 * (-FIX_1_847759065), CONST_BITS + PASS1_BITS);

        // Odd part
        let z1 = tmp4 + tmp7;
        let z2 = tmp5 + tmp6;
        let z3 = tmp4 + tmp6;
        let z4 = tmp5 + tmp7;
        let z5 = (z3 + z4) * FIX_1_175875602;

        let tmp4 = tmp4 * FIX_0_298631336;
        let tmp5 = tmp5 * FIX_2_053119869;
        let tmp6 = tmp6 * FIX_3_072711026;
        let tmp7 = tmp7 * FIX_1_501321110;
        let z1 = z1 * (-FIX_0_899976223);
        let z2 = z2 * (-FIX_2_562915447);
        let z3 = z3 * (-FIX_1_961570560) + z5;
        let z4 = z4 * (-FIX_0_390180644) + z5;

        data[DCTSIZE * 7 + col] = descale(tmp4 + z1 + z3, CONST_BITS + PASS1_BITS);
        data[DCTSIZE * 5 + col] = descale(tmp5 + z2 + z4, CONST_BITS + PASS1_BITS);
        data[DCTSIZE * 3 + col] = descale(tmp6 + z2 + z3, CONST_BITS + PASS1_BITS);
        data[DCTSIZE * 1 + col] = descale(tmp7 + z1 + z4, CONST_BITS + PASS1_BITS);
    }

    // Copy results to output
    for i in 0..DCTSIZE2 {
        coeffs[i] = data[i] as i16;
    }
}

/// Prepare a sample block for DCT by level-shifting (centering around 0).
///
/// JPEG requires samples to be centered around 0 before DCT.
/// For 8-bit samples (0-255), subtract 128.
///
/// # Arguments
/// * `samples` - Input samples (0-255)
/// * `output` - Output level-shifted samples (-128 to 127)
pub fn level_shift(samples: &[u8; DCTSIZE2], output: &mut [i16; DCTSIZE2]) {
    for i in 0..DCTSIZE2 {
        output[i] = (samples[i] as i16) - 128;
    }
}

/// Combined level-shift and forward DCT.
///
/// # Arguments
/// * `samples` - Input 8x8 block of pixel samples (0-255)
/// * `coeffs` - Output 8x8 block of DCT coefficients
pub fn forward_dct(samples: &[u8; DCTSIZE2], coeffs: &mut [i16; DCTSIZE2]) {
    let mut shifted = [0i16; DCTSIZE2];
    level_shift(samples, &mut shifted);
    forward_dct_8x8(&shifted, coeffs);
}

/// Combined level-shift, overshoot deringing, and forward DCT.
///
/// This variant applies mozjpeg's overshoot deringing preprocessing to reduce
/// visible ringing artifacts near hard edges on white backgrounds.
///
/// # Arguments
/// * `samples` - Input 8x8 block of pixel samples (0-255)
/// * `coeffs` - Output 8x8 block of DCT coefficients
/// * `dc_quant` - DC quantization value (used to limit overshoot amount)
///
/// # See Also
/// [`crate::deringing::preprocess_deringing`] for algorithm details.
pub fn forward_dct_with_deringing(
    samples: &[u8; DCTSIZE2],
    coeffs: &mut [i16; DCTSIZE2],
    dc_quant: u16,
) {
    use crate::deringing::preprocess_deringing;

    let mut shifted = [0i16; DCTSIZE2];
    level_shift(samples, &mut shifted);
    preprocess_deringing(&mut shifted, dc_quant);
    forward_dct_8x8(&shifted, coeffs);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_level_shift() {
        let samples = [128u8; DCTSIZE2];
        let mut output = [0i16; DCTSIZE2];
        level_shift(&samples, &mut output);

        // 128 - 128 = 0
        for v in output.iter() {
            assert_eq!(*v, 0);
        }

        let mut samples2 = [0u8; DCTSIZE2];
        samples2[0] = 255;
        samples2[1] = 0;
        level_shift(&samples2, &mut output);
        assert_eq!(output[0], 127);  // 255 - 128
        assert_eq!(output[1], -128); // 0 - 128
    }

    #[test]
    fn test_dc_coefficient() {
        // A flat block of all same values should have:
        // - DC coefficient = 8 * 8 * value (due to 2D DCT scaling, factor of 8 per dimension)
        // - All AC coefficients = 0
        let mut samples = [0i16; DCTSIZE2];
        for i in 0..DCTSIZE2 {
            samples[i] = 100; // Flat value after level shift
        }

        let mut coeffs = [0i16; DCTSIZE2];
        forward_dct_8x8(&samples, &mut coeffs);

        // DC should be 8 * 8 * value = 64 * value = 6400
        // (Factor of 8 from row pass and factor of 8 from column pass)
        assert_eq!(coeffs[0], 6400, "DC coefficient should be 64 * input value for flat block");

        // All AC coefficients should be 0 (or very close due to rounding)
        for i in 1..DCTSIZE2 {
            assert!(coeffs[i].abs() <= 1,
                "AC coefficient [{}] should be ~0 for flat block, got {}",
                i, coeffs[i]);
        }
    }

    #[test]
    fn test_zero_block() {
        let samples = [0i16; DCTSIZE2];
        let mut coeffs = [0i16; DCTSIZE2];
        forward_dct_8x8(&samples, &mut coeffs);

        // All coefficients should be 0
        for i in 0..DCTSIZE2 {
            assert_eq!(coeffs[i], 0, "Coefficient [{}] should be 0 for zero block", i);
        }
    }

    #[test]
    fn test_alternating_pattern() {
        // Horizontal stripes should produce non-zero vertical frequency components
        let mut samples = [0i16; DCTSIZE2];
        for row in 0..DCTSIZE {
            let val = if row % 2 == 0 { 100 } else { -100 };
            for col in 0..DCTSIZE {
                samples[row * DCTSIZE + col] = val;
            }
        }

        let mut coeffs = [0i16; DCTSIZE2];
        forward_dct_8x8(&samples, &mut coeffs);

        // DC should be 0 (equal positive and negative)
        assert!(coeffs[0].abs() <= 1, "DC should be ~0 for balanced pattern");

        // For alternating rows (+100/-100), energy should be concentrated in
        // the vertical high frequency components. Check that there's significant
        // energy in AC coefficients (any row > 0, col 0).
        let mut max_vertical_ac = 0i16;
        for row in 1..DCTSIZE {
            max_vertical_ac = max_vertical_ac.max(coeffs[row * DCTSIZE].abs());
        }
        assert!(max_vertical_ac > 50, "Vertical AC frequencies should be present, got {}", max_vertical_ac);
    }

    #[test]
    fn test_gradient() {
        // Horizontal gradient should produce low-frequency horizontal component
        let mut samples = [0i16; DCTSIZE2];
        for row in 0..DCTSIZE {
            for col in 0..DCTSIZE {
                samples[row * DCTSIZE + col] = (col as i16 - 4) * 20;
            }
        }

        let mut coeffs = [0i16; DCTSIZE2];
        forward_dct_8x8(&samples, &mut coeffs);

        // The horizontal gradient should produce significant energy at position [0][1]
        // (first horizontal AC coefficient)
        assert!(coeffs[1].abs() > 100, "Horizontal low frequency should be present");
    }

    #[test]
    fn test_descale_rounding() {
        // Test that descale rounds correctly (rounds toward negative infinity)
        assert_eq!(descale(7, 2), 2);   // (7+2) >> 2 = 9 >> 2 = 2
        assert_eq!(descale(8, 2), 2);   // (8+2) >> 2 = 10 >> 2 = 2
        assert_eq!(descale(9, 2), 2);   // (9+2) >> 2 = 11 >> 2 = 2
        assert_eq!(descale(10, 2), 3);  // (10+2) >> 2 = 12 >> 2 = 3

        // Negative values (arithmetic right shift rounds toward -infinity)
        assert_eq!(descale(-7, 2), -2);  // (-7+2) >> 2 = -5 >> 2 = -2
        assert_eq!(descale(-8, 2), -2);  // (-8+2) >> 2 = -6 >> 2 = -2
        assert_eq!(descale(-9, 2), -2);  // (-9+2) >> 2 = -7 >> 2 = -2
        assert_eq!(descale(-10, 2), -2); // (-10+2) >> 2 = -8 >> 2 = -2
    }
}
