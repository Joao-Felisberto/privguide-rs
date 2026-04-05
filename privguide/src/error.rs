
use oxigraph::sparql::{QueryEvaluationError, SparqlSyntaxError, UpdateEvaluationError};
use oxigraph::model::{BlankNodeIdParseError, IriParseError};
use oxigraph::store::StorageError;
use thiserror::Error;
use tree_sitter::{LanguageError, WasmError};

#[derive(Error, Debug)]
pub enum ConversionError {
    #[error("Failed to open the file: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Unsupported extension: .0")]
    UnsupportedExtensionError(String),
    #[error("Failed parsing the file into JSON: {0}")]
    JSONError(#[from] serde_json::Error),
    #[error("Failed parsing the file into YAML: {0}")]
    YAMLError(#[from] serde_yaml_ng::Error),
}

pub type ConversionResult<T> = Result<T, ConversionError>;

#[derive(Error, Debug)]
pub enum ExecuteQueryError {
    #[error("Query not found: {0}")]
    QueryNotFound(String),
    #[error("Syntax error on query: {0}")]
    SyntaxError(#[from] SparqlSyntaxError),
    #[error("Could not evaluate query: {0}")]
    QueryEvaluationError(#[from] QueryEvaluationError),
    #[error("Could not evaluate update query: {0}")]
    UpdateEvaluationError(#[from] UpdateEvaluationError),
}

/*
#[derive(Error, Debug)]
pub struct NotFoundError(pub String);

impl Display for NotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "'{}' not found.", self.0)
    }
}
*/

#[derive(Error, Debug)]
pub enum IRIConversionError {
    #[error("Prefix '{0}' not found in the prefix map")]
    PrefixNotFoundError(String),
    #[error("Error parsing IRI: {0}")]
    IRIParseError(#[from] IriParseError),
    #[error("Error parsing blank node ID: {0}")]
    BlankNodeParseError(#[from] BlankNodeIdParseError),
    #[error("Impossible to parse a non-terminal node into an IRI")]
    UnparseableObjectError,
}

#[derive(Error, Debug)]
pub enum DatabaseLoadError {
    #[error("Prefix '{0}' not found in the prefix map")]
    PrefixNotFoundError(String),
    #[error("Error parsing IRI: {0}")]
    IRIParseError(#[from] IriParseError),
    #[error("Error parsing blank node ID: {0}")]
    BlankNodeParseError(#[from] BlankNodeIdParseError),
    #[error("Impossible to parse a non-terminal node into an IRI")]
    UnparseableObjectError,
    #[error("Error loading triples into the database: {0}")]
    StorageError(#[from] StorageError)
}

#[derive(Error, Debug)]
pub enum DataLoadError {
    #[error("Failed reading data: {0}")]
    ConversionError(#[from] ConversionError),
    #[error("Failed to open the file: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Failed to interact with the database: {0}")]
    DatabaseError(#[from] DatabaseLoadError),
    #[error("Could not find prefix in path '{0}'")]
    FileWithNoExtensionError(String),
    #[error("Could not find prefix for extension '{0}'")]
    FileExtensionHasNoPrefixError(String),
    #[error("Could not find URI for prefix '{0}'")]
    PrefixHasNoURI(String),
}

#[derive(Error, Debug)]
pub enum CodeParseError {
    #[error("Failed to open source code file: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Cannot parse file with no extension '{0}'")]
    NoExtension(String),
    #[error("No grammar found for extension '{0}'")]
    NoGrammar(String),
    #[error("Could not load grammar from WASM file: {0}")]
    LoadGrammarError(#[from] WasmError),
    #[error("The language was generated with an incompatible version of the Tree-sitter CLI: {0}")]
    LanguageError(#[from] LanguageError),
    #[error("Error loading source code into database: {0}")]
    DatabaseLoadError(#[from] DatabaseLoadError),
}
