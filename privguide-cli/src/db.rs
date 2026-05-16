use std::{collections::HashMap, hash::Hash};

use privguide::database::{Database, MemDatabase};
use oxigraph::{model::{NamedNode, NamedOrBlankNode, Term}, sparql::QueryTripleIter, store::StorageError};
use serde_json::Value;

use crate::fs;

#[derive(Debug, Hash, PartialEq, Eq)]
pub enum QueryKind {
    Regulation,
    Attack,
    Requirement,
    Reasoner,
    ExtraData,
    SourceCode,
    CodeGenData,
}

pub enum DBKind {
    InMemory{dir: String},
}

pub enum DBInstance {
    MemDatabase(MemDatabase),
}

pub fn create_database<T: Database>(kind: DBKind) -> Result<DBInstance, StorageError> {
    match kind {
        DBKind::InMemory{dir} => {
            let prefix_uri_map = fs::get_prefix_uri_map(dir.clone())?;
            let file_prefixes = fs::get_file_prefix_map(dir)?;
            Ok(DBInstance::MemDatabase(MemDatabase::new(prefix_uri_map, file_prefixes)?))
        }
    }
}


pub fn triples_to_json<T: Database>(db: &T, triples: QueryTripleIter<'_>) -> Value {
    let root_id = Term::NamedNode(NamedNode::new("http://example.com/ROOT").unwrap());
    let prefix_uri_map: HashMap<String, String> = db.get_prefix_map().iter()
        .map(|(k, v)| (v.trim_end_matches("/").to_string(), k.clone()))
        .collect();

    let mut adjacency: HashMap<Term, Vec<(NamedNode, Term)>> = HashMap::new();
    for row in triples {
        let triple = row.unwrap();
        let s = match triple.subject {
            NamedOrBlankNode::NamedNode(nn) => Term::NamedNode(nn),
            NamedOrBlankNode::BlankNode(bn) => Term::BlankNode(bn),
        };
        let p = triple.predicate;
        let o = triple.object;
        adjacency.entry(s).or_default().push((p, o));
    }

    triples_to_json_rec(&prefix_uri_map, &mut adjacency, root_id)   
}

fn triples_to_json_rec(
    prefix_uri_map: &HashMap<String, String>,
    adjacency: &mut HashMap<Term, Vec<(NamedNode, Term)>>,
    key: Term,
) -> Value {
    if let Some(e) = adjacency.remove(&key) {
        let mut obj = serde_json::Map::new();
        for (k, v) in e {
            let os_k = uri_to_json_var(prefix_uri_map, k.clone().into_string())
                .unwrap_or(k.into_string());
            if let Some(Value::Array(arr)) = obj.get_mut(&os_k) {
                arr.push(triples_to_json_rec(prefix_uri_map, adjacency, v));
            } else {
                let arr = Value::Array(vec![triples_to_json_rec(prefix_uri_map, adjacency, v)]);
                obj.insert(os_k, arr);
            }
        }
        Value::Object(obj)
    } else {
        match key {
            Term::BlankNode(bn) => Value::String(bn.into_string()),
            Term::NamedNode(nn) => Value::String(nn.into_string()),
            Term::Literal(lit) => Value::String(lit.value().to_string()),
        }
    }
}

fn uri_to_json_var(prefixes: &HashMap<String, String>, original: String) -> Option<String> {
    let trimmed = original.trim_start_matches('<').trim_end_matches('>');

    if let Some(pos) = trimmed.rfind('/') {
        let (base, id) = trimmed.split_at(pos);
        let id = &id[1..];

        if let Some(value) = prefixes.get(base) {
            return Some(format!("{}_{}", value, id));
        }
    }

    None
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::db::uri_to_json_var;

    #[test]
    fn test_uri_conversion() {
        let prefixes = HashMap::from([
            ("http://example.com".to_string(), "ex".to_string()),
        ]);
        let s = "<http://example.com/prop1>".to_string();
    
        let expected = "ex_prop1".to_string();
        let actual = uri_to_json_var(&prefixes, s).unwrap();
        assert_eq!(expected, actual);
    }
}
