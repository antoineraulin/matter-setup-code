use thiserror::Error;
use deku::DekuError;

/// The primary error type for the `matter-payload` library.
#[derive(Error, Debug, PartialEq, Eq)]
pub enum MatterPayloadError {
    /// Errors originating from the Base38 decoding process.
    #[error("Base38 decoding failed")]
    Base38(#[from] Base38DecodeError),
    /// Errors originating from the Verhoeff checksum algorithm.
    #[error("Verhoeff algorithm error")]
    Verhoeff(#[from] VerhoeffError),
    /// Errors originating from bit manipulation utilities.
    #[error("Bit utility error")]
    BitUtils(#[from] BitUtilsError),
    /// Errors originating from payload parsing and generation processes.
    #[error("Payload processing error")]
    Payload(#[from] PayloadError),

    #[error("Deku framework error: {0}")]
    Deku(#[from] DekuError),
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

/// Specific errors that can occur during Verhoeff checksum operations.
#[derive(Error, Debug, PartialEq, Eq)]
pub enum VerhoeffError {
    #[error("input contains non-digit character '{0}'")]
    InvalidCharacter(char),

    #[error("input cannot be empty")]
    EmptyInput,
}

/// Specific errors that can occur during bit utility operations.
#[derive(Error, Debug, PartialEq, Eq)]
pub enum BitUtilsError {
    #[error("value {value} overflows the requested {bits} bits")]
    ValueOverflow { value: u64, bits: usize },
}

/// Specific errors that can occur during payload parsing or generation.
#[derive(Error, Debug, PartialEq, Eq)]
pub enum PayloadError {
    #[error("invalid payload length: expected 11 or 21, got {0}")]
    InvalidManualCodeLength(usize),

    #[error("manual code check digit is invalid")]
    InvalidManualCodeChecksum,

    #[error("manual code contains an invalid digit: {0}")]
    InvalidManualCodeDigit(String),

    #[error("manual code's first digit must be <= 7")]
    InvalidManualCodePrefix,

    #[error("QR code payload must start with 'MT:'")]
    InvalidQrCodePrefix,

    #[error("manual code discriminator must be <= 15, but was {0}")]
    DiscriminatorOutOfRange(u8),
}

pub type Result<T> = std::result::Result<T, MatterPayloadError>;