use std::{collections::HashMap, io, path::Path, fs::File};
use serde::Deserialize;
use serde_json;

use privguide::{code_analysis::{CodeAnalyser, Language}, converters::{Format, convert_file, convert_file_from_format}, database::Database, error::{CodeParseError, ConversionError, DataLoadError}, query::{AttackTree, Query}};

use crate::{db::QueryKind, error::LanguageLoadError};

const EXTENSIONS: [&str; 2] = ["rq", "sparql"];
const DESCRIPTIONS_DIR: &str = "descriptions";
const ATTACK_DIR: &str = "attack_trees";
const SUBDIR_AND_QUERYKIND: [(&str, QueryKind); 7]  = [ 
    ("attack_trees", QueryKind::Attack),
    ("reasoner", QueryKind::Reasoner),
    ("regulations", QueryKind::Regulation),
    ("report_data", QueryKind::ExtraData),
    ("requirements", QueryKind::Requirement),
    ("code_gen", QueryKind::CodeGenData),
    ("source_code", QueryKind::SourceCode),
];
const GRAMMARS_DIR: &str = "grammars";

// TODO: load a JSON/YAML file of these and convert 
#[derive(Deserialize)]
pub struct LanguageMetadata {
    language: String,
    file_extensions: Vec<String>,
    grammar_file: String,
}

impl TryFrom<LanguageMetadata> for Language {
    type Error = io::Error;

    fn try_from(lm: LanguageMetadata) -> Result<Self, Self::Error> {
        Language::new(lm.language.clone(), lm.file_extensions.clone(), lm.grammar_file.clone())
    }
}

pub fn get_prefix_uri_map(dir: String) -> Result<HashMap<String, String>, io::Error> {
    let file_path = Path::new(&dir).join("prefixes.json");
    let file = File::open(file_path)?;
    let reader = io::BufReader::new(file);

    let res: HashMap<_, _> = serde_json::from_reader(reader)?;
    Ok(res)
}

pub fn get_file_prefix_map(dir: String) -> Result<HashMap<String, String>, io::Error> {
    let file_path = Path::new(&dir).join("file_prefixes.json");
    let file = File::open(file_path)?;
    let reader = io::BufReader::new(file);

    let res: HashMap<_, _> = serde_json::from_reader(reader)?;
    Ok(res)
}

pub fn load_queries<T: Database>(db: &mut T, dir: &str) -> Result<HashMap<QueryKind, Vec<String>>, io::Error> {
    let dir = Path::new(dir);
    let mut query_index = HashMap::new();

    for (subdir, kind) in SUBDIR_AND_QUERYKIND {
        let mut stack = Vec::new();
        let mut query_keys = Vec::new();
        
        stack.push(std::fs::read_dir(dir.join(Path::new(subdir)).as_path())?);
        
        while let Some(mut dir_iter) = stack.pop() {
            while let Some(entry) = dir_iter.next() {
                let entry = entry?;
                let path = entry.path();
                
                if path.is_dir() {
                    stack.push(dir_iter);
                    stack.push(std::fs::read_dir(&path)?);
                    break;
                } 

                if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) && 
                    EXTENSIONS.iter().any(|&ext| ext.eq_ignore_ascii_case(extension)) {
                        let query = Query::from_file(path.as_path())?;
                        let q_name = path.to_str().unwrap().to_string();
                        query_keys.push(q_name.clone());
                        db.load_query(q_name, query);
                }
            }
        }

        query_index.insert(kind, query_keys);
    }

    Ok(query_index)
}

pub fn load_descriptions<T: Database>(db: &mut T, dir: String) -> Result<(), DataLoadError> {
    let dir = Path::new(&dir).join(DESCRIPTIONS_DIR);
    let dir = dir.as_path();
    // TODO: fix this as to not require the clones
    let file_prefixes = db.get_file_prefixes().clone();

    let mut stack = Vec::new();
    
    stack.push(std::fs::read_dir(dir)?);
    
    while let Some(mut dir_iter) = stack.pop() {
        while let Some(entry) = dir_iter.next() {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                stack.push(dir_iter);
                stack.push(std::fs::read_dir(&path)?);
                break;
            } 

            let quads = convert_file(path.as_path())?;
            let extension = match extract_placeholder_from_path(&path) {
                Some(ext) => Ok(ext),
    None => Err(DataLoadError::FileWithNoExtensionError(path.to_str().unwrap_or("").to_string())),
            }?;
            let file_prefix = match file_prefixes.get(&extension){
                Some(pref) => Ok(pref),
                None => Err(DataLoadError::FileExtensionHasNoPrefixError(extension)),
            }?;
            db.load_file_data(&quads, file_prefix)?;
        }
    }

    Ok(())
}

pub fn load_attack_trees_from_disk(dir: String) -> Result<Vec<AttackTree>, ConversionError> {
    let dir = Path::new(&dir).join(ATTACK_DIR);
    let dir = dir.as_path();

    let mut stack = Vec::new();
    let mut trees = Vec::new();
    
    stack.push(std::fs::read_dir(dir)?);
    
    while let Some(mut dir_iter) = stack.pop() {
        while let Some(entry) = dir_iter.next() {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                stack.push(dir_iter);
                stack.push(std::fs::read_dir(&path)?);
                break;
            } 

            if  let Some(ext) = path.extension() &&
                let Some(e) = ext.to_str() && 
                e == "yml" {
                    let tree = AttackTree::from_file(path.as_path())?;
                    trees.push(tree);
            }
            
        }
    }

    Ok(trees)
}

pub fn load_languages(dir: &str) -> Result<Vec<Language>, LanguageLoadError> {
    let path = Path::new(dir).join(GRAMMARS_DIR).join("grammars.json");
    let languages_metadata: Vec<LanguageMetadata> = convert_file_from_format(path.as_path(), Format::JSON)?;

    
    languages_metadata.into_iter()
        .map(|lm| {
            let meta = LanguageMetadata {
                language: lm.language,
                file_extensions: lm.file_extensions,
                grammar_file: Path::new(dir).join(GRAMMARS_DIR).join(lm.grammar_file).canonicalize()?.as_path().to_str().unwrap().to_string(),
            };
            Language::try_from(meta).map_err(LanguageLoadError::GrammarError)
        })
        .collect()
}

pub fn load_source_code_files<T: Database>(source_code_dir: &str, db: &mut T, code_analyser: &mut CodeAnalyser) -> Result<(), CodeParseError> {
    let dir = Path::new(&source_code_dir);

    let mut stack = Vec::new();
    
    stack.push(std::fs::read_dir(dir)?);
    
    while let Some(mut dir_iter) = stack.pop() {
        while let Some(entry) = dir_iter.next() {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                stack.push(dir_iter);
                stack.push(std::fs::read_dir(&path)?);
                break;
            } 

            match code_analyser.parse_file(db, path.as_path()) {
                Ok(()) | Err(CodeParseError::NoGrammar(_)) => Ok(()),
                Err(e) => Err(e), 
            }?;
        }
    }

    Ok(())
}

fn extract_placeholder_from_path<P: AsRef<Path>>(path: P) -> Option<String> {
    let path = path.as_ref();
    
    let file_stem = path.file_stem()?.to_str()?;
    
    if let Some(last_underscore) = file_stem.rfind('_') {
        let after_underscore = &file_stem[last_underscore + 1..];
        
        if !after_underscore.is_empty() {
            return Some(after_underscore.to_string());
        }
    }
    
    None
}

