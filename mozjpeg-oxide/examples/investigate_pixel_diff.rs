//! Investigate pixel differences between Rust and C mozjpeg encoders.
//!
//! This example examines where and why decoded pixels differ between Rust
//! and C implementations with identical settings.

use mozjpeg_oxide::{Encoder, Subsampling, TrellisConfig};

fn main() {
    println!("=== Rust vs C Pixel Difference Investigation ===\n");

    // Test multiple sizes and modes
    let test_cases = [
        (16, 16, Subsampling::S444, "16x16 4:4:4"),
        (16, 16, Subsampling::S420, "16x16 4:2:0"),
        (17, 17, Subsampling::S420, "17x17 4:2:0 (non-MCU)"),
        (32, 32, Subsampling::S420, "32x32 4:2:0"),
        (64, 64, Subsampling::S420, "64x64 4:2:0"),
    ];

    for (width, height, subsampling, name) in test_cases {
        investigate_size(width, height, subsampling, name);
    }
}

fn investigate_size(width: u32, height: u32, subsampling: Subsampling, name: &str) {
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

    // Encode with Rust - baseline, no optimizations
    let rust_jpeg = Encoder::new()
        .quality(85)
        .subsampling(subsampling)
        .progressive(false)
        .optimize_huffman(false)
        .trellis(TrellisConfig::disabled())
        .encode_rgb(&rgb, width, height)
        .unwrap();

    // Encode with C
    let c_jpeg = encode_c(&rgb, width, height, 85, subsampling, false);

    // Decode both
    let rust_dec = decode(&rust_jpeg);
    let c_dec = decode(&c_jpeg);

    // Analyze differences
    let (max_diff, avg_diff, diff_count, diff_positions) =
        analyze_differences(&rust_dec, &c_dec, w, h);

    println!("{}:", name);
    println!("  File sizes: Rust={} C={} (diff={})",
             rust_jpeg.len(), c_jpeg.len(),
             rust_jpeg.len() as i64 - c_jpeg.len() as i64);
    println!("  Max pixel diff: {}", max_diff);
    println!("  Avg pixel diff: {:.4}", avg_diff);
    println!("  Pixels differing: {} / {} ({:.2}%)",
             diff_count, rust_dec.len(),
             diff_count as f64 * 100.0 / rust_dec.len() as f64);

    if !diff_positions.is_empty() {
        println!("  First 10 diff positions (x,y,channel):");
        for (i, (pos, rust_val, c_val)) in diff_positions.iter().take(10).enumerate() {
            let pixel = pos / 3;
            let channel = pos % 3;
            let x = pixel % w;
            let y = pixel / w;
            let ch_name = ["R", "G", "B"][channel];
            println!("    {}: ({},{}) {}: Rust={} C={} diff={}",
                     i, x, y, ch_name, rust_val, c_val,
                     (*rust_val as i16 - *c_val as i16).abs());
        }
    }
    println!();
}

fn analyze_differences(rust: &[u8], c: &[u8], width: usize, _height: usize)
    -> (u8, f64, usize, Vec<(usize, u8, u8)>)
{
    let mut max_diff = 0u8;
    let mut total_diff = 0u64;
    let mut diff_count = 0usize;
    let mut diff_positions = Vec::new();

    for (i, (&r, &c_val)) in rust.iter().zip(c.iter()).enumerate() {
        let diff = (r as i16 - c_val as i16).unsigned_abs() as u8;
        if diff > 0 {
            diff_count += 1;
            if diff_positions.len() < 100 {
                diff_positions.push((i, r, c_val));
            }
        }
        if diff > max_diff {
            max_diff = diff;
        }
        total_diff += diff as u64;
    }

    let avg_diff = total_diff as f64 / rust.len() as f64;
    (max_diff, avg_diff, diff_count, diff_positions)
}

fn decode(data: &[u8]) -> Vec<u8> {
    jpeg_decoder::Decoder::new(std::io::Cursor::new(data))
        .decode()
        .expect("decode failed")
}

fn encode_c(rgb: &[u8], width: u32, height: u32, quality: i32, subsampling: Subsampling, progressive: bool) -> Vec<u8> {
    use mozjpeg_sys::*;
    use std::ptr;

    unsafe {
        let mut cinfo: jpeg_compress_struct = std::mem::zeroed();
        let mut jerr: jpeg_error_mgr = std::mem::zeroed();

        cinfo.common.err = jpeg_std_error(&mut jerr);
        jpeg_CreateCompress(
            &mut cinfo,
            JPEG_LIB_VERSION as i32,
            std::mem::size_of::<jpeg_compress_struct>(),
        );

        let mut outbuffer: *mut u8 = ptr::null_mut();
        let mut outsize: u64 = 0;
        jpeg_mem_dest(&mut cinfo, &mut outbuffer, &mut outsize);

        cinfo.image_width = width;
        cinfo.image_height = height;
        cinfo.input_components = 3;
        cinfo.in_color_space = JCS_RGB;

        jpeg_set_defaults(&mut cinfo);
        jpeg_set_quality(&mut cinfo, quality, 1);

        // CRITICAL: Disable progressive mode by clearing scan_info
        // (jpeg_set_defaults enables it via JCP_MAX_COMPRESSION profile)
        cinfo.scan_info = ptr::null();
        cinfo.num_scans = 0;

        // Set subsampling
        let (h_samp, v_samp) = match subsampling {
            Subsampling::S444 => (1, 1),
            Subsampling::S422 => (2, 1),
            Subsampling::S420 => (2, 2),
            Subsampling::S440 => (1, 2),
            Subsampling::Gray => (1, 1),
        };

        (*cinfo.comp_info.offset(0)).h_samp_factor = h_samp;
        (*cinfo.comp_info.offset(0)).v_samp_factor = v_samp;
        (*cinfo.comp_info.offset(1)).h_samp_factor = 1;
        (*cinfo.comp_info.offset(1)).v_samp_factor = 1;
        (*cinfo.comp_info.offset(2)).h_samp_factor = 1;
        (*cinfo.comp_info.offset(2)).v_samp_factor = 1;

        // Disable Huffman optimization (to match Rust)
        cinfo.optimize_coding = 0;

        if progressive {
            jpeg_simple_progression(&mut cinfo);
        }

        jpeg_start_compress(&mut cinfo, 1);

        let row_stride = (width * 3) as usize;
        let mut row_pointer: [*const u8; 1] = [ptr::null()];

        while cinfo.next_scanline < cinfo.image_height {
            let offset = cinfo.next_scanline as usize * row_stride;
            row_pointer[0] = rgb.as_ptr().add(offset);
            jpeg_write_scanlines(&mut cinfo, row_pointer.as_ptr(), 1);
        }

        jpeg_finish_compress(&mut cinfo);
        jpeg_destroy_compress(&mut cinfo);

        let result = std::slice::from_raw_parts(outbuffer, outsize as usize).to_vec();
        libc::free(outbuffer as *mut libc::c_void);

        result
    }
}
