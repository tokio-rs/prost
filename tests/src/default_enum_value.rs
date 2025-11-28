//! Issue <https://github.com/tokio-rs/prost/issues/118>
//!
//! When a message contains an enum field with a default value, we
//! must ensure that the appropriate name conventions are used.
#![allow(clippy::enum_variant_names)]

include!(concat!(env!("OUT_DIR"), "/default_enum_value.rs"));

#[test]
fn test_default_enum() {
    let msg = Test::default();
    assert_eq!(msg.privacy_level_1_or_default(), PrivacyLevel::One);
    assert_eq!(
        msg.privacy_level_3_or_default(),
        PrivacyLevel::PrivacyLevelThree
    );
    assert_eq!(
        msg.privacy_level_4_or_default(),
        PrivacyLevel::PrivacyLevelprivacyLevelFour
    );

    let msg = CMsgRemoteClientBroadcastHeader::default();
    assert_eq!(
        msg.msg_type_or_default(),
        ERemoteClientBroadcastMsg::KERemoteClientBroadcastMsgDiscovery
    );
}

#[test]
fn test_enum_to_string() {
    assert_eq!(PrivacyLevel::One.as_str_name(), "PRIVACY_LEVEL_ONE");
    assert_eq!(PrivacyLevel::Two.as_str_name(), "PRIVACY_LEVEL_TWO");
    assert_eq!(
        PrivacyLevel::PrivacyLevelThree.as_str_name(),
        "PRIVACY_LEVEL_PRIVACY_LEVEL_THREE"
    );
    assert_eq!(
        PrivacyLevel::PrivacyLevelprivacyLevelFour.as_str_name(),
        "PRIVACY_LEVELPRIVACY_LEVEL_FOUR"
    );

    assert_eq!(
        ERemoteClientBroadcastMsg::KERemoteClientBroadcastMsgDiscovery.as_str_name(),
        "k_ERemoteClientBroadcastMsgDiscovery"
    );
}

#[test]
fn test_enum_from_string() {
    assert_eq!(
        Some(PrivacyLevel::One),
        PrivacyLevel::from_str_name("PRIVACY_LEVEL_ONE")
    );
    assert_eq!(
        Some(PrivacyLevel::Two),
        PrivacyLevel::from_str_name("PRIVACY_LEVEL_TWO")
    );
    assert_eq!(
        Some(PrivacyLevel::PrivacyLevelThree),
        PrivacyLevel::from_str_name("PRIVACY_LEVEL_PRIVACY_LEVEL_THREE")
    );
    assert_eq!(
        Some(PrivacyLevel::PrivacyLevelprivacyLevelFour),
        PrivacyLevel::from_str_name("PRIVACY_LEVELPRIVACY_LEVEL_FOUR")
    );
    assert_eq!(None, PrivacyLevel::from_str_name("PRIVACY_LEVEL_FIVE"));

    assert_eq!(
        Some(ERemoteClientBroadcastMsg::KERemoteClientBroadcastMsgDiscovery),
        ERemoteClientBroadcastMsg::from_str_name("k_ERemoteClientBroadcastMsgDiscovery")
    );
}

#[test]
fn test_enum_try_from_i32() {
    use core::convert::TryFrom;

    assert_eq!(Ok(PrivacyLevel::One), PrivacyLevel::try_from(1));
    assert_eq!(Ok(PrivacyLevel::Two), PrivacyLevel::try_from(2));
    assert_eq!(
        Ok(PrivacyLevel::PrivacyLevelThree),
        PrivacyLevel::try_from(3)
    );
    assert_eq!(
        Ok(PrivacyLevel::PrivacyLevelprivacyLevelFour),
        PrivacyLevel::try_from(4)
    );
    assert_eq!(Err(prost::UnknownEnumValue(5)), PrivacyLevel::try_from(5));

    assert_eq!(
        Ok(ERemoteClientBroadcastMsg::KERemoteClientBroadcastMsgDiscovery),
        ERemoteClientBroadcastMsg::try_from(0)
    );
}
