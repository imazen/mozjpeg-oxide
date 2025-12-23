//! Huffman entropy encoder for JPEG.
//!
//! This module implements baseline Huffman encoding for DCT coefficients:
//! - DC coefficient encoding with differential coding
//! - AC coefficient encoding with run-length coding
//! - EOB (End of Block) and ZRL (Zero Run Length) symbols
//!
//! Reference: ITU-T T.81 Section F.1.2

use std::io::Write;

use crate::bitstream::BitWriter;
use crate::consts::{DCTSIZE2, JPEG_NATURAL_ORDER};
use crate::huffman::DerivedTable;

/// Maximum coefficient bit size for 8-bit JPEG (10 bits for AC, 11 for DC diff)
const MAX_COEF_BITS: u8 = 10;

/// EOB (End of Block) symbol - encodes as run=0, size=0
const EOB: u8 = 0x00;

/// ZRL (Zero Run Length 16) symbol - encodes 16 consecutive zeros
const ZRL: u8 = 0xF0;

/// Calculate the number of bits needed to represent a value.
///
/// This is the "category" in JPEG terminology:
/// - 0 → 0 bits (value must be 0)
/// - 1 → 1 bit (values -1, 1)
/// - 2 → 2 bits (values -3..-2, 2..3)
/// - etc.
///
/// Uses the efficient bit-scan approach from mozjpeg.
#[inline]
pub fn jpeg_nbits(value: i16) -> u8 {
    if value == 0 {
        return 0;
    }
    let abs_value = value.unsigned_abs();
    16 - abs_value.leading_zeros() as u8
}

/// Calculate nbits for a non-zero value (faster, no zero check).
#[inline]
pub fn jpeg_nbits_nonzero(value: u16) -> u8 {
    16 - value.leading_zeros() as u8
}

/// Entropy encoder state for a single scan.
pub struct EntropyEncoder<'a, W: Write> {
    /// Bitstream writer
    writer: &'a mut BitWriter<W>,
    /// Last DC value for each component (for differential coding)
    last_dc_val: [i16; 4],
}

impl<'a, W: Write> EntropyEncoder<'a, W> {
    /// Create a new entropy encoder.
    pub fn new(writer: &'a mut BitWriter<W>) -> Self {
        Self {
            writer,
            last_dc_val: [0; 4],
        }
    }

    /// Reset DC predictions (called at restart markers).
    pub fn reset_dc(&mut self) {
        self.last_dc_val = [0; 4];
    }

    /// Get the last DC value for a component.
    pub fn last_dc(&self, component: usize) -> i16 {
        self.last_dc_val[component]
    }

    /// Set the last DC value for a component.
    pub fn set_last_dc(&mut self, component: usize, value: i16) {
        self.last_dc_val[component] = value;
    }

    /// Encode a single 8x8 block of DCT coefficients.
    ///
    /// # Arguments
    /// * `block` - 64 quantized DCT coefficients in natural (row-major) order
    /// * `component` - Component index (for DC prediction tracking)
    /// * `dc_table` - Derived Huffman table for DC coefficients
    /// * `ac_table` - Derived Huffman table for AC coefficients
    pub fn encode_block(
        &mut self,
        block: &[i16; DCTSIZE2],
        component: usize,
        dc_table: &DerivedTable,
        ac_table: &DerivedTable,
    ) -> std::io::Result<()> {
        // Encode DC coefficient (differential)
        self.encode_dc(block[0], component, dc_table)?;

        // Encode AC coefficients in zigzag order
        self.encode_ac(block, ac_table)?;

        Ok(())
    }

    /// Encode the DC coefficient using differential coding.
    ///
    /// The DC value is encoded as the difference from the previous DC value
    /// of the same component, followed by the actual bits.
    fn encode_dc(
        &mut self,
        dc: i16,
        component: usize,
        dc_table: &DerivedTable,
    ) -> std::io::Result<()> {
        // Calculate difference from last DC
        let diff = dc.wrapping_sub(self.last_dc_val[component]);
        self.last_dc_val[component] = dc;

        // Handle the value encoding (Section F.1.2.1)
        // For negative values, we encode the complement
        let (nbits, value) = if diff < 0 {
            let nbits = jpeg_nbits(diff);
            // For negative, encode (diff - 1) which gives all-zeros for the sign-extended bits
            let value = (diff as u16).wrapping_sub(1) & ((1u16 << nbits) - 1);
            (nbits, value)
        } else {
            let nbits = jpeg_nbits(diff);
            (nbits, diff as u16)
        };

        // Emit Huffman code for the category (number of bits)
        let (code, size) = dc_table.get_code(nbits);
        if size > 0 {
            self.writer.put_bits(code, size)?;
        }

        // Emit the actual value bits
        if nbits > 0 {
            self.writer.put_bits(value as u32, nbits)?;
        }

        Ok(())
    }

