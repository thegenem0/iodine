use std::path::PathBuf;

use async_trait::async_trait;
use uuid::Uuid;

use crate::{error::Error, resource_manager::IOManager};

pub struct LocalFileSystemIOManager {
    id: Uuid,
    file_path: PathBuf,
}

impl LocalFileSystemIOManager {
    pub fn new(file_path: impl Into<PathBuf>) -> Self {
        Self {
            id: Uuid::new_v4(),
            file_path: file_path.into(),
        }
    }
}

#[async_trait]
impl IOManager for LocalFileSystemIOManager {
    fn manager_id(&self) -> Uuid {
        self.id
    }

    async fn read(&self) -> Result<Vec<u8>, Error> {
        match tokio::fs::read(&self.file_path).await {
            Ok(data) => Ok(data),
            Err(io_err) => Err(Error::Internal(format!(
                "Failed to read file at path {}: {}",
                self.file_path.display(),
                io_err
            ))),
        }
    }

    async fn write(&self, data: &[u8]) -> Result<(), Error> {
        match tokio::fs::write(&self.file_path, data).await {
            Ok(()) => Ok(()),
            Err(io_err) => Err(Error::Internal(format!(
                "Failed to write file at path {}: {}",
                self.file_path.display(),
                io_err
            ))),
        }
    }
}
