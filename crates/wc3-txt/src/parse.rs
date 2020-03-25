use crate::types::*;
use nom::{
  branch::alt,
  bytes::complete::{escaped, tag, take_while, take_while1},
  character::complete::alphanumeric1,
  character::complete::line_ending,
  character::complete::space0,
  character::complete::{char, one_of},
  combinator::all_consuming,
  combinator::peek,
  combinator::{map, not, opt},
  eof,
  error::context,
  error::VerboseError,
  multi::{fold_many0, many0, many1},
  sequence::{pair, preceded, terminated, tuple},
};

type ParseResult<I, O> = nom::IResult<I, O, VerboseError<I>>;

fn end_of_file(i: &str) -> ParseResult<&str, &str> {
  eof!(i,)
}

fn take_rest_of_line(i: &str) -> ParseResult<&str, &str> {
  take_while(|c| c != '\r' && c != '\n')(i)
}

fn blank_line(i: &str) -> ParseResult<&str, ()> {
  alt((
    map(line_ending, |_| ()),
    map(terminated(space0, line_ending), |_| ()),
  ))(i)
}

fn comment(i: &str) -> ParseResult<&str, &str> {
  let text = preceded(pair(tag("//"), space0), take_rest_of_line);
  terminated(text, alt((end_of_file, line_ending)))(i)
}

fn section_name(i: &str) -> ParseResult<&str, &str> {
  let name = map(
    tuple((char('['), alphanumeric1, char(']'), space0)),
    |(_, name, _, _)| name,
  );
  terminated(name, alt((end_of_file, line_ending)))(i)
}

fn prop_key(i: &str) -> ParseResult<&str, &str> {
  take_while1(|c| c != '=' && c != '\r' && c != '\n')(i)
}

fn invalid_prop_line(i: &str) -> ParseResult<&str, PropValue> {
  let (i, _) = not(peek(char('[')))(i)?;

  map(
    terminated(prop_key, alt((end_of_file, line_ending))),
    PropValue::String,
  )(i)
}

fn quoted_str(i: &str) -> ParseResult<&str, &str> {
  let t = take_while1(|c| c != '"' && c != '\r' && c != '\n');
  preceded(
    char('"'),
    terminated(escaped(t, '\\', one_of("\"n\\")), pair(char('"'), space0)),
  )(i)
}

fn unquoted_str_item(i: &str) -> ParseResult<&str, &str> {
  terminated(
    take_while1(|c| c != '\r' && c != '\n' && c != ','),
    peek(alt((tag(","), alt((end_of_file, line_ending))))),
  )(i)
}

fn prop_value_sep(i: &str) -> ParseResult<&str, &str> {
  map(tuple((space0, char(','), space0)), |_| "")(i)
}

fn prop_value(i: &str) -> ParseResult<&str, PropValue> {
  alt((
    terminated(
      alt((
        // ""
        map(tag("\"\""), |_| PropValue::String("")),
        // ""
        map(quoted_str, PropValue::String),
      )),
      peek(alt((line_ending, end_of_file))),
    ),
    // list
    terminated(
      map(
        many1(alt((
          // ,
          map(
            terminated(
              prop_value_sep,
              peek(alt((prop_value_sep, line_ending, end_of_file))),
            ),
            |_| None,
          ),
          // "..."
          map(quoted_str, Some),
          // ...,
          map(terminated(unquoted_str_item, peek(prop_value_sep)), Some),
          // ,"..."
          // ,...
          map(
            preceded(prop_value_sep, alt((quoted_str, unquoted_str_item))),
            Some,
          ),
        ))),
        |list| PropValue::Array(list.into_iter().filter_map(|v| v).collect()),
      ),
      peek(alt((line_ending, end_of_file))),
    ),
    map(take_rest_of_line, PropValue::String),
  ))(i)
}

fn prop(i: &str) -> ParseResult<&str, (&str, PropValue)> {
  let (i, _) = not(peek(char('[')))(i)?;

  let k = prop_key;
  let v = prop_value;
  let kv = map(tuple((k, char('='), v)), |(k, _, v)| (k, v));
  let prop = terminated(kv, alt((end_of_file, line_ending)));
  prop(i)
}

fn section(i: &str) -> ParseResult<&str, Section> {
  let (i, name) = preceded(opt(comment), section_name)(i)?;
  let prop = preceded(
    opt(comment),
    alt((prop, map(invalid_prop_line, |line| ("", line)))),
  );
  let (i, props) = many0(alt((map(blank_line, |_| None), map(prop, Some))))(i)?;
  Ok((
    i,
    Section {
      name,
      props: props.into_iter().filter_map(|i| i).collect(),
    },
  ))
}

