# mozjpeg-oxide

Pure Rust JPEG encoder based on Mozilla's [mozjpeg](https://github.com/mozilla/mozjpeg), featuring trellis quantization for optimal compression.

[![Crates.io](https://img.shields.io/crates/v/mozjpeg-oxide.svg)](https://crates.io/crates/mozjpeg-oxide)
[![Documentation](https://docs.rs/mozjpeg-oxide/badge.svg)](https://docs.rs/mozjpeg-oxide)
[![License](https://img.shields.io/crates/l/mozjpeg-oxide.svg)](LICENSE)

## Features

- **Trellis quantization** - Rate-distortion optimized coefficient selection for smaller files
- **Progressive JPEG** - Multi-scan encoding with DC-first, AC-band progression
- **Huffman optimization** - 2-pass encoding for optimal entropy coding
- **Chroma subsampling** - 4:4:4, 4:2:2, 4:2:0 modes
- **Quality presets** - `max_compression()` and `fastest()` for common use cases

## Usage

```rust
use mozjpeg_oxide::{Encoder, Subsampling};

// Default encoding (trellis + Huffman optimization enabled)
let encoder = Encoder::new().quality(85);
let jpeg_data = encoder.encode_rgb(&pixels, width, height)?;

// Maximum compression (progressive + trellis + optimized Huffman)
let encoder = Encoder::max_compression();
let jpeg_data = encoder.encode_rgb(&pixels, width, height)?;

// Fastest encoding (no optimizations)
let encoder = Encoder::fastest().quality(85);
let jpeg_data = encoder.encode_rgb(&pixels, width, height)?;

// Custom configuration
let encoder = Encoder::new()
    .quality(75)
    .progressive(true)
    .subsampling(Subsampling::S420)
    .optimize_huffman(true);
let jpeg_data = encoder.encode_rgb(&pixels, width, height)?;
```

## Performance vs C mozjpeg

Tested on 512x512 images in release mode:

| Configuration | Rust | C mozjpeg | Notes |
|---------------|------|-----------|-------|
| Baseline (no opts) | 7.5x slower | baseline | C has SIMD DCT |
| Trellis AC | 0.87x (faster) | baseline | |
| Max compression | 0.60x (faster) | baseline | |

File sizes are within 1% of C mozjpeg at equivalent quality settings.

## Compression Quality

Tested against the Kodak image corpus at Q75:

- Average file size: within 1% of C mozjpeg
- Average PSNR difference: 0.06 dB (imperceptible)

## License

BSD-3-Clause - Same license as the original mozjpeg.

## Acknowledgments

Based on Mozilla's [mozjpeg](https://github.com/mozilla/mozjpeg), which is itself based on libjpeg-turbo and the Independent JPEG Group's libjpeg.
