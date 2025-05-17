use async_trait::async_trait;
use uuid::Uuid;

use crate::error::Error;

#[async_trait]
pub trait IOManager: Send + Sync {
    /// Returns a unique identifier for this specific IOManager instance
    fn manager_id(&self) -> Uuid;

    /// Read blob data from a resource.
    /// ---
    /// The returned vector should contain the full contents of the blob.
    async fn read(&self) -> Result<Vec<u8>, Error>;

    /// Read blob data from a resource in batches.
    /// ---
    /// The returned vector will contain a vector of bytes for each batch.
    ///
    /// # Default implementation
    /// This method is implemented by default to return an unimplemented error.
    /// To support this operation, specialize it in the specific IOManager implementation.
    async fn read_batched(&self) -> Result<Vec<Vec<u8>>, Error> {
        Err(Error::Internal("Not implemented".to_string()))
    }

    /// Read continuously from a resource via an iterator.
    /// ---
    /// The returned iterator should yield vectors of bytes.
    /// The returned iterator must be `Send` and `Sync`.
    /// # Default implementation
    /// This method is implemented by default to return an unimplemented error.
    /// To support this operation, specialize it in the specific IOManager implementation.
    async fn read_iter<I: Iterator<Item = Result<Vec<u8>, Error>> + Send>(
        &self,
    ) -> Result<I, Error> {
        Err(Error::Internal("Not implemented".to_string()))
    }

    /// Write blob data to a resource.
    /// ---
    /// The input vector should contain the full contents of the blob.
    async fn write(&self, data: &[u8]) -> Result<(), Error>;

    /// Write blob data to a resource in batches.
    /// ---
    /// The input vector should contain a vector of bytes for each batch.
    /// # Default implementation
    /// This method is implemented by default to return an unimplemented error.
    /// To support this operation, specialize it in the specific IOManager implementation.
    async fn write_batched(&self, _data: &[Vec<u8>]) -> Result<(), Error> {
        Err(Error::Internal("Not implemented".to_string()))
    }

    /// Write continuously to a resource via an iterator.
    /// ---
    /// The input iterator should yeild vectors of bytes.
    /// The input iterator must be `Send` and `Sync`.
    /// # Default implementation
    /// This method is implemented by default to return an unimplemented error.
    /// To support this operation, specialize it in the specific IOManager implementation.
    async fn write_iter<I: Iterator<Item = Result<Vec<u8>, Error>> + Send>(
        &self,
        _data: I,
    ) -> Result<(), Error> {
        Err(Error::Internal("Not implemented".to_string()))
    }
}