    /// Encode AC coefficients using run-length coding.
    ///
    /// AC coefficients are encoded in zigzag order as (run, size) pairs
    /// where run is the number of preceding zeros and size is the magnitude bits.
    fn encode_ac(&mut self, block: &[i16; DCTSIZE2], ac_table: &DerivedTable) -> std::io::Result<()> {
        let mut run = 0u8; // Run length of zeros

        // Process coefficients 1-63 in zigzag order
        for &zigzag_idx in JPEG_NATURAL_ORDER[1..].iter() {
            let coef = block[zigzag_idx];

            if coef == 0 {
                run += 1;
            } else {
                // Emit ZRL codes for runs of 16+ zeros
                while run >= 16 {
                    let (code, size) = ac_table.get_code(ZRL);
                    self.writer.put_bits(code, size)?;
                    run -= 16;
                }

                // Calculate bits needed and value to encode
                let (nbits, value) = if coef < 0 {
                    let nbits = jpeg_nbits(coef);
                    let value = (coef as u16).wrapping_sub(1) & ((1u16 << nbits) - 1);
                    (nbits, value)
                } else {
                    let nbits = jpeg_nbits(coef);
                    (nbits, coef as u16)
                };

                // Symbol = (run << 4) | nbits
                let symbol = (run << 4) | nbits;
                let (code, size) = ac_table.get_code(symbol);
                self.writer.put_bits(code, size)?;

                // Emit the value bits
                if nbits > 0 {
                    self.writer.put_bits(value as u32, nbits)?;
                }

                run = 0;
            }
        }

        // If there are trailing zeros, emit EOB
        if run > 0 {
            let (code, size) = ac_table.get_code(EOB);
            self.writer.put_bits(code, size)?;
        }

        Ok(())
    }

