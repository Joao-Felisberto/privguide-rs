use std::io;

use thiserror::Error;
use oxigraph::store::StorageError;

#[derive(Error, Debug)]
pub enum DBCreationError {
    #[error("Failed on IO operation: {0}")]
    IOError(#[from] io::Error),
    #[error("Failed on database operation: {0}")]
    DBError(#[from] StorageError),
}
