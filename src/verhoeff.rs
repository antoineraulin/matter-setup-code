//! An implementation of the Verhoeff checksum algorithm.
//!
//! This algorithm is based on the dihedral group D₅ and is capable of detecting
//! all single-digit errors and all adjacent transposition errors.

use crate::error::{Result, VerhoeffError};

// --- Algorithm Constants ---

/// The multiplication table `d(j, k)` of the dihedral group D₅. This is the
/// core of the Verhoeff algorithm's calculation.
const D_TABLE: [[u8; 10]; 10] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9], [1, 2, 3, 4, 0, 6, 7, 8, 9, 5],
    [2, 3, 4, 0, 1, 7, 8, 9, 5, 6], [3, 4, 0, 1, 2, 8, 9, 5, 6, 7],
    [4, 0, 1, 2, 3, 9, 5, 6, 7, 8], [5, 9, 8, 7, 6, 0, 4, 3, 2, 1],
    [6, 5, 9, 8, 7, 1, 0, 4, 3, 2], [7, 6, 5, 9, 8, 2, 1, 0, 4, 3],
    [8, 7, 6, 5, 9, 3, 2, 1, 0, 4], [9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
];

/// The position-dependent permutation table `p(i, j)`. This table scrambles
/// the digits based on their position in the input string, strengthening the
/// algorithm against transposition errors.
const P_TABLE: [[u8; 10]; 8] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9], [1, 5, 7, 6, 2, 8, 3, 0, 9, 4],
    [5, 8, 0, 3, 7, 9, 6, 1, 4, 2], [8, 9, 1, 6, 0, 4, 3, 5, 2, 7],
    [9, 4, 5, 3, 1, 2, 6, 8, 7, 0], [4, 2, 8, 6, 5, 7, 3, 9, 0, 1],
    [2, 7, 9, 3, 8, 0, 6, 4, 1, 5], [7, 0, 4, 6, 9, 1, 3, 2, 5, 8],
];

/// The inverse table `inv(j)`. Used to find the final checksum digit `c` such
/// that `d(c, checksum) = 0`.
const INV_TABLE: [u8; 10] = [0, 4, 3, 2, 1, 5, 6, 7, 8, 9];

/// A private helper to parse a string slice into a vector of digits.
fn string_to_digits(s: &str) -> std::result::Result<Vec<u8>, VerhoeffError> {
    if s.is_empty() {
        return Err(VerhoeffError::EmptyInput);
    }
    s.chars()
        .map(|c| {
            c.to_digit(10)
                .map(|d| d as u8)
                .ok_or(VerhoeffError::InvalidCharacter(c))
        })
        .collect()
}

/// Calculates the Verhoeff checksum digit for a string of digits.
///
/// # Errors
///
/// Returns an `Err` if the input string is empty or contains non-digit characters.
///
/// # Example
///
/// ```
/// use matter_setup_payload::verhoeff::calculate_checksum;
///
/// let checksum = calculate_checksum("12345").unwrap();
/// assert_eq!(checksum, 1);
/// ```
pub fn calculate_checksum(input: &str) -> Result<u8> {
    let digits = string_to_digits(input)?;
    let mut c = 0u8;

    // The algorithm processes digits from right to left.
    for (i, &digit) in digits.iter().rev().enumerate() {
        // The permutation index `(i + 1)` is used for checksum calculation.
        let permuted_index = (i + 1) % 8;
        let permuted = P_TABLE[permuted_index][digit as usize];
        c = D_TABLE[c as usize][permuted as usize];
    }

    // The final checksum is the inverse of the accumulated value.
    Ok(INV_TABLE[c as usize])
}

/// Validates a string of digits that includes a Verhoeff checksum digit.
///
/// # Errors
///
/// Returns an `Err` if the input string is empty or contains non-digit characters.
///
/// # Example
///
/// ```
/// use matter_setup_code::verhoeff::validate;
///
/// assert!(validate("123451").unwrap());  // Valid
/// assert!(!validate("123450").unwrap()); // Invalid
/// ```
pub fn validate(input: &str) -> Result<bool> {
    let digits = string_to_digits(input)?;
    let mut c = 0u8;

    // The algorithm processes digits from right to left.
    for (i, &digit) in digits.iter().rev().enumerate() {
        // The permutation index `i` is used for validation. This is a subtle
        // but critical difference from the calculation function.
        let permuted_index = i % 8;
        let permuted = P_TABLE[permuted_index][digit as usize];
        c = D_TABLE[c as usize][permuted as usize];
    }

    // A valid string results in an accumulated value of 0.
    Ok(c == 0)
}

/// Appends a Verhoeff checksum digit to a string of digits.
///
/// # Errors
///
/// Returns an `Err` if the input string is empty or contains non-digit characters.
///
/// # Example
///
/// ```
/// use matter_payload::verhoeff::append_checksum;
///
/// let with_checksum = append_checksum("12345").unwrap();
/// assert_eq!(with_checksum, "123451");
/// ```
pub fn append_checksum(input: &str) -> Result<String> {
    let checksum = calculate_checksum(input)?;
    Ok(format!("{input}{checksum}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{MatterPayloadError, VerhoeffError};

    #[test]
    fn test_calculate_checksum() {
        assert_eq!(calculate_checksum("236").unwrap(), 3);
        assert_eq!(calculate_checksum("12345").unwrap(), 1);
        assert_eq!(calculate_checksum("142857").unwrap(), 0);
    }

    #[test]
    fn test_validate() {
        assert!(validate("2363").unwrap());
        assert!(validate("123451").unwrap());
        assert!(!validate("2364").unwrap());
        assert!(!validate("123450").unwrap());
    }

    #[test]
    fn test_append_checksum() {
        assert_eq!(append_checksum("236").unwrap(), "2363");
        assert_eq!(append_checksum("12345").unwrap(), "123451");
    }

    #[test]
    fn test_invalid_input() {
        // Non-digit character
        let result = calculate_checksum("12a45");
        let expected = MatterPayloadError::Verhoeff(VerhoeffError::InvalidCharacter('a'));
        assert_eq!(result.unwrap_err(), expected);

        // Empty input
        let result = validate("");
        let expected = MatterPayloadError::Verhoeff(VerhoeffError::EmptyInput);
        assert_eq!(result.unwrap_err(), expected);
    }

    #[test]
    fn test_single_digit_error_detection() {
        let base = "123456789";
        let full = append_checksum(base).unwrap();

        for i in 0..full.len() {
            let mut chars: Vec<char> = full.chars().collect();
            let original = chars[i].to_digit(10).unwrap();
            for new_digit in 0..10 {
                if new_digit != original {
                    chars[i] = std::char::from_digit(new_digit, 10).unwrap();
                    let modified: String = chars.iter().collect();
                    assert!(
                        !validate(&modified).unwrap(),
                        "Failed to detect single digit error at pos {i}"
                    );
                }
            }
        }
    }

    #[test]
    fn test_transposition_error_detection() {
        let base = "1234567890123";
        let full = append_checksum(base).unwrap();

        for i in 0..full.len() - 1 {
            let mut chars: Vec<char> = full.chars().collect();
            if chars[i] != chars[i + 1] {
                chars.swap(i, i + 1);
                let modified: String = chars.iter().collect();
                assert!(
                    !validate(&modified).unwrap(),
                    "Failed to detect transposition at pos {i}-{i_plus_1}",
                    i_plus_1 = i + 1
                );
            }
        }
    }
}