    /// Flush any remaining bits to the output.
    pub fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

/// Encode a single block without maintaining state (for testing).
pub fn encode_block_standalone<W: Write>(
    writer: &mut BitWriter<W>,
    block: &[i16; DCTSIZE2],
    last_dc: i16,
    dc_table: &DerivedTable,
    ac_table: &DerivedTable,
) -> std::io::Result<i16> {
    let mut encoder = EntropyEncoder::new(writer);
    encoder.set_last_dc(0, last_dc);
    encoder.encode_block(block, 0, dc_table, ac_table)?;
    Ok(encoder.last_dc(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitstream::VecBitWriter;
    use crate::consts::{
        DC_LUMINANCE_BITS, DC_LUMINANCE_VALUES,
        AC_LUMINANCE_BITS, AC_LUMINANCE_VALUES,
    };
    use crate::huffman::HuffTable;

    fn create_dc_luma_table() -> DerivedTable {
        let mut htbl = HuffTable::default();
        htbl.bits.copy_from_slice(&DC_LUMINANCE_BITS);
        for (i, &v) in DC_LUMINANCE_VALUES.iter().enumerate() {
            htbl.huffval[i] = v;
        }
        DerivedTable::from_huff_table(&htbl, true).unwrap()
    }

    fn create_ac_luma_table() -> DerivedTable {
        let mut htbl = HuffTable::default();
        htbl.bits.copy_from_slice(&AC_LUMINANCE_BITS);
        for (i, &v) in AC_LUMINANCE_VALUES.iter().enumerate() {
            htbl.huffval[i] = v;
        }
        DerivedTable::from_huff_table(&htbl, false).unwrap()
    }

    #[test]
    fn test_jpeg_nbits() {
        assert_eq!(jpeg_nbits(0), 0);
        assert_eq!(jpeg_nbits(1), 1);
        assert_eq!(jpeg_nbits(-1), 1);
        assert_eq!(jpeg_nbits(2), 2);
        assert_eq!(jpeg_nbits(-2), 2);
        assert_eq!(jpeg_nbits(3), 2);
        assert_eq!(jpeg_nbits(-3), 2);
        assert_eq!(jpeg_nbits(4), 3);
        assert_eq!(jpeg_nbits(7), 3);
        assert_eq!(jpeg_nbits(8), 4);
        assert_eq!(jpeg_nbits(255), 8);
        assert_eq!(jpeg_nbits(-255), 8);
        assert_eq!(jpeg_nbits(1023), 10);
    }

    #[test]
    fn test_encode_zero_block() {
        let dc_table = create_dc_luma_table();
        let ac_table = create_ac_luma_table();
        let mut writer = VecBitWriter::new_vec();

        let block = [0i16; DCTSIZE2];
        let new_dc = encode_block_standalone(&mut writer, &block, 0, &dc_table, &ac_table).unwrap();
        writer.flush().unwrap();

        assert_eq!(new_dc, 0);
        // Should just have DC=0 (category 0) and EOB
        let bytes = writer.into_bytes();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_encode_dc_only() {
        let dc_table = create_dc_luma_table();
        let ac_table = create_ac_luma_table();
        let mut writer = VecBitWriter::new_vec();

        // Block with only DC = 100, all AC = 0
        let mut block = [0i16; DCTSIZE2];
        block[0] = 100;

        let new_dc = encode_block_standalone(&mut writer, &block, 0, &dc_table, &ac_table).unwrap();
        writer.flush().unwrap();

        assert_eq!(new_dc, 100);
        let bytes = writer.into_bytes();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_encode_dc_differential() {
        let dc_table = create_dc_luma_table();
        let ac_table = create_ac_luma_table();
        let mut writer = VecBitWriter::new_vec();

        // First block: DC = 100
        let mut block = [0i16; DCTSIZE2];
        block[0] = 100;

        let mut encoder = EntropyEncoder::new(&mut writer);
        encoder.encode_block(&block, 0, &dc_table, &ac_table).unwrap();

        // Second block: DC = 105 (diff = 5)
        block[0] = 105;
        encoder.encode_block(&block, 0, &dc_table, &ac_table).unwrap();

        // Third block: DC = 95 (diff = -10)
        block[0] = 95;
        encoder.encode_block(&block, 0, &dc_table, &ac_table).unwrap();

        encoder.flush().unwrap();

        assert_eq!(encoder.last_dc(0), 95);
    }

    #[test]
    fn test_encode_with_ac_coefficients() {
        let dc_table = create_dc_luma_table();
        let ac_table = create_ac_luma_table();
        let mut writer = VecBitWriter::new_vec();

        // Block with some AC coefficients
        let mut block = [0i16; DCTSIZE2];
        block[0] = 50;   // DC
        block[1] = 10;   // AC at position 1
        block[8] = -5;   // AC at position 8 (zigzag position 2)
        block[16] = 3;   // AC at position 16 (zigzag position 3)

        encode_block_standalone(&mut writer, &block, 0, &dc_table, &ac_table).unwrap();
        writer.flush().unwrap();

        let bytes = writer.into_bytes();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_encode_run_length() {
        let dc_table = create_dc_luma_table();
        let ac_table = create_ac_luma_table();
        let mut writer = VecBitWriter::new_vec();

        // Block with zeros followed by a coefficient
        let mut block = [0i16; DCTSIZE2];
        block[0] = 10;   // DC
        block[63] = 1;   // Last AC coefficient (requires 62 zeros before it)

        encode_block_standalone(&mut writer, &block, 0, &dc_table, &ac_table).unwrap();
        writer.flush().unwrap();

        let bytes = writer.into_bytes();
        // Should contain ZRL codes (0xF0) for runs of 16 zeros
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_encode_negative_values() {
        let dc_table = create_dc_luma_table();
        let ac_table = create_ac_luma_table();
        let mut writer = VecBitWriter::new_vec();

        // Block with negative coefficients
        let mut block = [0i16; DCTSIZE2];
        block[0] = -50;   // DC
        block[1] = -10;   // AC
        block[8] = -1;    // AC

        encode_block_standalone(&mut writer, &block, 0, &dc_table, &ac_table).unwrap();
        writer.flush().unwrap();

        let bytes = writer.into_bytes();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_reset_dc() {
        let dc_table = create_dc_luma_table();
        let ac_table = create_ac_luma_table();
        let mut writer = VecBitWriter::new_vec();

        let mut encoder = EntropyEncoder::new(&mut writer);

        // Set DC values
        let mut block = [0i16; DCTSIZE2];
        block[0] = 100;
        encoder.encode_block(&block, 0, &dc_table, &ac_table).unwrap();
        assert_eq!(encoder.last_dc(0), 100);

        // Reset
        encoder.reset_dc();
        assert_eq!(encoder.last_dc(0), 0);
    }

    #[test]
    fn test_multiple_components() {
        let dc_table = create_dc_luma_table();
        let ac_table = create_ac_luma_table();
        let mut writer = VecBitWriter::new_vec();

        let mut encoder = EntropyEncoder::new(&mut writer);

        // Encode Y component
        let mut block = [0i16; DCTSIZE2];
        block[0] = 100;
        encoder.encode_block(&block, 0, &dc_table, &ac_table).unwrap();

        // Encode Cb component
        block[0] = 128;
        encoder.encode_block(&block, 1, &dc_table, &ac_table).unwrap();

        // Encode Cr component
        block[0] = 130;
        encoder.encode_block(&block, 2, &dc_table, &ac_table).unwrap();

        encoder.flush().unwrap();

        // Each component should have its own DC prediction
        assert_eq!(encoder.last_dc(0), 100);
        assert_eq!(encoder.last_dc(1), 128);
        assert_eq!(encoder.last_dc(2), 130);
    }
}
