//! Compare Rust vs C mozjpeg with trellis disabled

use std::fs;

fn main() {
    let path = "/home/lilith/work/codec-comparison/codec-corpus/kodak/1.png";
    
    let file = fs::File::open(path).unwrap();
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

    // Rust with trellis disabled
    let encoder = mozjpeg::Encoder::new()
        .quality(75)
        .trellis(mozjpeg::TrellisConfig::disabled());
    let rust_no_trellis = encoder.encode_rgb(&rgb_data, width, height).unwrap();

    // Rust with trellis enabled (default)
    let encoder = mozjpeg::Encoder::new().quality(75);
    let rust_with_trellis = encoder.encode_rgb(&rgb_data, width, height).unwrap();

    // C mozjpeg with defaults (trellis disabled by default)
    let c_jpeg = encode_c(&rgb_data, width, height, 75);

    println!("Rust (no trellis):   {} bytes", rust_no_trellis.len());
    println!("Rust (with trellis): {} bytes", rust_with_trellis.len());
    println!("C mozjpeg:           {} bytes", c_jpeg.len());
    println!();
    println!("Ratio (Rust no trellis / C): {:.4}", 
             rust_no_trellis.len() as f64 / c_jpeg.len() as f64);
    println!("Ratio (Rust trellis / C):    {:.4}", 
             rust_with_trellis.len() as f64 / c_jpeg.len() as f64);
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
