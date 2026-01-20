# mozjpeg-rs Performance Benchmark

## Quick Start

```bash
# Build and run performance benchmark
docker build -f benchmark/Dockerfile.perf -t mozjpeg-rs-perf .
docker run --rm mozjpeg-rs-perf
```

## What It Measures

Uses **Criterion** for statistically rigorous benchmarking. Compares:

- **Rust vs C mozjpeg**: Baseline and trellis configurations
- **Encoder configurations**: Fastest, huffman, trellis AC, trellis AC+DC, progressive, max compression
- **Image sizes**: 256×256 to 2048×2048
- **Subsampling modes**: 4:4:4, 4:2:2, 4:2:0
- **DCT implementations**: Scalar vs SIMD variants
- **Optimization flags**: Incremental cost of each feature

## Options

```bash
# Run specific benchmark group
docker run --rm mozjpeg-rs-perf rust_vs_c
docker run --rm mozjpeg-rs-perf image_sizes
docker run --rm mozjpeg-rs-perf dct

# Quick mode (fewer samples, faster)
docker run --rm mozjpeg-rs-perf rust_vs_c --quick

# Filter to specific benchmark
docker run --rm mozjpeg-rs-perf "rust_vs_c/rust/trellis"

# All benchmarks, full precision
docker run --rm mozjpeg-rs-perf
```

## Test Images

Uses **deterministically generated synthetic images** - no external downloads required.

Images contain gradients and noise patterns that exercise the encoder comprehensively.
Generated from a deterministic algorithm seeded by image dimensions.

## Reproducibility

### What's Pinned

| Component | Pinning Method |
|-----------|----------------|
| Rust deps | `Cargo.lock` (exact versions) |
| C mozjpeg | `mozjpeg-sys` compiles from vendored source (bundled in crate) |
| Test images | Deterministic generation from code |
| Rust toolchain | `rust:1.92-bookworm` (stable, matches MSRV 1.89+) |

**Note:** `mozjpeg-sys` always builds mozjpeg from C source bundled within the crate
(statically linked). It never uses system-installed libjpeg or mozjpeg libraries.

### For Maximum Reproducibility

Use image digests instead of tags:

```dockerfile
# Get current digest
docker pull rustlang/rust:nightly-bookworm
docker inspect rustlang/rust:nightly-bookworm --format='{{index .RepoDigests 0}}'

# Use in Dockerfile
FROM rustlang/rust:nightly-bookworm@sha256:abc123...
```

## Benchmark Methodology (Criterion)

Criterion provides:

- **Statistical rigor**: Confidence intervals, outlier detection
- **Warmup**: Automatic warmup before measurement
- **Iteration tuning**: Adjusts sample count for stable results
- **Regression detection**: Compares against previous runs
- **Throughput reporting**: Pixels/second metrics

## Running Locally (Without Docker)

```bash
# Run all benchmarks
cargo bench

# Run specific group
cargo bench --bench encode -- rust_vs_c

# Quick mode
cargo bench --bench encode -- --quick
```
