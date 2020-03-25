//! SYLK Parse
//!

use crate::types::{Field, FieldType, Record, RecordType, Value};
use nom::{
  branch::alt,
  bytes::complete::{escaped, tag, take_while1},
  character::complete::none_of,
  character::complete::{alpha1, char, digit1},
  combinator::all_consuming,
  combinator::peek,
  combinator::verify,
  combinator::{cut, map, map_res, value},
  eof,
  error::{context, VerboseError},
  multi::{fold_many1, many0},
  number::complete::double,
  sequence::{pair, preceded, terminated},
};

use std::borrow::Cow;

type ParseResult<I, O> = nom::IResult<I, O, VerboseError<I>>;

fn end_of_line(i: &str) -> ParseResult<&str, &str> {
  eof!(i,)
}

fn parse_str(i: &str) -> ParseResult<&str, &str> {
  let take = map(
    fold_many1(
      alt((
        take_while1(|c| c != ';' && c != '"'),
        map(pair(char('"'), peek(none_of(";"))), |_| "\""),
      )),
      0,
      |len, item| len + item.len(),
    ),
    |len| &i[0..len],
  );
  escaped(take, ';', char(';'))(i)
}

fn unescape_str(v: &str) -> Cow<str> {
  if v.contains(';') {
    Cow::Owned(v.replace(";;", ";"))
  } else {
    Cow::Borrowed(v)
  }
}

fn end_of_string_value(i: &str) -> ParseResult<&str, &str> {
  alt((
    map(pair(char('"'), end_of_line), |_| "\""),
    map(pair(tag("\""), peek(pair(char(';'), none_of(";")))), |_| {
      "\""
    }),
  ))(i)
}

fn string_value(i: &str) -> ParseResult<&str, Cow<str>> {
  context(
    "string_value",
    preceded(
      char('"'),
      alt((
        value(Cow::Borrowed(""), end_of_string_value),
        cut(map(
          terminated(parse_str, end_of_string_value),
          unescape_str,
        )),
      )),
    ),
  )(i)
}

fn integer_value(i: &str) -> ParseResult<&str, i32> {
  map_res(verify(digit1, |v: &str| !v.starts_with('0')), |v: &str| {
    v.parse()
  })(i)
}

fn end_of_field(i: &str) -> ParseResult<&str, &str> {
  alt((
    end_of_line,
    map(peek(pair(char(';'), end_of_line)), |_| ""),
    map(peek(pair(char(';'), none_of(";"))), |_| ""),
  ))(i)
}

fn parse_unknown_str(i: &str) -> ParseResult<&str, &str> {
  let (_, len) = fold_many1(
    alt((take_while1(|c| c != ';'), tag(";;"))),
    0,
    |len, item: &str| len + item.len(),
  )(i)?;
  Ok((if len > i.len() - 1 { "" } else { &i[len..] }, &i[0..len]))
}

fn unknown_value(i: &str) -> ParseResult<&str, Cow<str>> {
  context(
    "unknown_value",
    alt((
      value(Cow::Borrowed(""), end_of_field),
      map(terminated(parse_unknown_str, end_of_field), unescape_str),
    )),
  )(i)
}

fn field(i: &str) -> ParseResult<&str, Field> {
  let (i, ty) = preceded(char(';'), cut(alpha1))(i)?;
  let (ty, (i, value)) = match ty {
    "K" => (
      FieldType::K,
      cut(alt((
        map(end_of_field, |_| Value::Unknown(Cow::Borrowed(""))),
        map(double, Value::Number),
        map(string_value, Value::Text),
      )))(i)?,
    ),
    "X" => (FieldType::X, cut(map(integer_value, Value::Integer))(i)?),
    "Y" => (FieldType::Y, cut(map(integer_value, Value::Integer))(i)?),
    unknown => (
      FieldType::Unknown(unknown),
      map(unknown_value, Value::Unknown)(i)?,
    ),
  };
  Ok((i, Field { ty, value }))
}

fn record(i: &str) -> ParseResult<&str, Record> {
  let (i, ty) = cut(alpha1)(i)?;
  let ty = match ty {
    "ID" => RecordType::ID,
    "C" => RecordType::C,
    "B" => RecordType::B,
    "E" => RecordType::E,
    unknown => RecordType::Unknown(unknown),
  };
  let (i, fields) = all_consuming(many0(field))(i)?;
  Ok((i, Record { ty, fields }))
}

