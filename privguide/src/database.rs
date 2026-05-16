use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::LazyLock;

use oxigraph::model::BlankNode;
use oxigraph::model::GraphName;
use oxigraph::model::Literal;
use oxigraph::model::NamedNode;
use oxigraph::model::NamedOrBlankNode;
use oxigraph::model::Quad;
use oxigraph::model::Term;
use oxigraph::sparql::QueryTripleIter;
use oxigraph::sparql::{QueryResults, SparqlEvaluator};
use oxigraph::store::{StorageError, Store};

use regex::Regex;
use serde_json::Value;

use tree_sitter::Node;
use tree_sitter::Tree;

use crate::error::DatabaseLoadError;
use crate::error::ExecuteQueryError;
use crate::query::{AttackTree, Query};

// https://docs.rs/oxigraph/latest/oxigraph/
static IRI_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([a-zA-Z]+):(.*)").unwrap());

struct Frame<'a> {
    node: Node<'a>,
    subject: NamedOrBlankNode,
}

// pub type QueryResultsMap = Vec<Vec<(String, String)>>;
pub type QueryResultsMap = Vec<HashMap<String, String>>;

pub trait Database {
    fn execute_query(&self, query_k: &str) -> Result<QueryResultsMap, ExecuteQueryError>;

    fn execute_graph_query(&self, query_k: &str) -> Result<QueryTripleIter<'_>, ExecuteQueryError>;

    fn execute_update(&self, query_k: &str) -> Result<(), ExecuteQueryError>;

    fn load_query(&mut self, file: String, query: Query);

    fn run_attack_tree(&self, tree: &mut AttackTree) -> Result<(), ExecuteQueryError>;

    fn load_file_data(
        &mut self,
        value: &Value,
        default_predicate: &str,
    ) -> Result<(), DatabaseLoadError>;

    fn load_source_code(
        &mut self,
        tree: &Tree,
        source: &str,
    ) -> Result<(), DatabaseLoadError>;

    fn get_prefix_map(&self) -> &HashMap<String, String>;

    fn get_file_prefixes(&self) -> &HashMap<String, String>;
}

pub struct MemDatabase {
    store: Store,
    prefixes: HashMap<String, String>,
    file_prefixes: HashMap<String, String>,
    queries: HashMap<String, Query>,
}

impl MemDatabase {
    pub fn new(prefixes: HashMap<String, String>, file_prefixes: HashMap<String, String>) -> Result<Self, StorageError> {
        let store = Store::new()?;
        Ok(MemDatabase {
            store,
            prefixes,
            file_prefixes,
            queries: HashMap::new(),
        })
    }

    fn rdf_object_from_value(&self, o: &Value) -> Result<Term, DatabaseLoadError> {
        match o {
            Value::String(s) => {
                if let Some((_, [prefix, id])) = IRI_REGEX.captures(s).map(|caps| caps.extract()) {
                    let prefix = &self
                        .prefixes
                        .get(prefix)
                        .ok_or(DatabaseLoadError::PrefixNotFoundError(prefix.to_string()))?;
                    Ok(Term::NamedNode(NamedNode::new(format!("{prefix}/{id}"))?))
                } else {
                    Ok(Term::Literal(Literal::new_typed_literal(
                        s,
                        NamedNode::new("http://www.w3.org/2001/XMLSchema#string")?,
                    )))
                }
            }
            Value::Number(n) => {
                if n.is_f64() {
                    Ok(Term::Literal(Literal::new_typed_literal(
                        format!("{n}"),
                        NamedNode::new("http://www.w3.org/2001/XMLSchema#decimal")?,
                    )))
                } else {
                    Ok(Term::Literal(Literal::new_typed_literal(
                        format!("{n}"),
                        NamedNode::new("http://www.w3.org/2001/XMLSchema#integer")?,
                    )))
                }
            }
            Value::Bool(b) => Ok(Term::Literal(Literal::new_typed_literal(
                format!("{b}"),
                NamedNode::new("http://www.w3.org/2001/XMLSchema#boolean")?,
            ))),
            _ => {
                println!("Failed to parse {o:#?}");
                Err(DatabaseLoadError::UnparseableObjectError)
            },
        }
    }

}

