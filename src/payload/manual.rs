use crate::bit_utils::*;
use crate::error::{PayloadError, Result};
use crate::verhoeff;
use deku::prelude::*;

/// Represents the binary structure of a Matter manual pairing code.
/// This struct is an internal detail and is not exposed publicly.
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
pub(super) struct ManualCodeData {
    #[deku(bits = "1")]
    pub version: u8,
    #[deku(bits = "1")]
    pub vid_pid_present: u8,
    #[deku(bits = "4")]
    pub discriminator: u8,
    #[deku(bits = "14")]
    pub pincode_lsb: u16,
    #[deku(bits = "13")]
    pub pincode_msb: u16,
    #[deku(bits = "16", cond = "*vid_pid_present == 1")]
    pub vid: Option<u16>,
    #[deku(bits = "16", cond = "*vid_pid_present == 1")]
    pub pid: Option<u16>,
    #[deku(bits = "7")]
    pub padding: u8,
}

impl ManualCodeData {
    /// Parses a raw numeric string into the manual code data structure.
    pub(super) fn parse_from_str(payload: &str) -> Result<Self> {
        let len = payload.len();
        if len != 11 && len != 21 {
            return Err(PayloadError::InvalidManualCodeLength(len).into());
        }

        // let data_part = &payload[..len - 1];
        if !verhoeff::validate(payload)? {
            return Err(PayloadError::InvalidManualCodeChecksum.into());
        }

        let first_digit = payload
            .chars()
            .next()
            .and_then(|c| c.to_digit(10))
            .ok_or(PayloadError::InvalidManualCodeDigit(payload.to_string()))?;

        if first_digit > 7 {
            return Err(PayloadError::InvalidManualCodePrefix.into());
        }

        let is_long = (first_digit & (1 << 2)) != 0;

        // --- Parsing Chunks ---
        // Helper closure to parse slices
        let parse_chunk = |range: std::ops::Range<usize>| -> Result<u64> {
            payload
                .get(range.clone())
                .ok_or(PayloadError::InvalidManualCodeDigit(payload.to_string()))?
                .parse::<u64>()
                .map_err(|e| PayloadError::InvalidManualCodeDigit(e.to_string()).into())
        };

        let chunk1 = parse_chunk(0..1)?;
        let chunk2 = parse_chunk(1..6)?;
        let chunk3 = parse_chunk(6..10)?;
        let (chunk4, chunk5) = if is_long {
            (parse_chunk(10..15)?, parse_chunk(15..20)?)
        } else {
            (0, 0)
        };

        // --- Bit Stream Construction ---
        // We reserve exact capacity to avoid re-allocations (72 bits total)
        let mut bits = Vec::with_capacity(72);

        bits.extend(u64_to_bits_be(chunk1, 4)?);
        bits.extend(u64_to_bits_be(chunk2, 16)?);
        bits.extend(u64_to_bits_be(chunk3, 13)?);

        if is_long {
            bits.extend(u64_to_bits_be(chunk4, 16)?);
            bits.extend(u64_to_bits_be(chunk5, 16)?);
        } else {
            // Fill VID/PID with zeros if not present
            bits.extend(std::iter::repeat(0).take(32));
        }

        // Padding (7 bits)
        bits.extend(std::iter::repeat(0).take(7));

        // --- Pack and Parse ---
        // 1. Pack the expanded bits (0/1) into actual bytes
        let packed_bytes = bits_to_bytes_be(&bits);

        // 2. Deku parses the packed bytes into the Struct
        let ((_rest, _), container) = ManualCodeData::from_bytes((&packed_bytes, 0))?;

        Ok(container)
    }
}
