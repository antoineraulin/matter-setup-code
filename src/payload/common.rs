use deku::prelude::*;

/// Defines the commissioning flow for the Matter device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(
    id_type = "u8",
    bits = "2",
    endian = "endian",
    ctx = "endian: deku::ctx::Endian"
)]
#[repr(u8)]
pub enum CommissioningFlow {
    /// Standard commissioning flow.
    Standard = 0,
    /// User action is required to confirm commissioning.
    UserIntent = 1,
    /// Vendor-specific, custom commissioning flow.
    Custom = 2,
}