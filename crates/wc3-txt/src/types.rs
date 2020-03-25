use serde_derive::Serialize;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Clone)]
pub struct Section<'a> {
  pub name: &'a str,
  pub props: HashMap<&'a str, PropValue<'a>>,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
#[serde(untagged)]
pub enum PropValue<'a> {
  String(&'a str),
  Array(Vec<&'a str>),
}

impl<'a> From<&'a str> for PropValue<'a> {
  fn from(v: &'a str) -> Self {
    PropValue::String(v)
  }
}

impl<'a> From<Vec<&'a str>> for PropValue<'a> {
  fn from(v: Vec<&'a str>) -> Self {
    PropValue::Array(v)
  }
}
