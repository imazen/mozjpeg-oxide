//! Compare Rust vs C mozjpeg with trellis disabled
//!
//! Run `./scripts/fetch-corpus.sh` first to download test images,
//! or uses bundled test image if corpus not available.

use mozjpeg_oxide::corpus::{bundled_test_image, kodak_dir};
use std::fs;
use std::path::PathBuf;

fn main() {
    // Try kodak corpus first, fall back to bundled test image
    let path: PathBuf = kodak_dir()
        .map(|d| d.join("1.png"))
        .filter(|p| p.exists())
        .or_else(|| bundled_test_image("1.png"))
        .expect("No test image found. Run ./scripts/fetch-corpus.sh or check bundled images.");

    let file = fs::File::open(&path).unwrap();
    let decoder = png::Decoder::new(file);
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    let bytes = &buf[..info.buffer_size()];

    let width = info.width;
    let height = info.height;

    let rgb_data: Vec<u8> = match info.color_type {
        png::ColorType::Rgb => bytes.to_vec(),
        png::ColorType::Rgba => {
            bytes.chunks(4).flat_map(|c| [c[0], c[1], c[2]]).collect()
        }
        _ => panic!("Unsupported"),
    };

    // Rust with max_compression (progressive + trellis)
    let encoder = mozjpeg_oxide::Encoder::max_compression().quality(75);
    let rust_progressive = encoder.encode_rgb(&rgb_data, width, height).unwrap();

    // Rust baseline with trellis enabled (default)
    let encoder = mozjpeg_oxide::Encoder::new().quality(75);
    let rust_baseline = encoder.encode_rgb(&rgb_data, width, height).unwrap();

    // C mozjpeg with defaults (JCP_MAX_COMPRESSION = progressive + trellis)
    let c_jpeg = encode_c(&rgb_data, width, height, 75);

    println!("Rust (progressive+trellis): {} bytes", rust_progressive.len());
    println!("Rust (baseline+trellis):    {} bytes", rust_baseline.len());
    println!("C mozjpeg:                  {} bytes", c_jpeg.len());
    println!();
    println!("Ratio (Rust progressive / C): {:.4}",
             rust_progressive.len() as f64 / c_jpeg.len() as f64);
    println!("Ratio (Rust baseline / C):    {:.4}",
             rust_baseline.len() as f64 / c_jpeg.len() as f64);

    // Save files for analysis (cross-platform temp directory)
    let temp_dir = std::env::temp_dir();
    let rust_path = temp_dir.join("rust_progressive.jpg");
    let c_path = temp_dir.join("c_mozjpeg.jpg");
    fs::write(&rust_path, &rust_progressive).unwrap();
    fs::write(&c_path, &c_jpeg).unwrap();
    println!();
    println!("Files saved to {:?} and {:?}", rust_path, c_path);

    // Count SOS markers (number of scans)
    let rust_scans = rust_progressive.windows(2).filter(|w| *w == [0xFF, 0xDA]).count();
    let c_scans = c_jpeg.windows(2).filter(|w| *w == [0xFF, 0xDA]).count();
    println!();
    println!("Rust scan count: {}", rust_scans);
    println!("C scan count:    {}", c_scans);

    // Decode all and compare PSNR
    let rust_prog_decoded = decode_jpeg(&rust_progressive);
    let rust_base_decoded = decode_jpeg(&rust_baseline);
    let c_decoded = decode_jpeg(&c_jpeg);

    let psnr_rust_prog = calculate_psnr(&rgb_data, &rust_prog_decoded);
    let psnr_rust_base = calculate_psnr(&rgb_data, &rust_base_decoded);
    let psnr_c = calculate_psnr(&rgb_data, &c_decoded);
    println!();
    println!("Rust progressive PSNR: {:.2} dB", psnr_rust_prog);
    println!("Rust baseline PSNR:    {:.2} dB", psnr_rust_base);
    println!("C PSNR:                {:.2} dB", psnr_c);

    // Also test a small 16x16 image
    println!();
    println!("=== Testing small 16x16 image ===");
    let small_w = 16u32;
    let small_h = 16u32;
    let mut small_rgb = vec![128u8; (small_w * small_h * 3) as usize];
    for y in 0..small_h {
        for x in 0..small_w {
            let i = (y * small_w + x) as usize;
            small_rgb[i*3] = ((x * 16) % 256) as u8;
            small_rgb[i*3+1] = ((y * 16) % 256) as u8;
            small_rgb[i*3+2] = 128;
        }
    }

    let small_prog = mozjpeg_oxide::Encoder::max_compression()
        .quality(85)
        .encode_rgb(&small_rgb, small_w, small_h).unwrap();
    let small_base = mozjpeg_oxide::Encoder::new()
        .quality(85)
        .encode_rgb(&small_rgb, small_w, small_h).unwrap();

    let small_prog_dec = decode_jpeg(&small_prog);
    let small_base_dec = decode_jpeg(&small_base);

    let small_prog_psnr = calculate_psnr(&small_rgb, &small_prog_dec);
    let small_base_psnr = calculate_psnr(&small_rgb, &small_base_dec);

    println!("Small image progressive PSNR: {:.2} dB", small_prog_psnr);
    println!("Small image baseline PSNR:    {:.2} dB", small_base_psnr);
}

fn decode_jpeg(data: &[u8]) -> Vec<u8> {
    let mut decoder = jpeg_decoder::Decoder::new(std::io::Cursor::new(data));
    decoder.decode().unwrap()
}

fn calculate_psnr(original: &[u8], decoded: &[u8]) -> f64 {
    let mse: f64 = original.iter()
        .zip(decoded.iter())
        .map(|(&a, &b)| {
            let diff = a as f64 - b as f64;
            diff * diff
        })
        .sum::<f64>() / original.len() as f64;

    if mse == 0.0 {
        return f64::INFINITY;
    }
    10.0 * (255.0_f64 * 255.0 / mse).log10()
}

fn encode_c(rgb: &[u8], width: u32, height: u32, quality: i32) -> Vec<u8> {
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
        cinfo.optimize_coding = 1;

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
