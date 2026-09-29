#![cfg(test)]

use super::launch_fields::{Channel, Literal};

#[test]
fn false_zero_and_empty_text_are_three_distinct_set_texts() {
    let texts: Vec<String> = [
        Literal::Boolean(false),
        Literal::Integer(0),
        Literal::Text(String::new()),
        Literal::Integer(-7),
        Literal::Text("0".to_owned()),
    ]
    .iter()
    .map(Literal::text)
    .collect();
    assert_eq!(texts, ["false", "0", "", "-7", "0"]);
}

#[test]
fn literals_read_back_as_the_kind_they_were_recorded() -> Result<(), serde_json::Error> {
    let read: Vec<Literal> = serde_json::from_str(r#"[false, 0, "", "false", "0"]"#)?;
    assert_eq!(
        read,
        [
            Literal::Boolean(false),
            Literal::Integer(0),
            Literal::Text(String::new()),
            Literal::Text("false".to_owned()),
            Literal::Text("0".to_owned()),
        ]
    );
    assert!(
        serde_json::from_str::<Literal>("1.5").is_err(),
        "a fraction is no literal"
    );
    assert!(
        serde_json::from_str::<Literal>("[1]").is_err(),
        "a list is no literal"
    );
    assert_eq!(Channel::default(), Channel::Off);
    Ok(())
}
