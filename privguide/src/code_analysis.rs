use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, BufReader, Read},
    path::Path,
};

use tree_sitter::{wasmtime::Engine, Node, Parser, WasmStore};

use crate::{database::Database, error::CodeParseError};

struct Frame<'a> {
    node: Node<'a>,
    subject: String,
}

pub struct Language {
    language: String,
    grammar: Vec<u8>,
    file_extensions: Vec<String>,
    grammar_file: String,
}

impl Language {
    pub fn new(language: String, file_extensions: Vec<String>, grammar_file: String) -> io::Result<Self> {
        let f = File::open(&grammar_file)?;
        let mut reader = BufReader::new(f);
        let mut grammar = Vec::new();

        reader.read_to_end(&mut grammar)?;

        Ok(Self {
            language,
            grammar,
            file_extensions,
            grammar_file,
        })
    }
}

pub struct CodeAnalyser {
    engine: Engine,
    store: WasmStore,
    languages: Vec<Language>,
}

impl CodeAnalyser {
    pub fn new() -> Self {
        let engine = Engine::default();
        let store = WasmStore::new(&engine).unwrap();
        let languages = Vec::new();

        Self {
            engine,
            store,
            languages,
        }
    }

    pub fn add_language(&mut self, language: Language) {
        self.languages.push(language);
    }

    pub fn parse_file<T: Database>(&mut self, db: &mut T, file: &Path) -> Result<(), CodeParseError> {
        let mut parser = Parser::new();
        let mut store = WasmStore::new(&self.engine).unwrap();
        parser.set_wasm_store(store);

        let ext = match file.extension().and_then(|ext| ext.to_str()) {
            Some(ext) => Ok(ext.to_string()),
            None => Err(CodeParseError::NoExtension(format!("{file:?}"))),
        }?;

        let lang = match self
            .languages
            .iter()
            .filter(|l| l.file_extensions.contains(&ext))
            .nth(0)
        {
            Some(lang) => Ok(lang),
            None => Err(CodeParseError::NoGrammar(ext)),
        }?;
        let abi_v = tree_sitter::LANGUAGE_VERSION;
        println!("Parser ABI: {abi_v}");
        // let parser_language = self.store.load_language("lua", LUA_GRAMMAR)?; /*<--- FAIL*/
        let parser_language = self.store.load_language(&lang.language, &lang.grammar)?; /*<--- FAIL*/
        parser.set_language(&parser_language)?;
        // parser.set_language(&tree_sitter_lua::LANGUAGE.into())?;
        let source_code = fs::read_to_string(file)?;

        let tree = parser
            .parse(source_code.clone(), None)
            .expect("Could not parse file");

        db.load_source_code(&tree, source_code.as_str())?;

        Ok(())
    }
}