impl Database for MemDatabase {
    fn execute_query(&self, query_k: &str) -> Result<QueryResultsMap, ExecuteQueryError> {
        let query = self
            .queries
            .get(query_k)
            .ok_or(ExecuteQueryError::QueryNotFound(query_k.to_string()))?;

        let query_results = SparqlEvaluator::new()
            .parse_query(&query.get_query())?
            .on_store(&self.store)
            .execute()?;

        if let QueryResults::Solutions(solutions) = query_results {
            Ok(solutions.into_iter()
                .filter_map(|sol| sol.ok()) // maybe do better error handling?
                .map(|sol| sol.iter()
                    .map(|pair| (
                        pair.0.to_string(),
                        term_to_string(pair.1),
                    )).collect()
                ).collect())
        } else if let QueryResults::Graph(g) = query_results { 
            Ok(g.into_iter()
                .filter_map(|triple| triple.ok()) // maybe do better error handling?
                .map(|triple| {
                    HashMap::from([
                        ("?s".to_string(), triple.subject.to_string()),
                        ("?p".to_string(), triple.predicate.to_string()),
                        ("?o".to_string(), triple.object.to_string()),
                    ])
                }).collect())
        } else {
            Ok(Vec::new()) // todo: there are more options that might be useful to implement,
                           // check https://docs.rs/oxigraph/latest/oxigraph/sparql/enum.QueryResults.html
        }
    }

    fn execute_graph_query(&self, query_k: &str) -> Result<QueryTripleIter<'_>, ExecuteQueryError> {
        let query = self
            .queries
            .get(query_k)
            .ok_or(ExecuteQueryError::QueryNotFound(query_k.to_string()))?;

        let query_results = SparqlEvaluator::new()
            .parse_query(&query.get_query())?
            .on_store(&self.store)
            .execute()?;

