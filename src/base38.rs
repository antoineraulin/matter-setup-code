//! A Rust implementation of the Matter specification's Base38 encoding scheme.

use crate::error::{Base38DecodeError, Result};

const CODES: [char; 38] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I',
    'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '-', '.',
];
const RADIX: u64 = CODES.len() as u64;

// The Matter specification defines that byte chunks of 1, 2, or 3 bytes
// are encoded into Base38 character chunks of 2, 4, or 5 characters, respectively.
const BASE38_CHARS_NEEDED_IN_CHUNK: [usize; 3] = [2, 4, 5];
const MAX_BYTES_IN_CHUNK: usize = 3;
const MAX_ENCODED_CHARS_IN_CHUNK: usize = 5;

/// Encodes a slice of bytes into a Base38 string.
///
/// The encoding process works on chunks of up to 3 bytes, converting each
/// chunk into a fixed-size character string. This process is repeated for
/// the entire input slice.
///
/// # Example
///
/// ```
/// use matter_setup_code::base38::encode;
///
/// let data = vec![0x12, 0x34, 0x56, 0x78];
/// let encoded = encode(&data);
/// assert_eq!(encoded, "6593L1G");
/// ```
pub fn encode(bytes: &[u8]) -> String {
    let mut qrcode = String::new();
    for chunk in bytes.chunks(MAX_BYTES_IN_CHUNK) {
        // Pack the byte chunk into a u64 value in little-endian order.
        let mut value = chunk
            .iter()
            .enumerate()
            .fold(0u64, |acc, (i, &byte)| acc | ((byte as u64) << (i * 8)));

        let chars_needed = BASE38_CHARS_NEEDED_IN_CHUNK[chunk.len() - 1];

        // Perform the base conversion from base-256 (bytes) to base-38.
        for _ in 0..chars_needed {
            let remainder = (value % RADIX) as usize;
            qrcode.push(CODES[remainder]);
            value /= RADIX;
        }
    }
    qrcode
}

/// Decodes a Base38 string into a vector of bytes.
///
/// The function processes the string in chunks of up to 5 characters,
/// validating characters, chunk lengths, and value ranges.
///
/// # Errors
///
/// Returns `Err` if the input string contains invalid characters, has
/// malformed chunk lengths, or if a decoded value exceeds the range
/// for its chunk size.
///
/// # Example
///
/// ```
/// use matter_setup_code::base38::encode;
///
/// let encoded = "6593L1G";
/// let decoded = decode(encoded).unwrap();
/// assert_eq!(decoded, vec![0x12, 0x34, 0x56, 0x78]);
/// ```
pub fn decode(s: &str) -> Result<Vec<u8>> {
    let mut decoded_bytes = Vec::new();
    let chars: Vec<char> = s.chars().collect();

    for chunk in chars.chunks(MAX_ENCODED_CHARS_IN_CHUNK) {
        // Convert the Base38 character chunk back into an integer.
        // `try_fold` is used to accumulate the value while allowing an early
        // exit with an error if an invalid character is encountered.
        let value = chunk.iter().rev().try_fold(0u64, |acc, &c| {
            CODES
                .iter()
                .position(|&code| code == c)
                .map(|val| acc * RADIX + val as u64)
                .ok_or(Base38DecodeError::InvalidCharacter(c))
        })?;

        let bytes_in_chunk = match chunk.len() {
            2 => 1,
            4 => 2,
            5 => 3,
            len => return Err(Base38DecodeError::InvalidChunkLength(len).into()),
        };

        // This validation is critical. A malformed input could produce a decoded
        // value that is too large to fit into the expected number of bytes.
        // For example, 5 characters could decode to a value greater than 2^24 - 1.
        let max_value = 1u64 << (8 * bytes_in_chunk);
        if value >= max_value {
            return Err(Base38DecodeError::ValueOutOfRange {
                value,
                digits: chunk.len(),
                expected_bytes: bytes_in_chunk,
            }
            .into());
        }

        // Unpack the integer back into little-endian bytes.
        let mut temp_value = value;
        for _ in 0..bytes_in_chunk {
            decoded_bytes.push((temp_value & 0xFF) as u8);
            temp_value >>= 8;
        }
    }

    Ok(decoded_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::MatterPayloadError; 
    use crate::error::Base38DecodeError;

    #[test]
    fn test_round_trip() {
        let original_data = b"Hello, Matter!".to_vec();
        let encoded = encode(&original_data);
        let decoded = decode(&encoded).expect("Decoding failed");
        assert_eq!(original_data, decoded);
    }

    #[test]
    fn test_chunk_boundaries() {
        let inputs: Vec<Vec<u8>> = vec![
            vec![1],
            vec![1, 2],
            vec![1, 2, 3],
            vec![1, 2, 3, 4],
            vec![1, 2, 3, 4, 5],
            vec![1, 2, 3, 4, 5, 6],
            vec![],
        ];
        for input in inputs {
            let encoded = encode(&input);
            let decoded = decode(&encoded).unwrap();
            assert_eq!(input, decoded, "Round trip failed for input: {:?}", input);
        }
    }

    #[test]
    fn test_decode_invalid_character() {
        let result = decode("ABC@123");
        // We know the exact error we expect, so we can construct it and use assert_eq!
        let expected_error = MatterPayloadError::Base38(Base38DecodeError::InvalidCharacter('@'));
        assert_eq!(result.unwrap_err(), expected_error);
    }

    #[test]
    fn test_decode_invalid_length() {
        let result = decode("ABC");
        // Same as above, a direct comparison is clearest.
        let expected_error = MatterPayloadError::Base38(Base38DecodeError::InvalidChunkLength(3));
        assert_eq!(result.unwrap_err(), expected_error);
    }

    #[test]
    fn test_decode_value_out_of_range() {
        // 'ZZZZZ' decodes to 38^5 - 1, which is > 2^24 - 1.
        // This input must be rejected.
        let invalid_input = "ZZZZZ";
        let result = decode(invalid_input);
        
        // Here, we don't care about the exact values inside ValueOutOfRange,
        // just that we got that specific variant. The `matches!` macro is perfect.
        assert!(matches!(
            result,
            Err(MatterPayloadError::Base38(
                Base38DecodeError::ValueOutOfRange { .. }
            ))
        ));
    }

    #[test]
    fn test_edge_cases() {
        let edge_cases = vec![
            vec![0x00; 100],
            vec![0xFF; 100],
            (0..=255).collect(),
        ];
        for case in edge_cases {
            let encoded = encode(&case);
            let decoded = decode(&encoded).expect("Decoding failed");
            assert_eq!(case, decoded, "Edge case failed");
        }
    }
}