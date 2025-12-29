//! Bit manipulation utilities for Matter setup payload processing.
//!
//! These functions provide safe, idiomatic ways to convert between integers,
//! byte slices, and bit representations (as slices of `u8` containing 0 or 1),
//! using a Big-Endian bit order as required by the Matter specification.

use crate::error::{BitUtilsError, Result};

/// Converts a u64 integer into a Big-Endian vector of bits.
///
/// Each bit of the integer is represented as a `u8` (either 0 or 1) in the
/// output vector. The most significant bit of the integer appears first.
///
/// # Errors
///
/// Returns a `BitUtilsError::ValueOverflow` if the integer `val` cannot be
/// represented in the given number of `bits`.
///
/// # Example
///
/// ```
/// use matter_setup_code::bit_utils::u64_to_bits_be;
///
/// // 0b1101 = 13
/// let bits = u64_to_bits_be(13, 4).unwrap();
/// assert_eq!(bits, vec![1, 1, 0, 1]);
///
/// // Requesting more bits pads with leading zeros
/// let padded_bits = u64_to_bits_be(13, 8).unwrap();
/// assert_eq!(padded_bits, vec![0, 0, 0, 0, 1, 1, 0, 1]);
///
/// // This will fail because 16 (0b10000) requires 5 bits
/// assert!(u64_to_bits_be(16, 4).is_err());
/// ```
pub fn u64_to_bits_be(val: u64, bits_len: usize) -> Result<Vec<u8>> {
    // Check for overflow before proceeding. A value of 0 is a special case that never overflows.
    if val != 0 && bits_len < 64 && (val >> bits_len) != 0 {
        return Err(BitUtilsError::ValueOverflow {
            value: val,
            bits: bits_len,
        }
        .into());
    }

    let mut bits = Vec::with_capacity(bits_len);
    for i in (0..bits_len).rev() {
        // For positions beyond the 64-bit range of `val`, the bit is always 0.
        // This correctly handles cases where `bits_len` > 64.
        let bit = if i < 64 { (val >> i) & 1 } else { 0 };
        bits.push(bit as u8);
    }
    Ok(bits)
}

/// Converts a Big-Endian slice of bits into a `u64` integer.
///
/// This function is the inverse of `u64_to_bits_be`. The first bit in the
/// slice is treated as the most significant bit. If the slice contains more
/// than 64 bits, the leading bits are ignored.
///
/// # Example
///
/// ```
/// use matter_setup_code::bit_utils::bits_to_u64_be;
///
/// let bits = vec![1, 1, 0, 1];
/// assert_eq!(bits_to_u64_be(&bits), 13); // 0b1101
/// ```
pub fn bits_to_u64_be(bits: &[u8]) -> u64 {
    // `fold` provides a concise and idiomatic way to accumulate the integer value.
    bits.iter()
        .fold(0u64, |acc, &bit| (acc << 1) | (bit as u64 & 1))
}

/// Packs a slice of bits (0s and 1s) into a compact Big-Endian byte vector.
///
/// The input bits are packed starting from the most significant bit of each byte.
/// If the input length is not a multiple of 8, the last byte will be padded
/// with zero bits at the end (the least significant bits).
///
/// # Example
///
/// ```
/// use matter_setup_code::bit_utils::bits_to_bytes_be;
///
/// // 0b11010010, 0b11110000
/// let bits = vec![1, 1, 0, 1, 0, 0, 1, 0, 1, 1, 1, 1];
/// let bytes = bits_to_bytes_be(&bits);
/// assert_eq!(bytes, vec![0xD2, 0xF0]);
/// ```
pub fn bits_to_bytes_be(bits: &[u8]) -> Vec<u8> {
    bits.chunks(8)
        .map(|chunk| {
            chunk
                .iter()
                .enumerate()
                .fold(0u8, |acc, (i, &bit)| acc | (bit << (7 - i)))
        })
        .collect()
}

