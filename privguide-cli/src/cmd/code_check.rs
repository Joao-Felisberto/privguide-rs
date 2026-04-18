use std::{collections::{HashMap, HashSet}, fs::File, io::BufWriter, path::{Path, PathBuf}};

use privguide::{code_analysis::CodeAnalyser, database::{Database, MemDatabase}};

use crate::{db::{self, DBKind, QueryKind}, fs};

pub fn analyse(dir: &str, code_dir: &str) {
    let mut db: MemDatabase = match db::create_database::<MemDatabase>(DBKind::InMemory{dir: dir.to_string()}) {
        Ok(db::DBInstance::MemDatabase(db)) => db,
        Err(e) => {
            println!("Error creating database: {}", e);
            return;
        }
    };

    let mut code_analyser = CodeAnalyser::new();

    let query_index = match fs::load_queries(&mut db, dir) {
        Ok(index) => index,
        Err(e) => {
            println!("Error loading queries: {}", e);
            return;
        }
    };

    match fs::load_languages(dir) {
        Ok(languages) => {
            languages.into_iter().for_each(|l| code_analyser.add_language(l));
        },
        Err(e) => {
            println!("Error loading grammars: {}", e);
            return;
        }
    };

    if let Err(e) = fs::load_source_code_files(code_dir, &mut db, &mut code_analyser){
        println!("Error loading source code files: {e}");
        return;
    };

    // let mut query_results = HashMap::new();
    for query in query_index.get(&QueryKind::SourceCode).or(Some(&Vec::<String>::new())).unwrap() {
        match db.execute_query(query) {
            Ok(res) => {
                // query_results.insert(query.clone(), res);
                let input_path = Path::new(query.as_str());

                // Extract file stem (filename without extension)
                let file_stem = input_path
                    .file_stem()
                    .expect("Invalid input path: no file name");

                // Ensure output directory exists
                std::fs::create_dir_all("./out").expect("Could not create out dir");

                // Build output path: output_dir/<file_stem>.json
                let mut output_path = PathBuf::from("./out");
                output_path.push(file_stem);
                output_path.set_extension("json");

                // Create file
                let file = File::create(&output_path).expect("Could not create output file");
                let writer = BufWriter::new(file);

                // Write JSON
                // serde_json::to_writer_pretty(writer, &res).expect("Could not write JSON to file");
                serde_json::to_writer(writer, &res).expect("Could not write JSON to file");
            },
            Err(e) => { 
                println!("Error executing source code analysis query '{query}': {e}");
                return;
            }
        }
    }
    // println!("{query_results:#?}");

    // Ok(())
}
