use std::io;

use privguide::error::ConversionError;
use thiserror::Error;
use oxigraph::store::StorageError;

#[derive(Error, Debug)]
pub enum DBCreationError {
    #[error("Failed on IO operation: {0}")]
    IOError(#[from] io::Error),
    #[error("Failed on database operation: {0}")]
    DBError(#[from] StorageError),
}

#[derive(Error, Debug)]
pub enum LanguageLoadError {
    #[error("Failed to read grammar file '{0}'")]
    GrammarError(#[from] io::Error),
    #[error("Failed converting language metadata to a language: {0}")]
    LanguageError(#[from] ConversionError),
}