        match query_results {
            QueryResults::Graph(g) => Ok(g),
            _ => Err(ExecuteQueryError::WrongQueryType(query_k.to_string(), "Describe,Construct".to_string())),
        }
    }

    fn execute_update(&self, query_k: &str) -> Result<(), ExecuteQueryError> {
        let query = self
            .queries
            .get(query_k)
            .ok_or(ExecuteQueryError::QueryNotFound(query_k.to_string()))?;

        SparqlEvaluator::new()
            .parse_update(&query.get_query())?
            .on_store(&self.store)
            .execute()?;

        Ok(())
    }

    fn load_query(&mut self, file: String, query: Query) {
        self.queries.insert(file, query);
    }

    fn run_attack_tree(&self, tree: &mut AttackTree) -> Result<(), ExecuteQueryError> {
        let mut possible = false;
        for child in tree.get_children() {
            self.run_attack_tree(child)?;
            if child.is_possible() {
                possible = true;
                break;
            }
        }

        if !possible && !tree.get_children().is_empty() {
            return Ok(());
        }

        let res = self.execute_query(tree.get_query())?;
        tree.set_executed(res);
        Ok(())
    }

    fn load_file_data(
        &mut self,
        value: &Value,
        default_url: &str,
    ) -> Result<(), DatabaseLoadError> {
        let mut quads = Vec::new();
        let mut stack = VecDeque::new();

        let default_url = self
            .prefixes
            .get(default_url)
            .ok_or(DatabaseLoadError::PrefixNotFoundError(default_url.into()))?;

        stack.push_back((
            NamedOrBlankNode::NamedNode(NamedNode::new(format!("{default_url}/ROOT"))?),
            value,
        ));

        while let Some((current_subject, current_value)) = stack.pop_back() {
            match current_value {
                Value::Object(map) => {
                    for (key, val) in map.iter().rev() {
                        if key == "id" {
                            continue;
                        }

                        let predicate = NamedNode::new(format!("{default_url}/{key}"))?;

                        match val {
                            Value::Object(nested_obj) => {
                                let new_subject = match nested_obj.get("id") {
                                    Some(id) => match id {
                                        Value::String(s) => Ok(Term::NamedNode(NamedNode::new(format!("{default_url}/{s}"))?)),
                                        _ => {
                                            println!("Failed to parse {val:#?}");
                                            Err(DatabaseLoadError::UnparseableObjectError)
                                        },
                                    },
                                    None => {
                                        Ok(Term::BlankNode(BlankNode::default()))
                                    }
                                }?;

                                quads.push(Quad {
                                    subject: current_subject.clone(),
                                    predicate,
                                    object: new_subject.clone(),
                                    graph_name: GraphName::DefaultGraph,
                                });
                                if let Some(sub) = subject_from_term(new_subject) {
                                    stack.push_back((sub, val));
                                }
                            }
                            Value::Array(arr) => {
                                for item in arr.iter().rev() {
                                    if let Value::Object(nested_obj) = item {
                                        let new_subject = match nested_obj.get("id") {
                                            
                                            Some(id) => match id {
                                                Value::String(s) => Ok(Term::NamedNode(NamedNode::new(format!("{default_url}/{s}"))?)),
                                                _ => Err(DatabaseLoadError::UnparseableObjectError),
                                            },
                                            
                                            // Some(id) => self.rdf_object_from_value(id),
                                            None => {
                                                Ok(Term::BlankNode(BlankNode::default()))
                                            }
                                        }?;

                                        
                                        quads.push(Quad {
                                            subject: current_subject.clone(),
                                            predicate: predicate.clone(),
                                            object: new_subject.clone(),
                                            graph_name: GraphName::DefaultGraph,
                                        });
                                        
                                        if let Some(sub) = subject_from_term(new_subject) {
                                            stack.push_back((sub, item));
                                        }
                                        
                                    } else {
                                        quads.push(Quad {
                                            subject: current_subject.clone(),
                                            predicate: predicate.clone(),
                                            object: self.rdf_object_from_value(item)?,
                                            graph_name: GraphName::DefaultGraph,
                                        });
                                    }
                                }
                            }
                            _ => {
                                quads.push(Quad {
                                    subject: current_subject.clone(),
                                    predicate,
                                    object: self.rdf_object_from_value(val)?,
                                    graph_name: GraphName::DefaultGraph,
                                });
                            }
                        }
                    }
                }
                Value::Array(arr) => {
                    for item in arr.iter().rev() {
                        stack.push_back((current_subject.clone(), item));
                    }
                }
                _ => {
                    quads.push(Quad {
                        subject: current_subject,
                        predicate: NamedNode::new(format!("{default_url}/value"))?,
                        object: self.rdf_object_from_value(current_value)?,
                        graph_name: GraphName::DefaultGraph,
                    });
                }
            }
        }

        // add quads here
        let mut bulk_loader = self.store.bulk_loader();
        bulk_loader.load_quads(quads)?;
        bulk_loader.commit()?;
        Ok(())
    }

    fn get_prefix_map(&self) -> &HashMap<String, String> {
        &self.prefixes
    }

    fn get_file_prefixes(&self) -> &HashMap<String, String> {
        &self.file_prefixes
    }


    fn load_source_code(
        &mut self,
        tree: &Tree,
        source: &str
    ) -> Result<(), DatabaseLoadError> {
        let mut quads = Vec::new();
        let mut stack = Vec::new();
        
        let root = tree.root_node();

        let o = BlankNode::default();
        stack.push(Frame {
            node: root,
            subject: NamedOrBlankNode::BlankNode(o.clone()),
        });

        quads.push(Quad {
            subject: NamedOrBlankNode::NamedNode(NamedNode::new("http://exmaple.com/ROOT")?),
            predicate: NamedNode::new("http://example.com/".to_owned() + root.kind())?,
            object: Term::BlankNode(o),
            graph_name: GraphName::DefaultGraph,
        });

        while let Some(frame) = stack.pop() {

            let node = frame.node;
            let subject = frame.subject;

            let child_count: u32 = node.named_child_count().try_into().unwrap();

            for i in (0..child_count).rev() {

                let child = node.named_child(i).unwrap();

                let field = node
                    .field_name_for_named_child(i)
                    .unwrap_or(child.kind());

                let child_kind = child.kind();

                if child.child_count() == 0 {

                    let text = &source[child.byte_range()];

                    quads.push(Quad { 
                        subject: subject.clone(), 
                        predicate: NamedNode::new("http://example.com/".to_owned() + field)?, 
                        object: Term::Literal(format!("\"{text}\"").into()), 
                        graph_name: GraphName::DefaultGraph,
                    });

                } else {

                    let o = BlankNode::default();
                    quads.push(Quad { 
                        subject: subject.clone(),
                        predicate: NamedNode::new("http://example.com/".to_owned() + child_kind)?,
                        object: Term::BlankNode(o.clone()), // Term::BlankNode(BlankNode::default())
                        graph_name: GraphName::DefaultGraph,
                    });

                    stack.push(Frame {
                        node: child,
                        subject: NamedOrBlankNode::BlankNode(o),
                    });
                }
            }
        }

        let mut bulk_loader = self.store.bulk_loader();
        bulk_loader.load_quads(quads)?;
        bulk_loader.commit()?;

        Ok(())
    }
}

