use crate::*;
use serde::{Deserialize, Serialize};
use std::fmt::Formatter;

#[derive(Event)]
struct MockEvent {
    name: String,
    value: u64,
}

#[derive(Deserialize, Serialize)]
struct TestPair {
    key: String,
    val: String,
}

impl std::fmt::Display for TestPair {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "key: {}, val: {}", self.key, self.val)
    }
}

#[derive(Event)]
struct MockEvent2 {
    name: String,
    value: TestPair,
}

#[test]
fn test_emit_event() {
    let test_struct = MockEvent {
        name: String::from("example"),
        value: 10,
    };
    let pair = AbciPair::default();

    let event = Event::emit_event("mock".to_string(), test_struct);
    assert_eq!(event.field_type, "mock_MockEvent");
    assert_eq!(
        event.attributes.to_vec(),
        vec![
            AbciPair {
                key: "name".as_bytes().to_vec(),
                value: "example".as_bytes().to_vec(),
                ..pair.clone()
            },
            AbciPair {
                key: "value".as_bytes().to_vec(),
                value: 10_u32.to_string().as_bytes().to_vec(),
                ..pair
            }
        ]
    );
}

#[test]
fn test_emit_serde_event() {
    let test_struct = MockEvent2 {
        name: String::from("example"),
        value: TestPair {
            key: String::from("key"),
            val: String::from("100"),
        },
    };
    let pair = AbciPair::default();

    let event = Event::emit_serde_event("mock".to_string(), test_struct);
    assert_eq!(event.field_type, "mock_MockEvent2");
    assert_eq!(
        event.attributes.to_vec(),
        vec![
            AbciPair {
                key: "name".as_bytes().to_vec(),
                value: serde_json::to_vec("example").unwrap(),
                ..pair.clone()
            },
            AbciPair {
                key: "value".as_bytes().to_vec(),
                value: serde_json::to_vec(&TestPair {
                    key: String::from("key"),
                    val: String::from("100"),
                })
                .unwrap(),
                ..pair
            }
        ]
    );
}
