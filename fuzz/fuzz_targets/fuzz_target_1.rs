#![no_main]
use libfuzzer_sys::fuzz_target;
use matter_setup_code::SetupPayload;

fuzz_target!(|data: &[u8]| {
    // 1. Try to convert the random bytes to a UTF-8 string.
    // We only care about valid strings because your API expects &str.
    if let Ok(s) = std::str::from_utf8(data) {
        // 2. Feed it to your parser.
        // We assume your parser should return Ok(_) or Err(_), but NEVER panic.
        // The Result is ignored; we only care if the process crashes.
        let _ = SetupPayload::parse_str(s);
    }
});
