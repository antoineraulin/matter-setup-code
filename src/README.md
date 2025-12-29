# matter-setup-code

[![Crates.io](https://img.shields.io/crates/v/matter-setup-code.svg)](https://crates.io/crates/matter-setup-code)
[![Docs.rs](https://docs.rs/matter-setup-code/badge.svg)](https://docs.rs/matter-setup-code)
[![License](https://img.shields.io/crates/l/matter-setup-code.svg)](LICENSE-MIT)

A robust, type-safe Rust library for parsing and generating Matter onboarding payloads.

This library implements the full specification for Matter Setup Payloads, allowing developers to encode and decode the QR codes and Manual Pairing codes found on Matter-compliant IoT devices. It handles the low-level complexities of Base38 encoding, bit-packing, and Verhoeff checksum validation so you can focus on building your application.

It is a faithful Rust port of the official `setup_payload.py` script found in the Project CHIP (Connected Home over IP) / Matter repository.

## Features

* **QR Code Generation**: Create standard "MT:..." strings ready for QR code rendering.
* **Manual Code Generation**: Generate the 11 or 21-digit numeric codes used for manual entry.
* **Parsing**: Robustly parse existing payload strings into structured data.
* **Validation**: Built-in Verhoeff checksum verification for manual codes.
* **Standard Compliance**: Fully implements the Base38 encoding and bit-packing logic defined in the Matter Core Specification.
* **Type Safety**: Uses Rust enums and structs to ensure valid payload states (e.g., Commissioning Flows).

## Installation

Add the following to your `Cargo.toml` file:

```toml
[dependencies]
matter-setup-code = "0.1.0"
```

## Usage

The core interaction happens through the `SetupPayload` struct. You can create a payload from raw parameters to generate codes, or parse strings to extract those parameters.

### Generating Codes

To generate a QR code or Manual Pairing code, instantiate a `SetupPayload` with your device's commissioning data.

```rust
use matter_setup_code::{SetupPayload, CommissioningFlow};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Define the commissioning parameters for your device
    let payload = SetupPayload::new(
        1132,                           // Discriminator (12-bit)
        69414998,                       // Setup Passcode (PIN)
        65521,                          // Vendor ID (VID)
        32768,                          // Product ID (PID)
        CommissioningFlow::Standard,    // Flow type
        4,                         // Discovery Capabilities (e.g., OnNetwork)
    );

    // Generate the QR Code string (starts with "MT:")
    let qr_string = payload.to_qr_code_str()?;
    println!("QR Payload: {}", qr_string);

    // Generate the Manual Pairing Code (numeric string)
    let manual_code = payload.to_manual_code_str()?;
    println!("Manual Code: {}", manual_code);

    Ok(())
}
```

### Parsing Codes

If you are building a commissioner app (like a mobile app setup tool), you can parse scanned codes to retrieve device information.

```rust
use matter_setup_code::{SetupPayload, CommissioningFlow};

fn main() {
    // A sample QR code string
    let input = "MT:Y.K904QI143LH13SH10";

    match SetupPayload::from_str(input) {
        Ok(payload) => {
            println!("Parsed Successfully!");
            println!("Discriminator: {}", payload.discriminator);
            println!("Passcode:      {}", payload.pincode);
            println!("Vendor ID:     {}", payload.vid);
            println!("Product ID:    {}", payload.pid);
            
            if payload.flow == CommissioningFlow::UserIntent {
                println!("User action required on device.");
            }
        },
        Err(e) => eprintln!("Failed to parse payload: {}", e),
    }
}
```

## Technical Details

The Matter onboarding payload is a compact binary format that packs several pieces of information into a specific bitstream. This library abstracts away the following technical layers:

1.  **Bit Packing**: Data fields (Discriminator, PIN, VID, PID, etc.) are packed into a continuous bit stream, often with non-standard bit widths (e.g., 12 bits, 27 bits).
2.  **Base38 Encoding**: The bit stream is encoded using a custom Base38 alphabet designed for QR codes, utilizing alphanumeric characters while excluding ambiguous ones (like I, O, Q, Z).
3.  **Verhoeff Algorithm**: Manual pairing codes utilize the Verhoeff checksum algorithm to detect single-digit errors and adjacent transpositions, ensuring a better user experience during manual entry.

## Sources and Acknowledgments

This library is based on the logic defined in the Project CHIP (Matter) SDK. Specifically, it ports the functionality of the Python reference implementation:

*   **Source**: https://github.com/project-chip/connectedhomeip
*   **File**: `src/controller/python/matter/setup_payload/setup_payload.py`

While the logic is derived from the official script to ensure correctness, this implementation is native Rust and optimized for safety and ergonomics within the Rust ecosystem.

## License

Licensed under either of

*   Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
*   MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.