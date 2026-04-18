use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, BufReader, Read},
    path::Path,
};

use tree_sitter::{Parser, WasmStore, wasmtime::Engine};

use crate::{database::Database, error::{CodeParseError, LanguageLoadError}};

pub struct Language {
    language: String,
    grammar: Vec<u8>,
    file_extensions: Vec<String>,
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
        })
    }
}

#[derive(Default)]
pub struct CodeAnalyser {
    // TODO: try to remove old crap
    languages: Vec<Language>,
    extension_parser_map: HashMap<String, Parser>,
}

impl CodeAnalyser {
    pub fn add_language(&mut self, language: Language) -> Result<(), LanguageLoadError> {
        for ext in language.file_extensions.iter() {
            let mut parser = Parser::new();
            let engine = Engine::default();
            let mut store = WasmStore::new(&engine)?;
            let lang = store.load_language(&language.language, &language.grammar)?;
            parser.set_wasm_store(store)?;
            parser.set_language(&lang)?;
            self.extension_parser_map.insert(ext.clone(), parser);
        }
        self.languages.push(language);
        Ok(())
    }

    pub fn parse_file<T: Database>(&mut self, db: &mut T, file: &Path) -> Result<(), CodeParseError> {
        let res_opt = file.extension().and_then(|ext| ext.to_str());
        if res_opt.is_none() {
            return Ok(()); // FIXME all files with no extension are ignored, make this a more
                           // graceful experience
        }
        let ext = res_opt.unwrap();

        let parser = self.extension_parser_map.get_mut(ext);
        if parser.is_none() {
            return Ok(())
        }
        let parser = parser.unwrap();
        let source_code = fs::read_to_string(file)?;

        let tree = parser
            .parse(source_code.clone(), None)
            .expect("No language has been assigned to the parser, this should never occur!");

        db.load_source_code(&tree, source_code.as_str())?;

        Ok(())
    }
}

