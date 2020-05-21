use crate::storage::StorageHandle;
use anyhow::Result;
use serde::Serialize;
use std::collections::HashMap;
use wc3_txt::bom::strip_utf8_bom;
use wc3_txt::parse_sections;
use wc3_txt::PropValue;

#[derive(Debug, PartialEq, Clone, Serialize)]
#[serde(untagged)]
pub enum PropValueOwned {
  String(String),
  Array(Vec<String>),
}

impl<'a> From<PropValue<'a>> for PropValueOwned {
  fn from(v: PropValue<'a>) -> Self {
    match v {
      PropValue::String(v) => PropValueOwned::String(v.to_owned()),
      PropValue::Array(v) => PropValueOwned::Array(v.into_iter().map(ToOwned::to_owned).collect()),
    }
  }
}

/// [id] => (key => value)
pub fn get_txt_data_map(
  storage: &StorageHandle,
  path: &str,
) -> Result<HashMap<String, HashMap<String, PropValueOwned>>> {
  let file = storage.entry(path).open()?;
  let mut buf = Vec::<u8>::with_capacity(file.get_size() as usize);
  file.extract(&mut buf)?;
  let content = String::from_utf8_lossy(&buf);
  let sections = parse_sections(strip_utf8_bom(&content))?;
  Ok(
    sections
      .into_iter()
      .map(|s| {
        (
          s.name.to_string(),
          s.props
            .into_iter()
            .map(|(k, v)| (k.to_string(), PropValueOwned::from(v)))
            .collect(),
        )
      })
      .collect(),
  )
}
