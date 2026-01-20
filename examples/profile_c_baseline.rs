//! Profile C mozjpeg baseline encoding
//! Run with: samply record cargo run --release --example profile_c_baseline

use mozjpeg_sys::*;
use std::ptr;

fn main() {
    let width = 2048u32;
    let height = 2048u32;
    let iterations = 100;

    // Create test image with realistic-ish content (same as Rust profile)
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
        "Profiling C mozjpeg baseline encoding: {}x{}, {} iterations",
        width, height, iterations
    );
    println!("Settings: no trellis, no Huffman optimization, 4:2:0 subsampling, baseline mode");

    // Warmup
    let _ = encode_c(&rgb, width, height);

    // Profile loop
    let start = std::time::Instant::now();
    for i in 0..iterations {
        let result = encode_c(&rgb, width, height);
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

fn encode_c(rgb: &[u8], width: u32, height: u32) -> Vec<u8> {
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
        cinfo.in_color_space = J_COLOR_SPACE::JCS_RGB;

        jpeg_set_defaults(&mut cinfo);

        // Force baseline mode (no progressive scans)
        cinfo.num_scans = 0;
        cinfo.scan_info = ptr::null();

        // Use ImageMagick tables (index 3) to match Rust
        jpeg_c_set_int_param(&mut cinfo, JINT_BASE_QUANT_TBL_IDX, 3);
        jpeg_set_quality(&mut cinfo, 85, 1);

        // 4:2:0 subsampling
        (*cinfo.comp_info.offset(0)).h_samp_factor = 2;
        (*cinfo.comp_info.offset(0)).v_samp_factor = 2;
        (*cinfo.comp_info.offset(1)).h_samp_factor = 1;
        (*cinfo.comp_info.offset(1)).v_samp_factor = 1;
        (*cinfo.comp_info.offset(2)).h_samp_factor = 1;
        (*cinfo.comp_info.offset(2)).v_samp_factor = 1;

        // Disable Huffman optimization (use standard tables)
        cinfo.optimize_coding = 0;

        // Disable trellis
        jpeg_c_set_bool_param(&mut cinfo, JBOOLEAN_TRELLIS_QUANT, 0);
        jpeg_c_set_bool_param(&mut cinfo, JBOOLEAN_TRELLIS_QUANT_DC, 0);
        jpeg_c_set_bool_param(&mut cinfo, JBOOLEAN_OVERSHOOT_DERINGING, 0);

        jpeg_start_compress(&mut cinfo, 1);

        let row_stride = width as usize * 3;
        while cinfo.next_scanline < cinfo.image_height {
            let row_ptr = rgb.as_ptr().add(cinfo.next_scanline as usize * row_stride);
            jpeg_write_scanlines(&mut cinfo, &row_ptr as *const *const u8, 1);
        }

        jpeg_finish_compress(&mut cinfo);
        jpeg_destroy_compress(&mut cinfo);

        let result = std::slice::from_raw_parts(outbuffer, outsize as usize).to_vec();
        libc::free(outbuffer as *mut libc::c_void);
        result
    }
}
