use super::client::{FileGetError, CLIENT};

/// Sandbox file handler.
///
/// If the handler is be dropped, the corresponding file will be deleted from the sandbox.
#[derive(Debug)]
pub struct FileHandle {
  /// File id.
  pub(super) id: String,
}

impl Drop for FileHandle {
  fn drop(&mut self) {
    log::debug!("dropped file {}", &self.id);
    tokio::task::block_in_place(|| {
      let handle = tokio::runtime::Handle::current();
      let client = handle.block_on(CLIENT.get());
      handle.block_on(client.file_delete(&self.id));
    });
  }
}

impl FileHandle {
  /// Upload a file to sandbox and return it's file hander.
  pub async fn upload(content: &[u8]) -> Self {
    let id = CLIENT.get().await.file_add(content).await;
    Self { id }
  }

  /// Create a file handler with file id.
  pub(super) fn from_id(id: String) -> Self {
    Self { id }
  }

  /// Get content of file as Vec<u8>.
  pub async fn context(&self) -> Result<Vec<u8>, FileGetError> {
    CLIENT.get().await.file_get(&self.id).await
  }
}
