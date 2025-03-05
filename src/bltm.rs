//! Binary Lower Triangular Matrix (BLTM) operations for steganography

use bitvec::prelude::*;

/// Simple 3x3 Binary Lower Triangular Matrix implementation
#[derive(Debug, Clone)]
pub struct BLTM3x3;

impl BLTM3x3 {
    /// Create a new 3x3 BLTM
    pub fn new() -> Self {
        BLTM3x3
    }

    /// Get the columns of the 3x3 BLTM
    /// Returns C1, C2, C3 columns from right to left
    pub fn columns(&self) -> [BitVec<u8, Msb0>; 3] {
        // Create properly formatted bit vectors
        let c1 = bitvec![u8, Msb0; 0, 0, 1];
        let c2 = bitvec![u8, Msb0; 0, 1, 1];
        let c3 = bitvec![u8, Msb0; 1, 1, 1];

        [c1, c2, c3]
    }

    /// Convert binary vector to u8
    pub fn bits_to_u8(bits: &BitSlice<u8, Msb0>) -> u8 {
        let mut result = 0u8;
        for bit in bits {
            result = (result << 1) | (*bit as u8);
        }
        result
    }

    /// Convert u8 to bitvec (3 bits)
    pub fn u8_to_bits(value: u8) -> BitVec<u8, Msb0> {
        let mut bv = BitVec::<u8, Msb0>::new();
        bv.push((value & 4) != 0);
        bv.push((value & 2) != 0);
        bv.push((value & 1) != 0);
        bv
    }

    /// Simple lookup function to find Vn given delta
    pub fn lookup_vn(&self, delta: &BitSlice<u8, Msb0>) -> BitVec<u8, Msb0> {
        // Convert input to u8 for easier lookup
        let delta_val = Self::bits_to_u8(delta);

        // Hardcoded lookup table based on the provided example
        match delta_val {
            0 => {
                let mut bv = BitVec::new();
                bv.push(false);
                bv.push(false);
                bv.push(false);
                bv
            }
            1 => {
                let mut bv = BitVec::new();
                bv.push(false);
                bv.push(false);
                bv.push(true);
                bv
            }
            2 => {
                let mut bv = BitVec::new();
                bv.push(false);
                bv.push(true);
                bv.push(true);
                bv
            }
            3 => {
                let mut bv = BitVec::new();
                bv.push(false);
                bv.push(true);
                bv.push(false);
                bv
            }
            4 => {
                let mut bv = BitVec::new();
                bv.push(true);
                bv.push(true);
                bv.push(false);
                bv
            }
            5 => {
                let mut bv = BitVec::new();
                bv.push(true);
                bv.push(true);
                bv.push(true);
                bv
            }
            6 => {
                let mut bv = BitVec::new();
                bv.push(true);
                bv.push(false);
                bv.push(true);
                bv
            }
            7 => {
                let mut bv = BitVec::new();
                bv.push(true);
                bv.push(false);
                bv.push(false);
                bv
            }
            _ => panic!("Invalid delta value"),
        }
    }

    /// Lookup function that accepts Vec<bool> for compatibility
    pub fn lookup_vn_vec(&self, delta: &[bool]) -> Vec<bool> {
        // Convert Vec<bool> to BitVec
        let mut delta_bits = BitVec::<u8, Msb0>::new();
        for &bit in delta.iter().take(3) {
            delta_bits.push(bit);
        }

        // Use the BitVec version of lookup_vn
        let result_bits = self.lookup_vn(&delta_bits);

        // Convert back to Vec<bool>
        result_bits.iter().map(|b| *b).collect()
    }

    /// Helper function to convert Vec<bool> to u8 (for backward compatibility)
    pub fn bin_to_u8(bits: &[bool]) -> u8 {
        let mut result = 0u8;
        for &bit in bits {
            result = (result << 1) | (bit as u8);
        }
        result
    }

