#![cfg(test)]
//! Historical code 3 retains its two distinct principal shapes and refuses malformed or version-swapped encodings.
use std::error::Error;

use ciborium::Value;
use lys_identity::encoding::{decode_body, encode_body};
use lys_identity::event::{Change, IdentityEvent};
use lys_identity::{
    Actor, AuthMethod, IdentityId, LoginBinding, OperationId, PersonId, Profile, Provenance,
    ServiceAccountId,
};

type TestResult = Result<(), Box<dyn Error>>;

fn event(method: AuthMethod) -> Result<IdentityEvent, Box<dyn Error>> {
    Ok(IdentityEvent::new(
        OperationId::from_bytes([1; 16]),
        Actor::new(
            LoginBinding::new("https://issuer.example.test", "fixture")?,
            Provenance::new(method, 1),
        ),
        IdentityId::Person(PersonId::from_bytes([2; 16])),
        2,
        Change::RegisterPerson {
            profile: Profile::new("Compatibility fixture")?,
        },
    )?)
}

fn member(value: &mut Value, key: i32) -> Result<&mut Value, Box<dyn Error>> {
    let Value::Map(entries) = value else {
        return Err("expected a map".into());
    };
    entries
        .iter_mut()
        .find(|(candidate, _)| *candidate == Value::Integer(key.into()))
        .map(|(_, value)| value)
        .ok_or_else(|| "missing map key".into())
}

fn bytes(value: &Value) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut encoded = Vec::new();
    ciborium::into_writer(value, &mut encoded)?;
    Ok(encoded)
}

#[test]
fn historical_code_three_keeps_operator_and_service_account_distinct() -> TestResult {
    let account = ServiceAccountId::from_bytes([3; 16]);
    for (method, version, keys) in [
        (AuthMethod::Operator, 1, 4),
        (AuthMethod::ServiceAccountBearer(account), 2, 5),
    ] {
        let original = event(method)?;
        let body = encode_body(&original);
        let mut value: Value = ciborium::from_reader(body.as_slice())?;
        assert_eq!(*member(&mut value, 1)?, Value::Integer(version.into()));
        let actor = member(&mut value, 3)?;
        let Value::Map(fields) = actor else {
            return Err("actor is not a map".into());
        };
        assert_eq!(fields.len(), keys);
        assert_eq!(*member(actor, 3)?, Value::Integer(3.into()));
        let decoded = decode_body(&body)?;
        assert_eq!(decoded.actor().provenance().method(), method);
        assert_eq!(encode_body(&decoded), body);
        *member(&mut value, 1)? = Value::Integer((3 - version).into());
        assert!(
            decode_body(&bytes(&value)?).is_err(),
            "method {method:?} changed event version"
        );
    }
    Ok(())
}

#[test]
fn malformed_code_three_principals_are_refused_by_name() -> TestResult {
    let original = event(AuthMethod::ServiceAccountBearer(
        ServiceAccountId::from_bytes([3; 16]),
    ))?;
    for invalid in [
        Value::Null,
        Value::Text("service-account".to_owned()),
        Value::Bytes(vec![3; 15]),
        Value::Bytes(vec![3; 17]),
    ] {
        let mut value: Value = ciborium::from_reader(encode_body(&original).as_slice())?;
        *member(member(&mut value, 3)?, 5)? = invalid;
        let error = decode_body(&bytes(&value)?).expect_err("malformed principal admitted");
        assert!(
            error
                .to_string()
                .contains("actor key 5 must hold a 16-byte principal id"),
            "{error}"
        );
    }
    Ok(())
}
