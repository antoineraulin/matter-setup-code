use deku::prelude::*;
use crate::base38;
use crate::error::{PayloadError, Result};
use super::common::CommissioningFlow;

/// Represents the binary structure of a Matter QR code payload.
/// This struct is an internal detail and is not exposed publicly.
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
pub(super) struct QrCodeData {
    #[deku(bits = "4")]
    pub padding: u8,
    #[deku(bits = "27")]
    pub pincode: u32,
    #[deku(bits = "12")]
    pub discriminator: u16,
    #[deku(bits = "8")]
    pub discovery: u8,
    pub flow: CommissioningFlow,
    #[deku(bits = "16")]
    pub pid: u16,
    #[deku(bits = "16")]
    pub vid: u16,
    #[deku(bits = "3")]
    pub version: u8,
}

impl QrCodeData {
    /// Parses a raw "MT:..." string into the QR code data structure.
    pub(super) fn parse_from_str(payload: &str) -> Result<Self> {
        if !payload.starts_with("MT:") {
            return Err(PayloadError::InvalidQrCodePrefix.into());
        }

        let encoded = &payload[3..];
        let mut decoded_bytes = base38::decode(encoded)?;
        decoded_bytes.reverse();

        // Deku reads from a bit slice. The `from_bytes` helper creates this for us.
        let (_rest, data) = QrCodeData::from_bytes((&decoded_bytes, 0))?;
        Ok(data)
    }
}