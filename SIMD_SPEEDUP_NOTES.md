# SIMD Speedup Investigation Notes

Date: 2026-01-19
Branch: simd-speedup-exploration

## Summary

Successfully implemented a 34% faster AVX2 DCT using 16-bit packed data and vpmaddwd,
directly translating libjpeg-turbo's jfdctint-avx2.asm to Rust intrinsics.

## Key Results

### Performance (DCT, batch of 1000 blocks)

| Implementation | Time (µs) | Throughput (Melem/s) | vs scalar |
|----------------|-----------|----------------------|-----------|
| scalar (multiversion) | 20.8 | 48.0 | baseline |
| transpose_i32x8 | 27.3 | 36.7 | 0.76x |
| avx2_intrinsics (i32) | 16.2 | 61.6 | 1.28x |
| **avx2_i16_vpmaddwd** | **12.1** | **82.3** | **1.72x** |

The i16 vpmaddwd implementation is:
- **34% faster** than the i32 AVX2 intrinsics
- **72% faster** than scalar
- Produces **exact match** with scalar reference (max diff: 0)

### vpmaddwd Optimization

The `vpmaddwd` instruction is key to libjpeg-turbo's AVX2 DCT performance:

```nasm
; vpmaddwd computes: result[i] = a[2i] * b[2i] + a[2i+1] * b[2i+1]
; This maps perfectly to DCT patterns like:
;   data2 = tmp13 * (FIX_0_541 + FIX_0_765) + tmp12 * FIX_0_541
;   data6 = tmp13 * FIX_0_541 + tmp12 * (FIX_0_541 - FIX_1_847)
```

### Bugs Fixed

1. **_mm256_set_epi16 element ordering**: Arguments are in REVERSE order (e15..e0),
   so constant vector pairs were swapped. Fixed pw_f130_f054 and pw_mf078_f117.

2. **Reference scalar used wrong variable naming**: Libjpeg uses non-standard naming
   where `tmp13 = tmp0 - tmp3` (textbooks call this tmp11) and `tmp12 = tmp1 - tmp2`
   (textbooks call this tmp13). The scalar reference in debug examples was using
   textbook naming, causing apparent mismatches.

### libjpeg-turbo Assembly Analysis

libjpeg-turbo's `jfdctint-avx2.asm` uses:
- 16-bit data packed as (row0|row4), (row1|row5), (row2|row6), (row3|row7)
- 16-bit transpose with `vpunpcklwd/hi` (faster than 32-bit)
- `vpmaddwd` for all multiply-accumulate operations
- After transpose: (col1|col0), (col3|col2), (col4|col5), (col6|col7)

This layout enables efficient butterfly operations:
- `tmp1_0 = data1_0 + data6_7` processes two butterflies per instruction

## Files Modified

- `src/dct.rs`: Rewrote `forward_dct_8x8_avx2_i16` with new `dotranspose` and `dodct` helpers
- `benches/dct.rs`: Criterion benchmarks for all DCT variants
- `examples/debug_dct.rs`: Test harness comparing scalar vs AVX2 outputs
- `examples/debug_data26.rs`: Step-by-step data2_6 computation tracing
- `examples/debug_full_dct.rs`: Full DCT with intermediate value dumps
- `examples/trace_transpose.rs`: Transpose verification
- `examples/profile_dct.rs`: Profiling runner for perf analysis

## Next Steps

1. **Consider merging to main branch** - Implementation is complete and tested

2. **Entropy encoding optimization** - DCT is now fast (~12µs/1000 blocks)
   - Entropy encoding may be the new bottleneck
   - Consider Huffman table lookups optimization

3. **Profile full encoder** - Measure DCT impact on end-to-end encoding time

## Benchmark Command

```bash
cd ~/work/mozjpeg-rs-simd-speedup
RUSTFLAGS="-C target-cpu=native" cargo bench --bench dct -- --noplot
```
