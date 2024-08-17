#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| test_parse_date(data));

fn test_parse_date(data: &[u8]) {
    use std::str::from_utf8;
    use std::str::FromStr;

    // input must be text
    let Ok(original_text) = from_utf8(data) else {
        return;
    };

    // parse input as a datetime
    let Ok(timestamp) = prost_types::Timestamp::from_str(original_text) else {
        let chrono_parse = chrono::DateTime::parse_from_rfc3339(original_text);
        assert!(
            chrono_parse.is_err(),
            "prost failed to parse time, but chrono does parse this time: {}",
            original_text
        );
        return;
    };

    // roundtrip to and from string
    let roundtrip_text = format!("{timestamp}");
    assert_eq!(Ok(&timestamp), roundtrip_text.parse().as_ref());

    // chrono can only parse year 0000 till 9999
    if let Ok(chrono_time) = chrono::DateTime::parse_from_rfc3339(original_text) {
        if chrono_time.timestamp_subsec_nanos() > 999_999_999 {
            // prost ignores leap seconds, but chrono increases the nanos in that case
            return;
        }

        assert_eq!(timestamp.seconds, chrono_time.timestamp());
        assert_eq!(timestamp.nanos, chrono_time.timestamp_subsec_nanos() as i32);

        // roundtrip using chrono
        let chrono_text = chrono_time.to_utc().to_rfc3339();
        assert_eq!(
            roundtrip_text.strip_suffix("Z").unwrap(),
            chrono_text.strip_suffix("+00:00").unwrap()
        );
        assert_eq!(Ok(&timestamp), chrono_text.parse().as_ref());
    }
}
