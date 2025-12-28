# mozjpeg-oxide

Pure Rust JPEG encoder based on Mozilla's [mozjpeg](https://github.com/mozilla/mozjpeg), featuring trellis quantization for optimal compression.

[![Crates.io](https://img.shields.io/crates/v/mozjpeg-oxide.svg)](https://crates.io/crates/mozjpeg-oxide)
[![Documentation](https://docs.rs/mozjpeg-oxide/badge.svg)](https://docs.rs/mozjpeg-oxide)
[![CI](https://github.com/imazen/mozjpeg-oxide/actions/workflows/ci.yml/badge.svg)](https://github.com/imazen/mozjpeg-oxide/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/imazen/mozjpeg-oxide/graph/badge.svg)](https://codecov.io/gh/imazen/mozjpeg-oxide)
[![License](https://img.shields.io/crates/l/mozjpeg-oxide.svg)](LICENSE)

## Why mozjpeg-oxide?

| | mozjpeg-oxide | C mozjpeg | libjpeg-turbo |
|--|---------------|-----------|---------------|
| **Language** | Pure Rust | C | C/asm |
| **Memory safety** | Compile-time guaranteed | Manual | Manual |
| **Trellis quantization** | Yes | Yes | No |
| **Build complexity** | `cargo add` | cmake + nasm + C toolchain | cmake + nasm |

**Choose mozjpeg-oxide when you want:**
- Memory-safe JPEG encoding without C dependencies
- Smaller files than libjpeg-turbo (trellis quantization)
- Simple integration via Cargo

**Choose C mozjpeg when you need:**
- Smallest possible files at high quality (Q85+)
- Maximum baseline encoding speed (SIMD-optimized entropy coding)
- Established C ABI for FFI

## Compression Results vs C mozjpeg

Tested on full [Kodak](http://r0k.us/graphics/kodak/) corpus (24 images).
Settings: progressive mode, trellis quantization, optimized Huffman, 4:2:0 subsampling.

| Quality | Rust vs C | Notes |
|---------|-----------|-------|
| Q50 | **-1.35%** | Rust produces smaller files |
| Q75 | **+0.14%** | Near-identical |
| Q85 | +1.39% | Slight gap |
| Q90 | +2.58% | Gap grows |
| Q95 | +3.61% | |
| Q97 | +4.87% | |

**Summary**: At Q50-Q75, Rust matches or beats C mozjpeg. At Q85+, files are 1-5% larger due to differences in progressive scan structure (see [Differences from C mozjpeg](#differences-from-c-mozjpeg)).

Visual quality (SSIMULACRA2, Butteraugli) is virtually identical at all quality levels.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="benchmark/pareto_ssimulacra2.svg">
  <source media="(prefers-color-scheme: light)" srcset="benchmark/pareto_ssimulacra2.svg">
  <img alt="SSIMULACRA2 vs BPP Pareto curve" src="benchmark/pareto_ssimulacra2.svg">
</picture>

## Usage

```rust
use mozjpeg_oxide::{Encoder, Subsampling};

// Default: trellis quantization + Huffman optimization
let jpeg = Encoder::new()
    .quality(85)
    .encode_rgb(&pixels, width, height)?;

// Maximum compression: progressive + trellis + deringing
let jpeg = Encoder::max_compression()
    .quality(85)
    .encode_rgb(&pixels, width, height)?;

// Fastest: no optimizations (libjpeg-turbo compatible output)
let jpeg = Encoder::fastest()
    .quality(85)
    .encode_rgb(&pixels, width, height)?;

// Custom configuration
let jpeg = Encoder::new()
    .quality(75)
    .progressive(true)
    .subsampling(Subsampling::S420)
    .optimize_huffman(true)
    .encode_rgb(&pixels, width, height)?;
```

## Features

- **Trellis quantization** - Rate-distortion optimized coefficient selection (AC + DC)
- **Progressive JPEG** - Multi-scan encoding with spectral selection
- **Huffman optimization** - 2-pass encoding for optimal entropy coding
- **Overshoot deringing** - Reduces ringing artifacts at sharp edges
- **Chroma subsampling** - 4:4:4, 4:2:2, 4:2:0 modes
- **Safe Rust** - `#![deny(unsafe_code)]` with exceptions only for SIMD intrinsics

## Performance

Benchmarked on 512x768 image, 20 iterations, release mode:

| Configuration | Rust | C mozjpeg | Ratio |
|---------------|------|-----------|-------|
| Baseline (huffman opt) | 7.1 ms | 26.8 ms | **3.8x faster** |
| Trellis (AC + DC) | 19.7 ms | 25.3 ms | **1.3x faster** |
| Progressive + trellis | 20.0 ms | - | - |

**Note**: C mozjpeg's baseline encoding is typically faster with its hand-optimized SIMD entropy coding. The benchmark numbers above reflect mozjpeg-sys from crates.io which may not have all optimizations enabled.

### SIMD Support

mozjpeg-oxide uses `multiversion` for automatic vectorization by default. Optional hand-written SIMD intrinsics are available:

```toml
[dependencies]
mozjpeg-oxide = { version = "0.2", features = ["simd-intrinsics"] }
```

In benchmarks, the difference is minimal (~2%) as `multiversion` autovectorization works well for DCT and color conversion.

## Differences from C mozjpeg

mozjpeg-oxide aims for compatibility with C mozjpeg but has some differences:

| Feature | mozjpeg-oxide | C mozjpeg |
|---------|---------------|-----------|
| **Progressive scan script** | Simple 4-scan | 9-scan with successive approximation |
| **optimize_scans** | Disabled by default | Enabled in JCP_MAX_COMPRESSION |
| **Trellis EOB optimization** | Not implemented | Available (rarely used) |
| **Arithmetic coding** | Not implemented | Available (rarely used) |
| **Grayscale progressive** | Not implemented | Available |

### Why the file size gap at high quality?

C mozjpeg's `jpeg_simple_progression()` uses a 9-scan successive approximation script that splits coefficient bits into coarse and fine layers. This provides better Huffman coding efficiency, especially at high quality settings where more bits per coefficient are used.

mozjpeg-oxide uses a simpler 4-scan script (DC + full AC for each component). This produces identical visual quality but slightly larger files at Q85+.

### Matching C mozjpeg output exactly

For exact byte-identical output to C mozjpeg, you would need to:
1. Use baseline (non-progressive) mode
2. Match all encoder settings exactly
3. Use the same quantization tables (ImageMagick tables, index 3)

The FFI comparison tests in `tests/ffi_comparison.rs` verify component-level parity.

## Development

### Running CI Locally

```bash
# Format check
cargo fmt --all -- --check

# Clippy lints
cargo clippy --workspace --all-targets -- -D warnings

# Build
cargo build --workspace

# Unit tests
cargo test --lib

# Codec comparison tests
cargo test --test codec_comparison

# FFI validation tests (requires mozjpeg-sys from crates.io)
cargo test --test ffi_validation
```

### Reproduce Benchmarks

```bash
# Fetch test corpus (Kodak images, ~15MB)
./scripts/fetch-corpus.sh

# Run full corpus comparison
cargo run --release --example full_corpus_test

# Run pareto benchmark
cargo run --release --example pareto_benchmark
```

### Test Coverage

```bash
# Install cargo-llvm-cov
cargo install cargo-llvm-cov

# Generate coverage report
cargo llvm-cov --lib --html

# Open report
open target/llvm-cov/html/index.html
```

## License

BSD-3-Clause - Same license as the original mozjpeg.

## Acknowledgments

Based on Mozilla's [mozjpeg](https://github.com/mozilla/mozjpeg), which builds on libjpeg-turbo and the Independent JPEG Group's libjpeg.

## AI-Generated Code Notice

This crate was developed with significant assistance from Claude (Anthropic). While the code has been tested against the C mozjpeg reference implementation and passes 248 tests including FFI validation, **not all code has been manually reviewed or human-audited**.

Before using in production:
- Review critical code paths for your use case
- Run your own validation against expected outputs
- Consider the encoder's test suite coverage for your specific requirements

The FFI comparison tests in `tests/ffi_comparison.rs` and `tests/ffi_validation.rs` provide confidence in correctness by comparing outputs against C mozjpeg.
