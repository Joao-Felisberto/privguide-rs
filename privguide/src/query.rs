use std::io::BufReader;
use std::{collections::HashMap, fs::File};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};


use crate::error::ConversionResult;

#[derive(Debug)]
pub struct Query {
    metadata: QueryMetadata,
    query: String,
}

impl Query {
    pub fn new(metadata: QueryMetadata, query: String) -> Self {
        Query { metadata, query }
    }

    pub fn from_file_path(file: String) -> std::io::Result<Self> {
        let path = Path::new(file.as_str());
        Query::from_file(path)
    }

    pub fn from_file(path: &Path) -> std::io::Result<Self> {
        let query = fs::read_to_string(path)?;
        let metadata = QueryMetadata::from_query(query.as_str());

        Ok(Query { metadata, query })
    }

    pub fn get_query(&self) -> &String {
        &self.query
    }

    pub fn get_metadata(&self) -> &QueryMetadata {
        &self.metadata
    }
}

#[derive(Debug, Default)]
pub struct QueryMetadata {
    key_value_pairs: HashMap<String, String>,
}

impl QueryMetadata {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn from_query(query: &str) -> Self {
        let mut metadata = Self::new();

        let mut in_top_comment = false;
        let mut current_key: Option<String> = None;
        let mut current_value = String::new();

        for line in query.lines() {
            // Skip empty lines before any content
            if line.trim().is_empty() && !in_top_comment {
                continue;
            }

            if line.starts_with('#') {
                if !in_top_comment {
                    in_top_comment = true;
                }

                // Check if this is a continuation line (starts with # followed by 2 or more spaces)
                if line.starts_with("#  ") && current_key.is_some() {
                    // This is a continuation of a multiline value
                    if !current_value.is_empty() {
                        current_value.push(' ');
                    }
                    current_value.push_str(line[3..].trim());
                    continue; // Skip further processing for continuation lines
                }

                let comment_content = line[1..].trim();

                // Check if this line contains a key-value pair
                if let Some(colon_pos) = comment_content.find(':') {
                    let key = comment_content[..colon_pos].trim().to_string();
                    let value = comment_content[colon_pos + 1..].trim();

                    // If we were building a multiline value, save it first
                    if let Some(existing_key) = current_key.take() {
                        metadata
                            .key_value_pairs
                            .insert(existing_key, current_value.trim().to_string());
                        current_value.clear();
                    }

                    if !value.is_empty() {
                        // This line has content after the colon, so it's either:
                        // 1. A single line value, OR
                        // 2. The start of a multiline value that continues on next lines
                        // We'll treat it as the start of a multiline value and set current_key
                        current_key = Some(key);
                        current_value = value.to_string();
                    } else {
                        // Start of a multiline value with no content on this line
                        current_key = Some(key);
                    }
                }
            } else if in_top_comment {
                // We've reached the end of the top comment section
                break;
            } else if !line.trim().is_empty() {
                // We've reached the actual content (not a comment)
                break;
            }
        }

        // Save any pending multiline value
        if let Some(key) = current_key {
            metadata
                .key_value_pairs
                .insert(key, current_value.trim().to_string());
        }

        metadata
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.key_value_pairs.get(key)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AttackTree {
    query: String,
    children: Vec<AttackTree>,
    #[serde(skip_deserializing)]
    query_results: Vec<Vec<(String, String)>>,
    #[serde(skip_deserializing)]
    has_executed: bool,
}

impl AttackTree {
    pub fn from_file(path: &Path) -> ConversionResult<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let mut res: AttackTree = serde_yaml_ng::from_reader(reader)?;
        res.has_executed = false;
        res.query = match path.parent() {
            Some(par) => par.join(res.query).to_str().unwrap().to_string(),
            None => res.query,
        };
        Ok(res)
    }

    pub fn new(query_name: String) -> Self {
        Self {
            query: query_name,
            children: Vec::new(),
            query_results: Vec::new(),
            has_executed: false,
        }
    }

    pub fn add_child(mut self, child: AttackTree) {
        self.children.push(child);
    }

    pub fn get_query(&self) -> &String {
        &self.query
    }

    pub fn get_children(&mut self) -> &mut Vec<AttackTree> {
        &mut self.children
    }

    pub fn is_possible(&self) -> bool {
        self.has_executed && !self.query_results.is_empty()
    }

