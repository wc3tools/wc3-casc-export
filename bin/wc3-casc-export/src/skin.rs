use crate::{
  helpers::txt::{get_txt_data_map, PropValueOwned},
  storage::StorageHandle,
};
use anyhow::Result;
use pbr::ProgressBar;
use serde_json::Value;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::{create_dir_all, File};
use std::io::BufWriter;
use std::path::Path;
use wc3_image_conv::dds;
use wc3_txt::bom::strip_utf8_bom;
use wc3_txt::parse_sections;

const GROUPS: &[&str] = &["unitskin", "abilityskin", "itemskin"];

pub fn export(storage: StorageHandle, base_path: &Path) -> Result<()> {
  info!("parsing txt...");

  let mut all_data: HashMap<_, _> = HashMap::new();
  let mut all_arts = HashSet::new();
  let mut errors = vec![];

  for name in GROUPS {
    let path = get_txt_path(name);
    match get_txt_data_map(&storage, &path) {
      Ok(data) => {
        for (id, map) in &data {
          if let Some(art) = map.get("Art") {
            match *art {
              PropValueOwned::String(ref path) => {
                all_arts.insert(path.clone());
              }
              PropValueOwned::Array(ref values) => all_arts.extend(values.clone()),
            }
          }
        }
        all_data.insert(name, data);
      }
      Err(err) => {
        errors.push((name, err));
      }
    }
  }

  info!("found arts: {}", all_arts.len());

  let mut pb = ProgressBar::new(all_arts.len() as u64);
  pb.format("[=>_]");

  let art_base = base_path.join("arts");
  create_dir_all(&art_base.join("sd"))?;
  create_dir_all(&art_base.join("hd"))?;
  let mut art_buf = Vec::with_capacity(1024 * 1024 * 1024);

  let mut not_found_arts = vec![];

  for path in all_arts {
    if path.is_empty() {
      continue;
    }
    if !path.starts_with("ReplaceableTextures\\CommandButtons\\")
      && !path.starts_with("ReplaceableTextures\\PassiveButtons\\")
    {
      warn!("unexpected path: {}", path);
      continue;
    }

    let dds_path = if path.ends_with(".blp") {
      format!("{}dds", &path[..(path.len() - 3)]).replace("\\", "/")
    } else {
      warn!("not a blp path: {}", path);
      continue;
    };

    let filename = Path::new(&dds_path).file_stem().unwrap().to_string_lossy() + ".jpg";

    {
      let f = match storage.entry(get_sd_dds_path(&dds_path)).open() {
        Ok(f) => f,
        Err(casclib::CascError::FileNotFound) => {
          not_found_arts.push(path);
          continue;
        }
        Err(err) => return Err(err.into()),
      };
      art_buf.clear();
      f.extract(&mut art_buf)?;
      let out = File::create(&art_base.join("sd").join(&filename as &str))?;
      dds::encode_jpg(&art_buf, out)?;
    }

    {
      let f = match storage.entry(get_hd_dds_path(&dds_path)).open() {
        Ok(f) => f,
        Err(casclib::CascError::FileNotFound) => {
          not_found_arts.push(path);
          continue;
        }
        Err(err) => return Err(err.into()),
      };
      art_buf.clear();
      f.extract(&mut art_buf)?;
      let out = File::create(&art_base.join("hd").join(&filename as &str))?;
      dds::encode_jpg(&art_buf, out)?;
    }

    pb.inc();
  }

  pb.finish();

  for path in not_found_arts {
    warn!("art not found: {:?}", path);
  }

  info!("writing...");

  for (name, data) in all_data {
    let f = BufWriter::new(File::create(base_path.join(format!("skin_{}.json", name)))?);
    serde_json::to_writer_pretty(f, &data)?;
  }

  for err in errors {
    error!("{:?}", err);
  }

  Ok(())
}

fn get_txt_path(name: &str) -> String {
  format!("war3.w3mod:units/{}.txt", name)
}

fn get_hd_dds_path(path: &str) -> String {
  format!("war3.w3mod:_hd.w3mod:{}", path)
}

fn get_sd_dds_path(path: &str) -> String {
  format!("war3.w3mod:{}", path)
}
