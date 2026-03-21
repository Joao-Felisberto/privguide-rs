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

    for query in query_index.get(&QueryKind::SourceCode).or(Some(&Vec::<String>::new())).unwrap() {
        match db.execute_query(query) {
            Ok(mut res) => {
                // println!("{res:#?}");
                let l = res.len();
                println!("Done! {l}");
                res.reverse();
                for mut entry in res {
                    entry.reverse();
                    for sub in entry {
                        let (k, v) = (sub.0, sub.1);
                        print!("{k}: {v}, ");
                    }
                    print!("\n");
                }
            },
            Err(e) => { 
                println!("Error executing source code analysis query '{query}': {e}");
                return;
            }
        }
    }
}