fn subject_from_term(term: Term) -> Option<NamedOrBlankNode> {
    match term {
        Term::NamedNode(nn) => Some(NamedOrBlankNode::NamedNode(nn)),
        Term::BlankNode(bn) => Some(NamedOrBlankNode::BlankNode(bn)),
        Term::Literal(_) => None,
    }
}

fn term_to_string(term: &Term) -> String {
    match term {
        Term::NamedNode(nn) => nn.to_string(),
        Term::BlankNode(bn) => bn.to_string(),
        Term::Literal(l) => l.value().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use oxigraph::io::{RdfFormat, RdfParseError, RdfParser};
    use pretty_assertions::assert_eq;
    use std::ffi::OsStr;
    use std::fs::{self, File};
    use std::io::BufReader;
    use std::path::Path;
    use thiserror::Error;

    #[derive(Error, Debug)]
    #[allow(clippy::enum_variant_names)]
    enum TestError {
        #[error("Could not read the YAML content of the file: {0}")]
        YAMLError(#[from] serde_yaml_ng::Error),
        #[error("Could not load data into database: {0}")]
        DBUseError(#[from] DatabaseLoadError),
        #[error("Could not create database: {0}")]
        DBCreateError(#[from] StorageError),
        #[error("Could not open file: {0}")]
        IOError(#[from] std::io::Error),
        #[error("Could not parse RDF from reference file: {0}")]
        RdfParseError(#[from] RdfParseError),
    }

    #[test]
    fn run_all_file_tests() -> Result<(), TestError> {
        let cfg_dir = "/home/me/projects/oxigraph/privguide/privguide/parse_test/";
        let test_files = find_test_files(cfg_dir)?;

        println!("Tests: {:?}", test_files);
        for test_name in test_files {
            println!("===================================== {test_name}");
            
            let input_file = format!("{}.yml", test_name);
            let expected_file = format!("{}.ttl", test_name);

            let prefixes = HashMap::from([("ex".to_string(), "http://example.com".to_string())]);
            let mut db = MemDatabase::new(prefixes, HashMap::new())?;

            let data = serde_yaml_ng::from_str(&fs::read_to_string(&input_file)?)?;
            db.load_file_data(&data, "ex")?;

            let actual: Vec<Quad> = db.store.iter().map(|e| e.unwrap()).collect();
            let expected: Vec<Quad> = RdfParser::from_format(RdfFormat::Turtle)
                .for_reader(BufReader::new(File::open(expected_file)?))
                .collect::<Result<Vec<Quad>, _>>()?;

            let mut actual: Vec<_> = actual.iter().map(simple_quads).collect();
            let mut expected: Vec<_> = expected.iter().map(simple_quads).collect();
            actual.sort();
            expected.sort();

            assert_eq!(&actual, &expected, "Test '{}' failed!", test_name);
        }
        Ok(())
    }

    fn simple_quads(quad: &Quad) -> String {
        let mut res = String::new();

        res.push_str(quad.subject.to_string().as_str());
        res.push(' ');
        res.push_str(quad.predicate.to_string().as_str());
        res.push(' ');
        res.push_str(quad.object.to_string().as_str());
        
        res
    }

    fn find_test_files(cfg_dir: &str) -> Result<Vec<String>, std::io::Error> {
        let input_ext: [&OsStr; 3] = [OsStr::new("yml"), OsStr::new("yaml"), OsStr::new("json")];
        let output_ext: [&OsStr; 1] = [OsStr::new("ttl")];
        let mut test_files = Vec::new();

        let entries = fs::read_dir(cfg_dir)?;
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(extension) = path.extension() &&
                input_ext.contains(&extension) &&
                let Some(stem) = path.file_stem() {
                    let stem_str = &stem.to_string_lossy().to_string();
                    for ext in output_ext {
                        let expected_path = format!(
                            "{}.{}",
                            Path::new(cfg_dir).join(stem_str).to_str().unwrap(),
                            ext.to_string_lossy()
                        );
                        let expected_path = Path::new(&expected_path);
                        if expected_path.exists() {
                            test_files.push(
                                Path::new(cfg_dir)
                                    .join(stem_str)
                                    .to_str()
                                    .unwrap()
                                    .to_string(),
                            );
                            break;
                        }
                    }
                }
        }

        Ok(test_files)
    }
}