/// Unpacks a slice of bytes into a Big-Endian vector of bits (0s and 1s).
///
/// This function is the inverse of `bits_to_bytes_be`. Each byte is expanded
/// into 8 bits, with the most significant bit appearing first.
///
/// # Example
///
/// ```
/// use matter_setup_code::bit_utils::bytes_to_bits_be;
///
/// let bytes = vec![0xD2, 0xF0]; // 0b11010010, 0b11110000
/// let bits = bytes_to_bits_be(&bytes);
/// let expected = vec![
///     1, 1, 0, 1, 0, 0, 1, 0, // 0xD2
///     1, 1, 1, 1, 0, 0, 0, 0, // 0xF0
/// ];
/// assert_eq!(bits, expected);
/// ```
pub fn bytes_to_bits_be(bytes: &[u8]) -> Vec<u8> {
    let mut bits = Vec::with_capacity(bytes.len() * 8);
    for &byte in bytes {
        for i in (0..8).rev() {
            bits.push((byte >> i) & 1);
        }
    }
    bits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{MatterPayloadError, BitUtilsError};

    #[test]
    fn test_u64_to_bits_be() {
        assert_eq!(u64_to_bits_be(0b1011, 4).unwrap(), vec![1, 0, 1, 1]);
        assert_eq!(u64_to_bits_be(0b1011, 8).unwrap(), vec![0, 0, 0, 0, 1, 0, 1, 1]);
        assert_eq!(u64_to_bits_be(0, 4).unwrap(), vec![0, 0, 0, 0]);
        assert_eq!(u64_to_bits_be(u64::MAX, 64).unwrap().len(), 64);
    }

    #[test]
    fn test_u64_to_bits_overflow() {
        let result = u64_to_bits_be(16, 4); // 16 is 0b10000, needs 5 bits
        let expected = MatterPayloadError::BitUtils(BitUtilsError::ValueOverflow {
            value: 16,
            bits: 4,
        });
        assert_eq!(result.unwrap_err(), expected);

        // Zero should never overflow
        assert!(u64_to_bits_be(0, 1).is_ok());
        // Exact fit should be ok
        assert!(u64_to_bits_be(15, 4).is_ok());
    }

    #[test]
    fn test_bits_to_u64_be() {
        assert_eq!(bits_to_u64_be(&[1, 0, 1, 1]), 11);
        assert_eq!(bits_to_u64_be(&[0, 0, 0, 0, 1, 0, 1, 1]), 11);
        assert_eq!(bits_to_u64_be(&[0]), 0);
        assert_eq!(bits_to_u64_be(&[]), 0);
    }

    #[test]
    fn test_pack_unpack_roundtrip() {
        let original_bits = vec![1, 0, 1, 1, 0, 1, 0, 1, 1, 1, 1, 0]; // 12 bits
        let bytes = bits_to_bytes_be(&original_bits);
        assert_eq!(bytes, vec![0b10110101, 0b11100000]); // Check packed value

        let unpacked_bits = bytes_to_bits_be(&bytes);
        // Unpacking always yields a multiple of 8, so we slice to the original length.
        assert_eq!(&unpacked_bits[..original_bits.len()], &original_bits[..]);
    }

    #[test]
    fn test_full_byte_packing() {
        let bits = vec![1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0];
        let bytes = bits_to_bytes_be(&bits);
        assert_eq!(bytes, vec![0xFF, 0x00]);
        let unpacked = bytes_to_bits_be(&bytes);
        assert_eq!(unpacked, bits);
    }

    #[test]
    fn test_empty_inputs() {
        assert_eq!(u64_to_bits_be(0, 0).unwrap(), Vec::<u8>::new());
        assert!(u64_to_bits_be(1, 0).is_err());
        assert_eq!(bits_to_u64_be(&[]), 0);
        assert_eq!(bits_to_bytes_be(&[]), Vec::<u8>::new());
        assert_eq!(bytes_to_bits_be(&[]), Vec::<u8>::new());
    }
}