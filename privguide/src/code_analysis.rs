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
    // TODO: try to remove old crap
    engine: Engine,
    store: WasmStore,
    languages: Vec<Language>,
    extension_parser_map: HashMap<String, Parser>,
}

impl CodeAnalyser {
    pub fn new() -> Self {
        let engine = Engine::default();
        let store = WasmStore::new(&engine).unwrap();
        let languages = Vec::new();
        let extension_parser_map = HashMap::new();

        Self {
            engine,
            store,
            languages,
            extension_parser_map,
        }
    }

    pub fn add_language(&mut self, language: Language) {
        language.file_extensions.iter()
            .for_each(|ext| {
                let mut parser = Parser::new();
                let engine = Engine::default();
                let mut store = WasmStore::new(&engine).unwrap();
                let lang = store.load_language(&language.language, &language.grammar)
                    .expect("Could not load language");
                parser.set_wasm_store(store);
                parser
                    .set_language(&lang.into())
                    .expect("Error loading parser");
                self.extension_parser_map.insert(ext.clone(), parser);
            });
        self.languages.push(language);
    }

    pub fn parse_file<T: Database>(&mut self, db: &mut T, file: &Path) -> Result<(), CodeParseError> {
        /*
        let ext = match file.extension().and_then(|ext| ext.to_str()) {
            Some(ext) => Ok(ext.to_string()),
            None => Err(CodeParseError::NoExtension(format!("{file:?}"))),
        }?;
        */
        let res_opt = file.extension().and_then(|ext| ext.to_str());
        if res_opt.is_none() {
            return Ok(());
        }
        let ext = res_opt.unwrap();

        /*
        let lang = match self
            .languages
            .iter()
            .filter(|l| l.file_extensions.contains(&ext))
            .nth(0)
        {
            Some(lang) => Ok(lang),
            None => Err(CodeParseError::NoGrammar(ext)),
        }?;
        */
        let parser = self.extension_parser_map.get_mut(ext);
        if parser.is_none() {
            return Ok(())
        }
        let parser = parser.unwrap();
        // parser.set_language(&tree_sitter_lua::LANGUAGE.into())?;
        let source_code = fs::read_to_string(file)?;

        let tree = parser
            .parse(source_code.clone(), None)
            .expect("Could not parse file");

        db.load_source_code(&tree, source_code.as_str())?;

        Ok(())
    }
}

