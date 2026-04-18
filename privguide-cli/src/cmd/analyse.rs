use std::{fs::File, io::BufWriter, path::PathBuf};

use privguide::database::{Database, MemDatabase};
use crate::{db::{self, DBKind, QueryKind}, fs, report::Report};

pub fn analyse(dir: &str) {
    // Create database
    /*
    let prefixes = match fs::get_prefix_map(dir.to_string()) {
        Err(e) => {
            println!("Error loading prefix map: {e}");
            return;
        },
        Ok(pref) => pref,
    };
    */
    let mut report = Report::default();

    let mut db: MemDatabase = match db::create_database::<MemDatabase>(DBKind::InMemory{dir: dir.to_string()}) {
        Ok(db::DBInstance::MemDatabase(db)) => db,
        Err(e) => {
            println!("Error creating database: {}", e);
            return;
        }
        /*
        _ => {
            println!("Wrong database type returned by factory");
            return;
        }
        */
    };

    // Load queries into database
    let query_index = match fs::load_queries(&mut db, dir) {
        Ok(index) => index,
        Err(e) => {
            println!("Error loading queries: {}", e);
            return;
        }
    };
    let idx = query_index.iter().clone();

    // Load descriptions
    if let Err(e) = fs::load_descriptions(&mut db, dir.to_string()) {
        println!("Error loading file descriptions: {e}");
        return;
    }

    // Run reasoner
    for query in query_index.get(&QueryKind::Reasoner).or(Some(&Vec::<String>::new())).unwrap() {
        if let Err(e) = db.execute_update(query) {
            println!("Error executing reasoner rule '{query}': {e}");
            return;
        }
    }

    // Run regulations
    for query in query_index.get(&QueryKind::Regulation).or(Some(&Vec::<String>::new())).unwrap() {
        match db.execute_query(query) {
            Ok(res) => report.add_violations(query.clone(), res),
            Err(e) => println!("Could not run regulation query '{query}': {e}"),
        };
    }

    // Run attacks
    let attack_trees = match fs::load_attack_trees_from_disk(dir.to_string()) {
        Err(e) => {
            println!("Error loading attack trees: {e}");
            return;
        },
        Ok(e) => e,
    };

    for mut atk in attack_trees {
        if let Err(e) = db.run_attack_tree(&mut atk) {
            println!("Error running attack tree: {e}");
            return;
        }
        report.add_attack_tree(atk);
    }

    // Check requirements
    for query in query_index.get(&QueryKind::Requirement).or(Some(&Vec::<String>::new())).unwrap() {
        match db.execute_query(query) {
            Ok(res) => {
                if res.is_empty() {
                    report.add_unmet_requirement(query.clone());
                }
            },
            Err(e) => println!("Could not run requirements query '{query}': {e}"),
        };
    }

    // Extra data
    for query in query_index.get(&QueryKind::ExtraData).or(Some(&Vec::<String>::new())).unwrap() {
        match db.execute_query(query) {
            Ok(res) => report.add_extra_data(query.clone(), res),
            Err(e) => println!("Could not run extra data query '{query}': {e}"),
        };
    }

    // Compile report
    std::fs::create_dir_all("./out").expect("Could not create out dir");

    let mut output_path = PathBuf::from("./out");
    output_path.push("analysis_report");
    output_path.set_extension("json");

    let file = File::create(&output_path).expect("Could not create output file");
    let writer = BufWriter::new(file);

    serde_json::to_writer(writer, &report).expect("Could not write JSON to file");
}

