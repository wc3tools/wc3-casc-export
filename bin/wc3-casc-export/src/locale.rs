use anyhow::Result;
use pbr::ProgressBar;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use crate::constants::{GROUPS, LANGUAGES};
use crate::storage::StorageHandle;
use wc3_txt::bom::strip_utf8_bom;
use wc3_txt::parse_sections;

const SUB_GROUPS: &[&str] = &["ability", "unit", "upgrade", "item"];

pub fn export(storage: StorageHandle, base_path: &Path) -> Result<()> {
  info!("exporting...");

  let mut all_data: HashMap<_, HashMap<_, _>> = HashMap::new();
  let mut errors = vec![];

  let total = LANGUAGES.len() * (GROUPS.len() * SUB_GROUPS.len() + 1);
  let mut pb = ProgressBar::new(total as u64);
  pb.format("[=>_]");

  for lang in LANGUAGES {
    for g in GROUPS {
      for sg in SUB_GROUPS {
        let key = format!("{}_{}", g, sg);
        let path = get_txt_path(lang, g, sg);
        match get_data(&storage, &path) {
          Ok(data) => {
            all_data.entry(lang).or_default().insert(key, data);
          }
          Err(err) => {
            errors.push((lang, *g, *sg, err));
          }
        }

        pb.inc();
      }
    }

    let key = "command";
    let path = get_txt_path(lang, key, "");
    match get_data(&storage, &path) {
      Ok(data) => {
        all_data
          .entry(lang)
          .or_default()
          .insert(key.to_string(), data);
      }
      Err(err) => {
        errors.push((lang, key, "", err));
      }
    }
  }

  pb.finish();

  info!("writing...");

  for (lang, data) in all_data {
    let f = BufWriter::new(File::create(
      base_path.join(format!("locale_{}.json", lang)),
    )?);
    serde_json::to_writer_pretty(f, &data)?;
  }

  for err in errors {
    error!("{:?}", err);
  }

  Ok(())
}

/// [id] => (key => value)
fn get_data(storage: &StorageHandle, path: &str) -> Result<Value> {
  let file = storage.entry(path).open()?;
  let mut buf = Vec::<u8>::with_capacity(file.get_size() as usize);
  file.extract(&mut buf)?;
  let content = String::from_utf8_lossy(&buf);
  let sections = parse_sections(strip_utf8_bom(&content))?;
  let map: HashMap<_, HashMap<_, _>> = sections.into_iter().map(|s| (s.name, s.props)).collect();
  Ok(serde_json::to_value(&map)?)
}

fn get_txt_path(lang: &str, group: &str, sub_group: &str) -> String {
  format!(
    "war3.w3mod:_locales/{}.w3mod:units/{}{}strings.txt",
    lang, group, sub_group
  )
}
