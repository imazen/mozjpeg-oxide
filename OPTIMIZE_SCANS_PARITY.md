# optimize_scans Parity with C mozjpeg

## Goal
Match C mozjpeg's `optimize_scans` output exactly (0% file size difference).

## Current Status
- **Gap:** 2-4% larger files with `optimize_scans` enabled
- **Root Cause:** Trial encoding for refinement scans produces garbage sizes

## Problem Analysis

### How C mozjpeg implements optimize_scans (jcmaster.c)

1. **Multi-pass encoding with buffers:**
   - Each candidate scan is encoded in sequence
   - Encoded bytes are stored in `master->scan_buffer[scan_number]`
   - Scan sizes stored in `master->scan_size[scan_number]`

2. **Selection during encoding:**
   - `select_scans()` is called after each scan completes
   - Compares costs and updates `best_Al_luma`, `best_Al_chroma`, etc.
   - Can skip remaining scans via early termination

3. **Buffer copying for output:**
   - After all scans encoded, `copy_buffer()` copies selected scans to output
   - Final output is concatenation of pre-encoded scan data

### How Rust currently implements optimize_scans

1. **Independent trial encoding:**
   - Each candidate scan encoded separately to get size
   - No state maintained between scans
   - Refinement scans (Ah > 0) produce garbage because they lack context

2. **Selection after all trials:**
   - ScanSelector processes all 64 sizes
   - Algorithm matches C, but inputs are wrong for refinement scans

3. **Re-encoding for output:**
   - Selected configuration is re-encoded from scratch
   - Different from C which uses pre-encoded bytes

### Evidence of the Problem

```
Scan sizes: [2466, 2128, 11, 22728, 657, 5, 21922, 24, 2, 21515, ...]
Indices:      0     1     2      3    4   5      6   7  8      9

Scan 3 (Y refine Ah=1, Al=0): 22,728 bytes - WRONG (should be ~200-500)
Scan 6 (Y refine Ah=2, Al=1): 21,922 bytes - WRONG
Scan 9 (Y refine Ah=3, Al=2): 21,515 bytes - WRONG

Al=0 cost: 2128 + 11 = 2,139 (correct)
Al=1 cost: 657 + 5 + 22728 = 23,390 (garbage due to scan 3)

Result: Optimizer always picks Al=0, never uses successive approximation
```

## Implementation Plan

### Phase 1: Sequential Trial Encoding with State
- [ ] Create `ScanTrialEncoder` struct that maintains progressive encoder state
- [ ] Encode scans in order (0, 1, 2, 3, ...) with shared state
- [ ] Store encoded bytes in Vec<Vec<u8>> for each scan
- [ ] Extract sizes for ScanSelector

### Phase 2: Proper Refinement Scan Encoding
- [ ] Track which coefficients have been "first-scanned"
- [ ] Refinement scans only encode the refining bits
- [ ] Verify refinement scan sizes are reasonable (should be small)

### Phase 3: Buffer-Based Output Assembly
- [ ] After selection, copy pre-encoded buffers to output
- [ ] No re-encoding needed - use exact trial-encoded bytes
- [ ] This matches C mozjpeg's `copy_buffer()` approach

### Phase 4: Verification
- [ ] Compare scan-by-scan sizes with C mozjpeg
- [ ] Verify selection decisions match (Al levels, frequency splits)
- [ ] Compare final file sizes at all quality levels
- [ ] Target: 0% difference

## Key C mozjpeg Code References

### jcmaster.c - select_scans() (lines 773-962)
- Called after each scan completes
- Updates best_Al_luma, best_Al_chroma based on costs
- Uses early termination to skip unnecessary scans

### jcmaster.c - copy_buffer() (lines ~902-956)
- Copies selected scan buffers to final output
- Order: DC, chroma DC, luma freq split, luma refinements, chroma, chroma refinements

### jcparam.c - jpeg_search_progression() (lines 733-852)
- Generates 64 candidate scans for YCbCr
- Layout matches our generate_search_scans() exactly (verified)

## Progress Log

### 2024-12-28 Session 1
- [x] Identified scan layout mismatch (67 vs 64 scans)
- [x] Fixed scan layout to match C exactly
- [x] Discovered refinement scans produce garbage sizes
- [x] Identified root cause: independent trial encoding
- [x] Documented architectural difference between Rust and C

### Next Steps
1. Implement ScanTrialEncoder with sequential encoding
2. Store scan buffers during trial encoding
3. Use buffer copying for output assembly
4. Verify 0% difference achieved

## Test Commands

```bash
# Run benchmark comparison
cargo test --release --test benchmark_runner -- --nocapture

# Check specific quality levels
# Look for max_compression mode results

# Enable debug output in scan_optimize.rs
# const DEBUG_SCAN_OPT: bool = true;
```