    pub fn set_executed(&mut self, results: Vec<Vec<(String, String)>>) {
        self.has_executed = true;
        self.query_results = results;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_key_value_pairs() {
        let content = r#"# key: and its value
# some number: 42
# multiline: strings with multiple lines can be encoded in the 
#  value if the comment starts with more than 2 spaces.
SELECT * WHERE { ?s ?p ?o }"#;

        let metadata = QueryMetadata::from_query(content);

        assert_eq!(metadata.get("key"), Some(&"and its value".to_string()));
        assert_eq!(metadata.get("some number"), Some(&"42".to_string()));
        assert_eq!(metadata.get("multiline"), Some(&"strings with multiple lines can be encoded in the value if the comment starts with more than 2 spaces.".to_string()));
        assert_eq!(metadata.key_value_pairs.len(), 3);
    }

    #[test]
    fn test_mixed_comments() {
        let content = r#"# This is a regular comment
# key: value
# Another comment
# second key: another value
SELECT * WHERE { ?s ?p ?o }"#;

        let metadata = QueryMetadata::from_query(content);

        assert_eq!(metadata.get("key"), Some(&"value".to_string()));
        assert_eq!(
            metadata.get("second key"),
            Some(&"another value".to_string())
        );
        assert_eq!(metadata.key_value_pairs.len(), 2);
    }

    #[test]
    fn test_empty_lines() {
        let content = r#"# 
# key: value
#
# another: test
SELECT * WHERE { ?s ?p ?o }"#;

        let metadata = QueryMetadata::from_query(content);

        assert_eq!(metadata.get("key"), Some(&"value".to_string()));
        assert_eq!(metadata.get("another"), Some(&"test".to_string()));
        assert_eq!(metadata.key_value_pairs.len(), 2);
    }

    #[test]
    fn test_multiline_with_multiple_continuations() {
        let content = r#"# description: This is a multi-line
#   description that spans
#   multiple continuation lines
#   and should be concatenated
# tags: sparql, query, test
SELECT * WHERE { ?s ?p ?o }"#;

        let metadata = QueryMetadata::from_query(content);

        assert_eq!(metadata.get("description"), Some(&"This is a multi-line description that spans multiple continuation lines and should be concatenated".to_string()));
        assert_eq!(
            metadata.get("tags"),
            Some(&"sparql, query, test".to_string())
        );
        assert_eq!(metadata.key_value_pairs.len(), 2);
    }

    #[test]
    fn test_no_key_value_pairs() {
        let content = r#"# Just a regular comment
# Another comment
SELECT * WHERE { ?s ?p ?o }"#;

        let metadata = QueryMetadata::from_query(content);

        assert_eq!(metadata.key_value_pairs.len(), 0);
    }

    #[test]
    fn test_exactly_two_spaces() {
        let content = r#"# multiline: first line
#  second line with exactly two spaces
#   third line with three spaces
SELECT * WHERE { ?s ?p ?o }"#;

        let metadata = QueryMetadata::from_query(content);

        assert_eq!(
            metadata.get("multiline"),
            Some(
                &"first line second line with exactly two spaces third line with three spaces"
                    .to_string()
            )
        );
        assert_eq!(metadata.key_value_pairs.len(), 1);
    }

    #[test]
    fn test_multiline_with_gap() {
        let content = r#"# multiline: first part
#  continuation
#
#  another continuation after empty line
SELECT * WHERE { ?s ?p ?o }"#;

        let metadata = QueryMetadata::from_query(content);

        assert_eq!(
            metadata.get("multiline"),
            Some(&"first part continuation another continuation after empty line".to_string())
        );
        assert_eq!(metadata.key_value_pairs.len(), 1);
    }

    #[test]
    fn test_from_file_string() {
        let content = r#"# title: Example Query
# author: John Doe
# description: This query finds all subjects
SELECT ?s WHERE { ?s ?p ?o }"#;

        let metadata = QueryMetadata::from_query(content);

        assert_eq!(metadata.get("title"), Some(&"Example Query".to_string()));
        assert_eq!(metadata.get("author"), Some(&"John Doe".to_string()));
        assert_eq!(
            metadata.get("description"),
            Some(&"This query finds all subjects".to_string())
        );
        assert_eq!(metadata.key_value_pairs.len(), 3);
    }

    #[test]
    fn test_read_attack_trees() {
        let src = r#"
query: attack.rq
children: []
        "#;
        
        let res: AttackTree = serde_yaml_ng::from_str(src).unwrap();
        println!("{res:#?}");
    }
}
