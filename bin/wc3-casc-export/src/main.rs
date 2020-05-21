#[macro_use]
extern crate log;

use casclib::open;
use std::env;
use std::fs;
use std::path::PathBuf;

mod constants;
mod func;
mod helpers;
mod locale;
mod skin;
mod storage;

use storage::StorageHandle;

fn main() {
  dotenv::dotenv().unwrap();
  env_logger::init();

  let storage_path = env::var("WC3_CASC_STORAGE").unwrap();
  info!("CASC storage: {}", storage_path);

  let out_path = PathBuf::from(env::var("OUTDIR").unwrap());
  info!("output path: {:?}", out_path);

  fs::create_dir_all(&out_path).expect("create our dir");

  info!("opening storage...");
  let storage = StorageHandle::new(open(&storage_path).expect("open storage"));

  locale::export(storage.clone(), &out_path).expect("export locale");
  skin::export(storage.clone(), &out_path).expect("export skin");
  // func::export(storage.clone(), &out_path).expect("export func");
}
