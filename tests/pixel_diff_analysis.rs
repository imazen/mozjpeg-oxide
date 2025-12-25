//! Validate pixel quality of Rust encoder across configurations.
//!
//! This test verifies that the Rust encoder produces valid, decodable
//! JPEGs with reasonable quality characteristics.

use mozjpeg_oxide::{Encoder, Subsampling, TrellisConfig};

/// Test that encoded JPEGs are valid and decodable across multiple configurations.
#[test]
fn test_encoder_produces_valid_output() {
    let test_cases = [
        (16, 16, Subsampling::S444, "16x16 4:4:4"),
        (16, 16, Subsampling::S420, "16x16 4:2:0"),
        (17, 17, Subsampling::S420, "17x17 4:2:0 (non-MCU)"),
        (32, 32, Subsampling::S420, "32x32 4:2:0"),
        (64, 64, Subsampling::S420, "64x64 4:2:0"),
    ];

    for (width, height, subsampling, name) in test_cases {
        let w = width as usize;
        let h = height as usize;

        // Create gradient image
        let mut rgb = vec![0u8; w * h * 3];
        for y in 0..h {
            for x in 0..w {
                let idx = (y * w + x) * 3;
                rgb[idx] = ((x * 255) / w.max(1)) as u8;
                rgb[idx + 1] = ((y * 255) / h.max(1)) as u8;
                rgb[idx + 2] = 128;
            }
        }

        // Encode with baseline (no optimizations)
        let jpeg = Encoder::new()
            .quality(85)
            .subsampling(subsampling)
            .progressive(false)
            .optimize_huffman(false)
            .trellis(TrellisConfig::disabled())
            .encode_rgb(&rgb, width, height)
            .expect("Encoding failed");

        // Verify JPEG is valid and decodable
        let mut decoder = jpeg_decoder::Decoder::new(std::io::Cursor::new(&jpeg));
        let decoded = decoder.decode().expect(&format!("{}: decode failed", name));
        let info = decoder.info().expect("Failed to get JPEG info");

        // Verify dimensions match
        assert_eq!(info.width as u32, width, "{}: width mismatch", name);
        assert_eq!(info.height as u32, height, "{}: height mismatch", name);

        // Verify decoded size matches original (width * height * 3 for RGB)
        assert_eq!(
            decoded.len(),
            w * h * 3,
            "{}: decoded size mismatch",
            name
        );

        // Calculate PSNR to verify quality is reasonable
        let psnr = calculate_psnr(&rgb, &decoded);
        assert!(
            psnr > 30.0,
            "{}: PSNR too low ({:.1} dB), quality issue",
            name,
            psnr
        );
    }
}

/// Test encoder with progressive mode.
#[test]
fn test_progressive_encoder_valid() {
    let width = 32u32;
    let height = 32u32;
    let w = width as usize;
    let h = height as usize;

    let mut rgb = vec![0u8; w * h * 3];
    for i in 0..rgb.len() {
        rgb[i] = (i % 256) as u8;
    }

    let jpeg = Encoder::new()
        .quality(85)
        .progressive(true)
        .encode_rgb(&rgb, width, height)
        .expect("Progressive encoding failed");

    // Verify it's decodable
    let decoded = jpeg_decoder::Decoder::new(std::io::Cursor::new(&jpeg))
        .decode()
        .expect("Progressive decode failed");

    assert_eq!(decoded.len(), w * h * 3);

    let psnr = calculate_psnr(&rgb, &decoded);
    // Note: PSNR of 25+ dB is acceptable for Q85 with synthetic patterns
    assert!(psnr > 20.0, "Progressive PSNR too low: {:.1}", psnr);
}

fn calculate_psnr(original: &[u8], decoded: &[u8]) -> f64 {
    if original.len() != decoded.len() {
        return 0.0;
    }

    let mut mse = 0f64;
    for (o, d) in original.iter().zip(decoded.iter()) {
        let diff = *o as f64 - *d as f64;
        mse += diff * diff;
    }
    mse /= original.len() as f64;

    if mse == 0.0 {
        return f64::INFINITY;
    }

    10.0 * (255.0 * 255.0 / mse).log10()
}
