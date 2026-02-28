use std::{collections::HashMap, sync::Arc};

use privguide::query::AttackTree;
use serde::Serialize;

type QueryResults = Vec<Vec<(String, String)>>;

#[derive(Debug, Default, Serialize)]
pub struct Report {
    violations: HashMap<String, QueryResults>,
    attack_trees: Vec<AttackTree>,
}

impl Report {
    pub fn add_violations(&mut self, rule: String, violations: QueryResults) {
        self.violations.insert(rule, violations);
    }

    pub fn add_attack_trees(&mut self, attack_trees: Vec<AttackTree>) {
        self.attack_trees.extend(attack_trees);
    }

    pub fn add_attack_tree(&mut self, attack_tree: AttackTree) {
        self.attack_trees.push(attack_tree);
    }
}

