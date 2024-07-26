#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| test_parse_duration(data));

pub fn test_parse_duration(data: &[u8]) {
    use std::str::from_utf8;
    use std::str::FromStr;

    // input must be text
    let Ok(original_text) = from_utf8(data) else {
        return;
    };

    // parse input as a duration
    let Ok(duration) = prost_types::Duration::from_str(original_text) else {
        if original_text.ends_with("s") {
            assert!(
                original_text.parse::<f64>().is_err(),
                "prost failed to parse duration, but it seems to be a valid number: {}",
                original_text
            );
        }
        return;
    };

    // roundtrip to and from string
    let roundtrip_text = format!("{duration}");
    assert_eq!(Ok(&duration), roundtrip_text.parse().as_ref());
}
