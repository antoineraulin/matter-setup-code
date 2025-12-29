// src/payload/mod.rs

//! Logic for generating and parsing Matter setup payloads.

// Declare the sub-modules. They are private to the `payload` module.
mod common;
mod manual;
mod qr;

// Re-export public-facing types for easier use
pub use common::CommissioningFlow;

use crate::base38;
use crate::bit_utils::{bits_to_u64_be, bytes_to_bits_be};
use crate::error::{PayloadError, Result};
use crate::verhoeff::calculate_checksum;
use deku::prelude::*;
use manual::ManualCodeData;
use qr::QrCodeData;

/// The primary representation of a Matter setup payload.
///
/// This struct holds all the necessary commissioning information and provides
/// methods to generate QR codes and manual pairing codes, or to parse them
/// from a string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupPayload {
    /// Long discriminator (12 bits)
    pub long_discriminator: Option<u16>,
    /// Short discriminator (8 bits)
    pub short_discriminator: u8,
    /// Setup PIN code (27 bits)
    pub pincode: u32,
    /// Discovery capabilities bitmask
    pub discovery: Option<u8>,
    /// Commissioning flow type
    pub flow: CommissioningFlow,
    /// Vendor ID
    pub vid: Option<u16>,
    /// Product ID
    pub pid: Option<u16>,
}

impl SetupPayload {
    /// Creates a new SetupPayload
    ///
    /// # Arguments
    ///
    /// * `discriminator` - 12-bit discriminator value
    /// * `pincode` - 27-bit setup PIN code
    /// * `rendezvous` - Discovery capabilities bitmask (default: 4 for OnNetwork)
    /// * `flow` - Commissioning flow type (default: Standard)
    /// * `vid` - Vendor ID (default: None)
    /// * `pid` - Product ID (default: None)
    pub fn new(
        discriminator: u16,
        pincode: u32,
        rendezvous: Option<u8>,
        flow: Option<CommissioningFlow>,
        vid: Option<u16>,
        pid: Option<u16>,
    ) -> Self {
        let long_discriminator = if discriminator == 0 {
            None
        } else {
            Some(discriminator)
        };
        let short_discriminator = (discriminator >> 8) as u8;
        let discovery = rendezvous.filter(|&d| d != 0);

        SetupPayload {
            long_discriminator,
            short_discriminator,
            pincode,
            discovery,
            flow: flow.unwrap_or(CommissioningFlow::Standard),
            vid,
            pid,
        }
    }

    /// Parses a string to create a `SetupPayload`.
    ///
    /// The string can be either a QR code payload (starting with "MT:") or
    /// a numeric manual pairing code.
    ///
    /// # Errors
    ///
    /// Returns an error if the payload string is malformed, has an invalid
    /// checksum, or cannot be decoded.
    pub fn parse_str(payload_str: &str) -> Result<Self> {
        if payload_str.starts_with("MT:") {
            let container = QrCodeData::parse_from_str(payload_str)?;
            Ok(SetupPayload::new(
                container.discriminator,
                container.pincode,
                Some(container.discovery),
                Some(container.flow),
                Some(container.vid),
                Some(container.pid),
            ))
        } else {
            let container = ManualCodeData::parse_from_str(payload_str)?;
            let mut payload = SetupPayload::new(
                container.discriminator.into(),
                ((container.pincode_msb as u32) << 14) | (container.pincode_lsb as u32),
                None,
                if container.vid_pid_present != 0 {
                    Some(CommissioningFlow::Custom)
                } else {
                    None
                },
                if container.vid_pid_present != 0 {
                    container.vid
                } else {
                    None
                },
                if container.vid_pid_present != 0 {
                    container.pid
                } else {
                    None
                },
            );
            payload.short_discriminator = container.discriminator;
            payload.long_discriminator = None;
            payload.discovery = None;
            Ok(payload)
        }
    }

    /// Generates the QR code string ("MT:...") for this payload.
    pub fn to_qr_code_str(&self) -> Result<String> {
        let qr_data = QrCodeData {
            version: 0,
            vid: self.vid.expect("VID is required for QR code generation"),
            pid: self.pid.expect("PID is required for QR code generation"),
            flow: self.flow,
            discovery: self
                .discovery
                .expect("Discovery is required for QR code generation"),
            discriminator: self
                .long_discriminator
                .expect("Long discriminator is required for QR code generation"),
            pincode: self.pincode,
            padding: 0,
        };

        let mut bytes = qr_data.to_bytes()?;
        bytes.reverse();
        let encoded = base38::encode(&bytes);
        Ok(format!("MT:{}", encoded))
    }

