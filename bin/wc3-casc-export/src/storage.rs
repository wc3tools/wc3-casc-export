use casclib::Storage;
use std::sync::Arc;

#[derive(Clone)]
pub struct StorageHandle(Arc<Storage>);

impl StorageHandle {
  pub fn new(storage: Storage) -> Self {
    StorageHandle(Arc::new(storage))
  }
}

impl std::ops::Deref for StorageHandle {
  type Target = Arc<Storage>;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}
