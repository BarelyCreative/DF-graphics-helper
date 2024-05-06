use super::*;
use pretty_assertions::assert_eq;

#[test]
fn test_cool_greeting() {
    assert_eq!(GREETING, ".: Cool Tunes :.");
}

// #[test]
// #[should_panic]
// fn test_reject_piano_key_too_high() {
//     assert_eq!(PianoKey::new("A9").unwrap(), PianoKey::default());
// }