    /// Generates the numeric manual pairing code string for this payload.
    ///
    /// # Errors
    /// Returns an error if the short discriminator is out of range (> 15).
    pub fn to_manual_code_str(&self) -> Result<String> {
        // 1. Map Payload to ManualCode Struct
        // WARNING: Divergence from standard/Python implementation
        // To support round-trip generation via CLI where a user might pass a small integer
        // (e.g. 2) as 'discriminator' expecting it to be the short discriminator,
        // we check if the calculated short_discriminator is 0 AND the long_discriminator
        // is small enough to fit in the 4-bit manual code discriminator field (<= 15).
        let discriminator_val =
            if self.short_discriminator == 0 && self.long_discriminator.unwrap_or(0) <= 15 {
                self.long_discriminator.unwrap_or(0) as u8
            } else {
                self.short_discriminator
            };

        // Safety check: The discriminator in ManualCode must be 4 bits (0-15).
        if discriminator_val > 15 {
            return Err(PayloadError::DiscriminatorOutOfRange(discriminator_val).into());
        }

        let manual_code = ManualCodeData {
            version: 0, // Currently always 0
            vid_pid_present: if self.flow == CommissioningFlow::Standard {
                0
            } else {
                1
            },
            // Discriminator in ManualCode is 4 bits.
            discriminator: discriminator_val,
            // Split 27-bit PIN: Bottom 14 bits -> LSB, Top 13 bits -> MSB
            pincode_lsb: (self.pincode & 0x3FFF) as u16,
            pincode_msb: ((self.pincode >> 14) & 0x1FFF) as u16,
            vid: if self.flow == CommissioningFlow::Standard {
                Some(0)
            } else {
                self.vid
            },
            pid: if self.flow == CommissioningFlow::Standard {
                Some(0)
            } else {
                self.pid
            },
            padding: 0,
        };

        // 2. Serialize Struct to Bytes via Deku
        let packed_bytes = manual_code.to_bytes()?;

        // 3. Unpack bytes to raw bits (Reverse of pack_bits)
        let bits = bytes_to_bits_be(&packed_bytes);

        // 4. Reconstruct Chunks (Reverse of parse_from_str bit logic)
        // The parsing logic constructed the bitstream by concatenating chunks of specific sizes.
        // We must slice the stream using those exact sizes.

        // Chunk 1: 4 bits (Version + Flag + Top 2 bits of Disc) -> 1 Digit
        let c1 = bits_to_u64_be(&bits[0..4]);

        // Chunk 2: 16 bits (Bottom 2 bits of Disc + Pin LSB) -> 5 Digits
        let c2 = bits_to_u64_be(&bits[4..20]);

        // Chunk 3: 13 bits (Pin MSB) -> 4 Digits
        let c3 = bits_to_u64_be(&bits[20..33]);

        // Start building the string
        let mut code_string = format!("{}{:05}{:04}", c1, c2, c3);

        // if has_vid_pid {
        //     // Chunk 4: 16 bits (VID) -> 5 Digits
        //     let c4 = bits_to_u64_be(&bits[33..49]);
        //     // Chunk 5: 16 bits (PID) -> 5 Digits
        //     let c5 = bits_to_u64_be(&bits[49..65]);

        //     code_string.push_str(&format!("{:05}{:05}", c4, c5));
        // }

        // 5. Calculate Checksum (Verhoeff)
        let checksum_digit = calculate_checksum(&code_string)?;

        // Append checksum (convert u8 digit to char)
        code_string.push(std::char::from_digit(checksum_digit as u32, 10).unwrap());

        Ok(code_string)
    }
}

#[cfg(test)]
mod tests {
    use crate::MatterPayloadError;

    use super::*;

    // A standard payload for consistent testing
    fn standard_payload() -> SetupPayload {
        SetupPayload {
            short_discriminator: 4,
            long_discriminator: Some(1132),
            pincode: 69414998,
            vid: Some(0xfff1),
            pid: Some(0x8000),
            flow: CommissioningFlow::Standard,
            discovery: Some(4),
        }
    }

    #[test]
    fn test_qr_code_roundtrip() {
        let original_payload = standard_payload();
        let qr_str = original_payload.to_qr_code_str().unwrap();

        // Python reference:
        // ./chip-tool payload generate -d 1132 -p 69414998 -vid 65521 -pid 32768 -dm 4 -cf 0
        // Manualcode : 11237442363
        // QRCode     : MT:Y.K904QI143LH13SH10
        assert_eq!(qr_str, "MT:Y.K904QI143LH13SH10");

        let parsed_payload = SetupPayload::parse_str(&qr_str).unwrap();
        assert_eq!(original_payload, parsed_payload);
    }

    #[test]
    fn test_manual_code_roundtrip() {
        let original_payload = standard_payload();

        let manual_str = original_payload.to_manual_code_str().unwrap();

        // Python reference:
        // ./chip-tool payload generate -d 1132 -p 69414998 -vid 65521 -pid 32768 -dm 4 -cf 0
        // Manualcode : 11237442363
        // QRCode     : MT:Y.K904QI143LH13SH10
        assert_eq!(manual_str, "11237442363");

        let parsed_payload = SetupPayload::parse_str(&manual_str).unwrap();

        // Note: Manual parsing reconstructs the short discriminator into the high bits of the 12-bit field.
        assert_eq!(
            original_payload.short_discriminator,
            parsed_payload.short_discriminator
        );
        assert_eq!(original_payload.pincode, parsed_payload.pincode);
    }

    #[test]
    fn test_short_manual_code() {
        let payload = SetupPayload {
            short_discriminator: 4,
            long_discriminator: None,
            vid: None,
            pid: None,
            pincode: 69414998,
            flow: CommissioningFlow::Standard,
            discovery: Some(0),
        };
        let manual_str = payload.to_manual_code_str().unwrap();
        // Python ref: 11237442363
        assert_eq!(manual_str, "11237442363");

        let parsed = SetupPayload::parse_str(&manual_str).unwrap();
        assert_eq!(payload.short_discriminator, parsed.short_discriminator);
        assert_eq!(payload.pincode, parsed.pincode);
    }

    #[test]
    fn test_invalid_manual_code_errors() {
        // Invalid length
        let err = SetupPayload::parse_str("12345").unwrap_err();
        assert!(matches!(
            err,
            MatterPayloadError::Payload(PayloadError::InvalidManualCodeLength(5))
        ));

        // Invalid checksum
        let err = SetupPayload::parse_str("20000000031").unwrap_err();
        assert!(matches!(
            err,
            MatterPayloadError::Payload(PayloadError::InvalidManualCodeChecksum)
        ));
    }
}