pub fn parse_record(i: &str) -> anyhow::Result<Record> {
  use nom::error::convert_error;
  use nom::Err;
  match record(i) {
    Err(Err::Error(e)) | Err(Err::Failure(e)) => {
      Err(anyhow::anyhow!("parse error:\n{}", convert_error(i, e)))
    }
    Err(_) => unreachable!(),
    Ok((_, record)) => Ok(record),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parse_str() {
    assert_eq!(parse_str(r#"123"#), Ok(("", "123")));
    assert_eq!(parse_str(r#"123""#), Ok(("\"", "123")));
    assert_eq!(parse_str("a;;"), Ok(("", "a;;")));
    assert_eq!(parse_str("a\"\""), Ok(("\"", "a\"")));
  }

  #[test]
  #[should_panic]
  fn test_parse_str_failure() {
    parse_str("a;").unwrap();
  }

  #[test]
  fn test_string_value() {
    assert_eq!(string_value(r#""123""#), Ok(("", Cow::Borrowed("123"))));
    assert_eq!(string_value(r#""""#), Ok(("", Cow::Borrowed(""))));
    assert_eq!(
      string_value(r#"";;;;""#),
      Ok(("", Cow::Owned(";;".to_string())))
    );
    assert_eq!(string_value(r#""1";Y"#), Ok((";Y", Cow::Borrowed("1"))));
  }

  #[test]
  fn test_parse_unknown_str() {
    assert_eq!(parse_unknown_str(r#"123"#), Ok(("", "123")));
    assert_eq!(parse_unknown_str(r#"123;"#), Ok((";", "123")));
    assert_eq!(parse_unknown_str(r#"123;;"#), Ok(("", "123;;")));
  }

  #[test]
  fn test_field_k_string() {
    assert_eq!(
      field(r#";K"1""#),
      Ok((
        "",
        Field {
          ty: FieldType::K,
          value: Value::Text(Cow::Borrowed("1"))
        }
      ))
    );

    assert_eq!(
      field(r#";K"1"""#),
      Ok((
        "",
        Field {
          ty: FieldType::K,
          value: Value::Text(Cow::Borrowed("1\""))
        }
      ))
    );

    assert_eq!(
      field(r#";K"""#),
      Ok((
        "",
        Field {
          ty: FieldType::K,
          value: Value::Text(Cow::Borrowed(""))
        }
      ))
    );

    assert_eq!(
      field(r#";K;"#),
      Ok((
        ";",
        Field {
          ty: FieldType::K,
          value: Value::Unknown(Cow::Borrowed(""))
        }
      ))
    );

    assert_eq!(
      field(r#";K"#),
      Ok((
        "",
        Field {
          ty: FieldType::K,
          value: Value::Unknown(Cow::Borrowed(""))
        }
      ))
    );
  }

  #[test]
  fn test_field_k_number() {
    assert_eq!(
      field(r#";K123"#),
      Ok((
        "",
        Field {
          ty: FieldType::K,
          value: Value::Number(123_f64)
        }
      ))
    );

    assert_eq!(
      field(r#";K123.45"#),
      Ok((
        "",
        Field {
          ty: FieldType::K,
          value: Value::Number(123.45_f64)
        }
      ))
    );
  }

  #[test]
  fn test_field_xy() {
    assert_eq!(
      field(r#";X1111"#),
      Ok((
        "",
        Field {
          ty: FieldType::X,
          value: Value::Integer(1111)
        }
      ))
    );

    assert_eq!(
      field(r#";Y222"#),
      Ok((
        "",
        Field {
          ty: FieldType::Y,
          value: Value::Integer(222)
        }
      ))
    );
  }

  #[test]
  fn test_field_unknown() {
    assert_eq!(
      field(r#";WTF1234"#),
      Ok((
        "",
        Field {
          ty: FieldType::Unknown("WTF"),
          value: Value::Unknown(Cow::Borrowed("1234"))
        }
      ))
    );

    assert_eq!(
      field(r#";WTF1234;;"#),
      Ok((
        "",
        Field {
          ty: FieldType::Unknown("WTF"),
          value: Value::Unknown(Cow::Owned("1234;".to_string()))
        }
      ))
    );

    assert_eq!(
      field(r#";WTF1234;;;K"#),
      Ok((
        ";K",
        Field {
          ty: FieldType::Unknown("WTF"),
          value: Value::Unknown(Cow::Owned("1234;".to_string()))
        }
      ))
    );

    assert_eq!(
      field(r#";PWXL;N;E"#),
      Ok((
        ";N;E",
        Field {
          ty: FieldType::Unknown("PWXL"),
          value: Value::Unknown(Cow::Borrowed(""))
        }
      ))
    );
  }

  #[test]
  fn test_record() {
    assert_eq!(
      record(r#"ID;PWXL;N;E"#),
      Ok((
        "",
        Record {
          ty: RecordType::ID,
          fields: vec![
            Field {
              ty: FieldType::Unknown("PWXL"),
              value: Value::Unknown(Cow::Borrowed(""))
            },
            Field {
              ty: FieldType::Unknown("N"),
              value: Value::Unknown(Cow::Borrowed(""))
            },
            Field {
              ty: FieldType::Unknown("E"),
              value: Value::Unknown(Cow::Borrowed(""))
            }
          ]
        }
      ))
    );

    assert_eq!(
      record(r#"C;K""11";Y1;X1"#),
      Ok((
        "",
        Record {
          ty: RecordType::C,
          fields: vec![
            Field {
              ty: FieldType::K,
              value: Value::Text(Cow::Borrowed("\"11"))
            },
            Field {
              ty: FieldType::Y,
              value: Value::Integer(1)
            },
            Field {
              ty: FieldType::X,
              value: Value::Integer(1)
            },
          ]
        }
      ))
    );
  }

  #[test]
  fn test_parse_file() {
    use std::io::BufRead;
    use std::io::BufReader;

    let r = BufReader::new(include_bytes!("../sample/unitabilities.slk") as &[u8]);
    for line in r.lines() {
      let line = line.unwrap();
      parse_record(&line).unwrap();
    }

    let r = BufReader::new(include_bytes!("../sample/unitdata.slk") as &[u8]);
    for line in r.lines() {
      let line = line.unwrap();
      parse_record(&line).unwrap();
    }
  }
}
