use std::{fs::File, io::{BufWriter, Write}, path::{Path, PathBuf}};

use privguide::database::{Database, MemDatabase};
use tera::{Context, Tera};
use crate::{db::{self, DBKind, QueryKind, triples_to_json}, fs, report::Report};

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

    // Load descriptions
    if let Err(e) = fs::load_descriptions(&mut db, dir.to_string()) {
        println!("Error loading file descriptions: {e}");
        return;
    }

    // Run reasoner
    for query in query_index.get(&QueryKind::Reasoner).unwrap_or(&Vec::<String>::new()) {
        if let Err(e) = db.execute_update(query) {
            println!("Error executing reasoner rule '{query}': {e}");
            return;
        }
    }

    // Run regulations
    for query in query_index.get(&QueryKind::Regulation).unwrap_or(&Vec::<String>::new()) {
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
    for query in query_index.get(&QueryKind::Requirement).unwrap_or(&Vec::<String>::new()) {
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
    for query in query_index.get(&QueryKind::ExtraData).unwrap_or(&Vec::<String>::new()) {
        match db.execute_query(query) {
            Ok(res) => report.add_extra_data(query.clone(), res),
            Err(e) => println!("Could not run extra data query '{query}': {e}"),
        };
    }

    // Generate code
    let tera_dir = Path::new(dir)
        .join("code_templates/");
    let tera_dir = tera_dir.to_str().unwrap().to_owned() + "**";
    
    let tera = Tera::new(tera_dir.as_str()); 
    if let Err(e) = tera {
        println!("Failed to create Tera template engine: {e}");
        return;
    }
    let mut tera = tera.unwrap();
    tera.autoescape_on(vec![]);

    for query in query_index.get(&QueryKind::CodeGenData).unwrap_or(&Vec::<String>::new()) {
        println!("Running {query}");
        match db.execute_graph_query(query) {
            Ok(res) => {
                let json_res = triples_to_json(&db, res);
                let entries = match json_res.get("ex_all") {
                    Some(serde_json::Value::Array(es)) => es,
                    _ => &Vec::new(),
                };
                for entry in entries {
                    println!("{entry:#?}");
                    let f_name = match entry.get("ex_f_name") {
                        Some(serde_json::Value::Array(arr)) => {
                            if let Some(serde_json::Value::String(s)) = arr.first() {
                                s.as_str()
                            } else {
                                "NONE"
                            }
                        },
                        _ => "NONE",
                    };
                    let f_tpl = match entry.get("ex_f_tpl") {
                        Some(serde_json::Value::Array(arr)) => {
                            if let Some(serde_json::Value::String(s)) = arr.first() {
                                s.as_str()
                            } else {
                                "NONE"
                            }
                        },
                        _ => "NONE"
                    };

                    let ctx = Context::from_value(entry.clone()).unwrap();
                    let res2 = tera.render(f_tpl, &ctx).unwrap();

                    let out_path = Path::new("./out").join(f_name);
                    let prefix = out_path.parent().unwrap();
                    std::fs::create_dir_all(prefix).unwrap();
                    let mut out_file = File::create(out_path).unwrap();
                    out_file.write_all(res2.as_bytes()).unwrap();

                    println!("file: {f_name}; tpl: {f_tpl}");
                }

            }
            Err(e) => println!("Could not run code generation query '{query}': {e}"),
        };
    }

    // Compile report
    std::fs::create_dir_all("./out").expect("Could not create out dir");

    let mut output_path = PathBuf::from("./out");
    output_path.push("analysis_report");
    output_path.set_extension("json");

    let file = File::create(&output_path).expect("Could not create output file");
    let writer = BufWriter::new(file);

    // serde_json::to_writer(writer, &report).expect("Could not write JSON to file");
    serde_json::to_writer_pretty(writer, &report).expect("Could not write JSON to file");
}

