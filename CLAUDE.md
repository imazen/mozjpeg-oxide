# mozjpeg-rs Development Guide

## Project Overview

Rust port of Mozilla's mozjpeg JPEG encoder, following the jpegli-rs methodology.

**Scope**: Encoder only + mozjpeg extensions (trellis, progressive scans, deringing)
**API**: Idiomatic Rust (not C-compatible)
**Validation**: FFI dual-execution against C mozjpeg

## Workflow Rules

### Commit Strategy
- **Commit when new tests pass** - After fixing/completing a module
- **Commit when new tests are added** - Even if they're failing (documents expected behavior)
- Write descriptive commit messages explaining what was ported

### Validation Approach
- Validate equivalence **layer by layer**, not just end-to-end
- Use `mozjpeg-sys` from crates.io for FFI validation
- For granular internal function testing, may need to modify C code to export functions

## Key Learnings

### mozjpeg Specifics
1. **Default quant tables**: mozjpeg uses ImageMagick tables (index 3), not JPEG Annex K (index 0)
2. **Quality scaling**: Q50 = 100% scale factor (use tables as-is)
3. **DCT scaling**: Output is scaled by factor of 64 (8 per dimension)
4. **Huffman pseudo-symbol**: Symbol 256 ensures no real symbol gets all-ones code

### Implementation Notes
1. **Huffman tree construction**: Use sentinel values carefully to avoid overflow
   - `FREQ_INITIAL_MAX = 1_000_000_000` for comparison
   - `FREQ_MERGED = 1_000_000_001` for merged nodes
2. **Bitstream stuffing**: 0xFF bytes ALWAYS require 0x00 stuffing in entropy data
3. **Bit buffer**: Use 64-bit buffer, flush when full, pad with 1-bits at end

### Testing Patterns
1. Use `#[cfg(test)]` modules within each source file
2. FFI validation tests in `tests/ffi_validation.rs`
3. Test both positive cases and error conditions

## Architecture

```
mozjpeg/src/
├── lib.rs          # Module exports, public API
├── consts.rs       # Layer 0: Constants, tables, markers
├── types.rs        # Layer 0: ColorSpace, ScanInfo, etc.
├── error.rs        # Error types
├── quant.rs        # Layer 1: Quantization tables
├── huffman.rs      # Layer 1: Huffman table construction
├── dct.rs          # Layer 2: Forward DCT
├── color.rs        # Layer 2: Color conversion
├── bitstream.rs    # Layer 3: Bit-level I/O
├── sample.rs       # Layer 2: Chroma subsampling (TODO)
├── entropy.rs      # Layer 4: Entropy encoding (TODO)
├── trellis.rs      # Layer 4: Trellis quantization (TODO)
├── progressive.rs  # Layer 5: Progressive scans (TODO)
└── encode.rs       # Layer 6: Encoder pipeline (TODO)
```

## Build & Test

```bash
cargo test                    # Run all tests
cargo test huffman           # Run specific module tests
cargo test --test ffi_validation  # Run FFI validation only
```

## Dependencies

- `mozjpeg-sys = "2.2"` (dev) - FFI validation against C mozjpeg
- `bytemuck = "1.14"` - Safe transmutes (for future SIMD)