#[derive(Debug, Clone, PartialEq)]
enum Item<'a> {
  Comment(&'a str),
  Section(Section<'a>),
}

fn file(i: &str) -> ParseResult<&str, Vec<Item>> {
  all_consuming(fold_many0(
    alt((
      context("blank line", map(blank_line, |_| None)),
      context("section", map(section, |v| Some(Item::Section(v)))),
      context("comment", map(comment, |v| Some(Item::Comment(v)))),
    )),
    vec![],
    |mut items, item| {
      if let Some(item) = item {
        items.push(item);
      }
      items
    },
  ))(i)
}

pub fn parse_sections(i: &str) -> anyhow::Result<Vec<Section>> {
  use nom::error::convert_error;
  use nom::Err;
  match file(i) {
    Err(Err::Error(e)) | Err(Err::Failure(e)) => {
      Err(anyhow::anyhow!("parse error:\n{}", convert_error(i, e)))
    }
    Err(_) => unreachable!(),
    Ok((_, items)) => Ok(
      items
        .into_iter()
        .filter_map(|i| match i {
          Item::Section(s) => Some(s),
          _ => None,
        })
        .collect(),
    ),
  }
}
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_comment() {
    assert_eq!(
      comment("//    Attribute Bonus"),
      Ok(("", "Attribute Bonus"))
    );
    assert_eq!(comment("// Attribute Bonus"), Ok(("", "Attribute Bonus")));
    assert_eq!(
      comment("// Attribute Bonus\n111"),
      Ok(("111", "Attribute Bonus"))
    );
    assert_eq!(
      comment("//-----------------------------------------------------------------------------\n"),
      Ok((
        "",
        "-----------------------------------------------------------------------------"
      ))
    );
    assert_eq!(
      comment("//    Attribute Bonus\n// 2"),
      Ok(("// 2", "Attribute Bonus"))
    );
  }

  #[test]
  fn test_section_name() {
    assert_eq!(section_name("[ACs7]"), Ok(("", "ACs7")));
    assert_eq!(section_name("[ACs7]   \n"), Ok(("", "ACs7")));
    assert_eq!(section_name("[ACs7]\n111"), Ok(("111", "ACs7")));
  }

  #[test]
  fn test_prop_value() {
    assert_eq!(prop_value(r#"1,2,3"#), Ok(("", vec!["1", "2", "3"].into())));

    assert_eq!(
      prop_value(r#""123",4,5"#),
      Ok(("", vec!["123", "4", "5"].into()))
    );

    assert_eq!(prop_value(r#""12""#), Ok(("", "12".into())));

    assert_eq!(prop_value(r#""1","#), Ok(("", vec!["1"].into())));

    assert_eq!(prop_value(r#""1""2""#), Ok(("", vec!["1", "2"].into())));

    assert_eq!(prop_value(r#""12"   "#), Ok(("", "12".into())));
    assert_eq!(prop_value(r#""12"   ,,,,"#), Ok(("", vec!["12"].into())));
  }

  #[test]
  fn test_prop() {
    assert_eq!(
      prop("UnitSkinID=1233"),
      Ok(("", ("UnitSkinID", "1233".into())))
    );
    assert_eq!(
      prop("UnitSkinID=1233\n"),
      Ok(("", ("UnitSkinID", "1233".into())))
    );
    assert_eq!(
      prop("UnitSkinID=\"123\"\n"),
      Ok(("", ("UnitSkinID", "123".into())))
    );
    assert_eq!(
      prop("UnitSkinID=\"123\",\"345\"\n"),
      Ok(("", ("UnitSkinID", vec!["123", "345"].into())))
    );
    assert_eq!(
      prop("UnitSkinID=\"123\" , \"345\"\n"),
      Ok(("", ("UnitSkinID", vec!["123", "345"].into())))
    );
    assert_eq!(
      prop(r#"Herodeath="1,2","3,4",56,"7,8""#),
      Ok(("", ("Herodeath", vec!["1,2", "3,4", "56", "7,8"].into())))
    );
  }

  #[test]
  fn test_section() {
    assert_eq!(
      section(
        r#"// test!
[ACs7]
skinType=ability
UnitSkinID=1233
Art=ReplaceableTextures\CommandButtons\BTNSpiritWolf.blp
Researchart=ReplaceableTextures\CommandButtons\BTNSpiritWolf.blp


Specialart=Abilities\Spells\Orc\FeralSpirit\feralspirittarget.mdl
//comment"#
      ),
      Ok((
        "//comment",
        Section {
          name: "ACs7",
          props: [
            ("skinType", "ability".into()),
            ("UnitSkinID", "1233".into()),
            (
              "Art",
              r#"ReplaceableTextures\CommandButtons\BTNSpiritWolf.blp"#.into()
            ),
            (
              "Researchart",
              r#"ReplaceableTextures\CommandButtons\BTNSpiritWolf.blp"#.into()
            ),
            (
              "Specialart",
              r#"Abilities\Spells\Orc\FeralSpirit\feralspirittarget.mdl"#.into()
            )
          ]
          .iter()
          .cloned()
          .collect()
        }
      ))
    );
  }

  #[test]
  fn test_file() {
    assert_eq!(
      file(
        r#"//1
// FIRELORD
//2

// Incinerate
[test]
// ??
Name=焚身化骨

"#
      ),
      Ok((
        "",
        vec![
          Item::Comment("1"),
          Item::Comment("FIRELORD"),
          Item::Comment("2"),
          Item::Section(Section {
            name: "test",
            props: [("Name", "焚身化骨".into())].iter().cloned().collect()
          })
        ]
      ))
    );
  }

  #[test]
  fn test_parse_sections_skin() {
    let sections = parse_sections(include_str!("../sample/abilityskin.txt")).unwrap();
    assert_eq!(sections.len(), 1108);

    let sections = parse_sections(include_str!("../sample/unitskin.txt")).unwrap();
    assert_eq!(sections.len(), 874);
  }

  #[test]
  fn test_parse_sections_func() {
    let sections = parse_sections(include_str!("../sample/campaignabilityfunc.txt")).unwrap();
    assert_eq!(sections.len(), 40);
  }

  #[test]
  fn test_parse_sections_strings() {
    let sections = parse_sections(crate::bom::strip_utf8_bom(&include_str!(
      "../sample/neutralabilitystrings.txt"
    )))
    .unwrap();
    assert_eq!(sections.len(), 364);
  }

  #[test]
  fn test_parse_sections_strings2() {
    let sections = parse_sections(crate::bom::strip_utf8_bom(&include_str!(
      "../sample/undeadabilitystrings.txt"
    )))
    .unwrap();
    assert_eq!(sections.len(), 108);
  }
}
