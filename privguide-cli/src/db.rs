use std::{collections::HashMap, hash::Hash};

use privguide::database::{Database, MemDatabase};
use oxigraph::store::StorageError;

use crate::fs;

#[derive(Debug, Hash, PartialEq, Eq)]
pub enum QueryKind {
    Regulation,
    Attack,
    Requirement,
    Reasoner,
    ExtraInfo,
    SourceCode,
}

pub enum DBKind {
    InMemory{dir: String},
}

pub enum DBInstance {
    MemDatabase(MemDatabase),
}

pub fn create_database<T>(kind: DBKind) -> Result<DBInstance, StorageError> {
    match kind {
        DBKind::InMemory{dir} => {
            let prefix_uri_map = fs::get_prefix_uri_map(dir.clone())?;
            let file_prefixes = fs::get_file_prefix_map(dir)?;
            Ok(DBInstance::MemDatabase(MemDatabase::new(prefix_uri_map, file_prefixes)?))
        }
    }
}
