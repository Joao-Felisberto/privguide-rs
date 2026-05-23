use std::collections::HashMap;

use eframe::{App, CreationContext, NativeOptions, run_native};
use egui_graphs::{DefaultEdgeShape, DefaultNodeShape, Graph, GraphView, LayoutHierarchical, LayoutStateHierarchical, SettingsNavigation, SettingsStyle};
use petgraph::{Directed, csr::DefaultIx, stable_graph::StableGraph};
use oxigraph::model::{NamedNode, NamedOrBlankNode, Term};

use privguide::{code_analysis::CodeAnalyser, database::{Database, MemDatabase}};
use privguide_cli::{db::{self, DBKind}, fs};

pub struct BasicCustomApp {
    g: Graph,
}

impl BasicCustomApp {
    fn new(_: &CreationContext<'_>) -> Self {
        let g = generate_graph();
        Self{ g }
    }
}

fn generate_graph() -> Graph<(), ()> {
    let mut g = Graph::new(StableGraph::default());

    let db = get_triples_from_codebase(".privguide", ".privguide/src/");
    let adjacency = generate_adjacency_table(&db);
    let mut node_idxs = HashMap::new();

    for (node, _) in adjacency.iter() {
        let n = g.add_node_with_label((), node.to_string());
        node_idxs.insert(node, n);
    }
    for (node, edge_next_list) in adjacency.iter() {
        let src = node_idxs[&node];
        for (edge, next) in edge_next_list {
            // let dest = node_idxs[&next];
            // let dest = node_idxs.get(next).take().unwrap_or_else(|| g.add_node_with_label((), dn));
            let dest = if node_idxs.contains_key(next) {
                node_idxs.get(next).unwrap().to_owned()
            } else {
                g.add_node_with_label((), next.to_string())
            };

            g.add_edge_with_label(src, dest, (), edge.clone().to_string());
        }
    }

    g
}

fn get_triples_from_codebase(dir: &str, code_dir: &str) -> MemDatabase {
    let mut db: MemDatabase = match db::create_database::<MemDatabase>(DBKind::InMemory{dir: dir.to_string()}) {
        Ok(db::DBInstance::MemDatabase(db)) => db,
        Err(e) => {
            panic!("Error creating database: {}", e);
        }
    };

    let mut code_analyser = CodeAnalyser::default();

    match fs::load_languages(dir) {
        Ok(languages) => {
            for l in languages {
                if let Err(e) = code_analyser.add_language(l) {
                    println!("Could not load language: {e}");
                }
            }
        },
        Err(e) => {
            panic!("Error loading grammars: {}", e);
        }
    };

    if let Err(e) = fs::load_source_code_files(code_dir, &mut db, &mut code_analyser){
        panic!("Error loading source code files: {e}");
    };

    db
}

fn generate_adjacency_table<T: Database>(db: &T) -> HashMap<Term, Vec<(NamedNode, Term)>> {
    let mut adjacency: HashMap<Term, Vec<(NamedNode, Term)>> = HashMap::new();
    for row in db.triples() {
        let triple = row.unwrap();
        let s = match triple.subject {
            NamedOrBlankNode::NamedNode(nn) => Term::NamedNode(nn),
            NamedOrBlankNode::BlankNode(bn) => Term::BlankNode(bn),
        };
        let p = triple.predicate;
        let o = triple.object;
        adjacency.entry(s).or_default().push((p, o));
    }

    adjacency
}

impl App for BasicCustomApp {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.add(
                &mut GraphView::<
                    (), 
                    (), 
                    Directed, 
                    DefaultIx, 
                    DefaultNodeShape, 
                    DefaultEdgeShape, 
                    LayoutStateHierarchical, 
                    LayoutHierarchical
                >::new(&mut self.g)
                    .with_styles(&SettingsStyle::default().with_labels_always(true))
                    .with_navigations(&SettingsNavigation::new()
                        .with_zoom_and_pan_enabled(true)
                        .with_fit_to_screen_enabled(false)
                    )
            );
        });
    }
}

fn main() {
    let native_options = NativeOptions::default();
    run_native(
        "basic_custom",
        native_options,
        Box::new(|cc| Ok(Box::new(BasicCustomApp::new(cc)))),
    )
    .unwrap();
}
