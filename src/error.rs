use thiserror::Error;

/// The primary error type for the `matter-payload` library.
#[derive(Error, Debug, PartialEq, Eq)]
pub enum MatterPayloadError {
    /// Errors originating from the Base38 decoding process.
    #[error("Base38 decoding failed")]
    Base38(#[from] Base38DecodeError),
    // You can add other top-level errors here later, e.g.:
    // #[error("Payload parsing failed: {0}")]
    // PayloadParse(String),
}

/// Specific errors that can occur during Base38 decoding.
#[derive(Error, Debug, PartialEq, Eq)]
pub enum Base38DecodeError {
    #[error("invalid character '{0}' found in input")]
    InvalidCharacter(char),

    #[error("decoded chunk has an invalid length of {0}; expected 2, 4, or 5")]
    InvalidChunkLength(usize),

    #[error("decoded value {value} from {digits} digits is too large for {expected_bytes} bytes")]
    ValueOutOfRange {
        value: u64,
        digits: usize,
        expected_bytes: usize,
    },
}

pub type Result<T> = std::result::Result<T, MatterPayloadError>;