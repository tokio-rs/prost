include!(concat!(env!("OUT_DIR"), "/enum_keyword_variant.rs"));

#[test]
fn test_usage() {
    let _ = Feeding::Assisted;
    let _ = Feeding::Self_;
    let _ = Feeding::Else;
    let _ = Feeding::Error;
    let _ = Feeding::Gen;

    let _ = Grooming::Assisted;
    let _ = Grooming::Self_;
    let _ = Grooming::Else;

    let _ = Number::Number1;
}
