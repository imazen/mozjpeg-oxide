//! Profile baseline encoding (no trellis, no Huffman optimization)
//! Run with: samply record cargo run --release --example profile_baseline

use mozjpeg_rs::{Encoder, Subsampling, TrellisConfig};

fn main() {
    let width = 2048u32;
    let height = 2048u32;
    let iterations = 100;

    // Create test image with realistic-ish content
    let mut rgb = vec![0u8; (width * height * 3) as usize];
    for y in 0..height as usize {
        for x in 0..width as usize {
            let idx = (y * width as usize + x) * 3;
            let noise = ((x * 7 + y * 13) % 50) as u8;
            rgb[idx] = ((x * 255 / width as usize) as u8).saturating_add(noise);
            rgb[idx + 1] = ((y * 255 / height as usize) as u8).saturating_add(noise);
            rgb[idx + 2] =
                (((x + y) * 255 / (width + height) as usize) as u8).saturating_add(noise);
        }
    }

    println!(
        "Profiling baseline encoding: {}x{}, {} iterations",
        width, height, iterations
    );
    println!("Settings: no trellis, no Huffman optimization, 4:2:0 subsampling");

    // Warmup
    let encoder = Encoder::fastest()
        .quality(85)
        .subsampling(Subsampling::S420)
        .progressive(false)
        .optimize_huffman(false)
        .trellis(TrellisConfig::disabled());

    let _ = encoder.encode_rgb(&rgb, width, height).unwrap();

    // Profile loop
    let start = std::time::Instant::now();
    for i in 0..iterations {
        let result = encoder.encode_rgb(&rgb, width, height).unwrap();
        if i == 0 {
            println!("Output size: {} bytes", result.len());
        }
    }
    let elapsed = start.elapsed();

    println!("Total time: {:.2}s", elapsed.as_secs_f64());
    println!(
        "Per iteration: {:.2}ms",
        elapsed.as_secs_f64() * 1000.0 / iterations as f64
    );
}