    /// Helper function to convert u8 to Vec<bool> (for backward compatibility)
    pub fn u8_to_bin(value: u8) -> Vec<bool> {
        vec![(value & 4) != 0, (value & 2) != 0, (value & 1) != 0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bltm_columns() {
        let bltm = BLTM3x3::new();
        let columns = bltm.columns();

        // Check first column (C1) - [0, 0, 1]
        let mut expected_c1 = BitVec::<u8, Msb0>::new();
        expected_c1.push(false);
        expected_c1.push(false);
        expected_c1.push(true);
        assert_eq!(columns[0], expected_c1);

        // Check second column (C2) - [0, 1, 1]
        let mut expected_c2 = BitVec::<u8, Msb0>::new();
        expected_c2.push(false);
        expected_c2.push(true);
        expected_c2.push(true);
        assert_eq!(columns[1], expected_c2);

        // Check third column (C3) - [1, 1, 1]
        let mut expected_c3 = BitVec::<u8, Msb0>::new();
        expected_c3.push(true);
        expected_c3.push(true);
        expected_c3.push(true);
        assert_eq!(columns[2], expected_c3);
    }

    #[test]
    fn test_lookup_function_bitvec() {
        let bltm = BLTM3x3::new();

        // Test cases from the provided table using BitVec
        let test_cases = [
            (0, vec![false, false, false]), // 000 -> 000
            (1, vec![false, false, true]),  // 001 -> 001
            (2, vec![false, true, true]),   // 010 -> 011
            (3, vec![false, true, false]),  // 011 -> 010
            (4, vec![true, true, false]),   // 100 -> 110
            (5, vec![true, true, true]),    // 101 -> 111
            (6, vec![true, false, true]),   // 110 -> 101
            (7, vec![true, false, false]),  // 111 -> 100
        ];

        for (val, expected) in test_cases.iter() {
            let mut delta = BitVec::<u8, Msb0>::new();
            // Convert value to bits
            delta.push((*val & 4) != 0);
            delta.push((*val & 2) != 0);
            delta.push((*val & 1) != 0);

            let vn = bltm.lookup_vn(&delta);

            // Convert expected to BitVec for comparison
            let mut expected_bits = BitVec::<u8, Msb0>::new();
            for &bit in expected {
                expected_bits.push(bit);
            }

            assert_eq!(
                vn, expected_bits,
                "For delta {:?} (value {}), expected Vn {:?}, got {:?}",
                delta, val, expected_bits, vn
            );
        }
    }

    #[test]
    fn test_lookup_function_vec() {
        let bltm = BLTM3x3::new();

        // Test cases from the provided table using Vec<bool>
        let test_cases = [
            (vec![false, false, false], vec![false, false, false]), // 000 -> 000
            (vec![false, false, true], vec![false, false, true]),   // 001 -> 001
            (vec![false, true, false], vec![false, true, true]),    // 010 -> 011
            (vec![false, true, true], vec![false, true, false]),    // 011 -> 010
            (vec![true, false, false], vec![true, true, false]),    // 100 -> 110
            (vec![true, false, true], vec![true, true, true]),      // 101 -> 111
            (vec![true, true, false], vec![true, false, true]),     // 110 -> 101
            (vec![true, true, true], vec![true, false, false]),     // 111 -> 100
        ];

        for (delta, expected_vn) in test_cases.iter() {
            let vn = bltm.lookup_vn_vec(delta);
            assert_eq!(
                vn, *expected_vn,
                "For delta {:?}, expected Vn {:?}, got {:?}",
                delta, expected_vn, vn
            );
        }
    }

    #[test]
    fn test_binary_conversion() {
        // Test u8_to_bits and bits_to_u8
        for val in 0..8 {
            let bits = BLTM3x3::u8_to_bits(val);
            assert_eq!(BLTM3x3::bits_to_u8(&bits), val);
        }

        // Test Vec<bool> conversions for compatibility
        let vec_tests = [
            (0, vec![false, false, false]),
            (1, vec![false, false, true]),
            (2, vec![false, true, false]),
            (3, vec![false, true, true]),
            (4, vec![true, false, false]),
            (5, vec![true, false, true]),
            (6, vec![true, true, false]),
            (7, vec![true, true, true]),
        ];

        for (num, bin) in vec_tests.iter() {
            assert_eq!(BLTM3x3::u8_to_bin(*num), *bin);
            assert_eq!(BLTM3x3::bin_to_u8(bin), *num);
        }
    }
}
