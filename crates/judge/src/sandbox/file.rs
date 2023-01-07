use std::sync::Arc;

use super::client::{FileGetError, CLIENT};

/// Sandbox file handler.
///
/// Wraps FileHandleInner to implement atomic counting.
/// As such it has a *cheap* `Clone` implementation.
///
/// If the last handler instance of a file is dropped, the file will be deleted in the sandbox.
#[derive(Debug, Clone)]
pub struct FileHandle {
  inner: Arc<FileHandleInner>,
}

#[derive(Debug)]
struct FileHandleInner {
  /// File id.
  id: String,
}

impl Drop for FileHandleInner {
  fn drop(&mut self) {
    log::debug!("dropped file {}", &self.id);
    let id = self.id.clone();
    tokio::spawn(async move { CLIENT.get().await.file_delete(&id).await });
  }
}

impl FileHandle {
  /// Upload a file to sandbox and return it's file hander.
  pub async fn upload(content: &[u8]) -> Self {
    let id = CLIENT.get().await.file_add(content).await;
    Self {
      inner: Arc::new(FileHandleInner { id }),
    }
  }

  /// Create a file handler with file id.
  pub(super) fn from_id(id: String) -> Self {
    Self {
      inner: Arc::new(FileHandleInner { id }),
    }
  }

  /// Get the id of the file corresponding to the FileHandle.
  pub(super) fn id(&self) -> &String {
    &self.inner.id
  }

  /// Get content of file as Vec<u8>.
  pub async fn context(&self) -> Result<Vec<u8>, FileGetError> {
    CLIENT.get().await.file_get(&self.id()).await
  }
}
