use std::borrow::Cow;

#[derive(Debug, PartialEq)]
pub enum Value<'a> {
  Text(Cow<'a, str>),
  Integer(i32),
  Number(f64),
  Unknown(Cow<'a, str>),
}

#[derive(Debug, PartialEq)]
pub enum RecordType<'a> {
  /// A header to identify spreadsheet type and creator.
  ID,
  /// Tells number of rows and columns in the spreadsheet.
  B,
  /// Cell contents
  C,
  E,
  Unknown(&'a str),
}

#[derive(Debug, PartialEq)]
pub struct Record<'a> {
  pub ty: RecordType<'a>,
  pub fields: Vec<Field<'a>>,
}

#[derive(Debug, PartialEq)]
pub enum FieldType<'a> {
  K,
  X,
  Y,
  Unknown(&'a str),
}

#[derive(Debug, PartialEq)]
pub struct Field<'a> {
  pub ty: FieldType<'a>,
  pub value: Value<'a>,
}
