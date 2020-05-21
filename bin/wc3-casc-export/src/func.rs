use crate::constants::GROUPS;
use crate::storage::StorageHandle;
use anyhow::Result;
use pbr::ProgressBar;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use wc3_txt::bom::strip_utf8_bom;
use wc3_txt::parse_sections;

pub fn export(storage: StorageHandle, base_path: &Path) -> Result<()> {
  Ok(())
}
