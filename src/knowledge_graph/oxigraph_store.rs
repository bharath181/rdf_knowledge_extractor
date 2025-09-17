use anyhow::{Result, Context};
use oxigraph::store::Store;
use oxigraph::model::*;
use oxigraph::sparql::QueryResults;
use oxigraph::io::{RdfFormat, RdfParser, RdfSerializer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufReader, BufWriter};
use tracing::{debug, info};

use crate::config::RdfSchema;
use crate::core::RdfTriple;

#[derive(Clone, Serialize, Deserialize)]
pub struct OxigraphKnowledgeGraph {
    #[serde(skip)]
    store: Option<Store>,
    storage_path: String,
    schema: RdfSchema,
}

impl OxigraphKnowledgeGraph {
    pub fn new(storage_path: String, schema: RdfSchema) -> Result<Self> {
        let store = if storage_path == ":memory:" {
            Store::new()?
        } else {
            Store::open(&storage_path)?
        };

        info!("Oxigraph store initialized at: {}", storage_path);

        Ok(Self {
            store: Some(store),
            storage_path,
            schema,
        })
    }

    pub fn add_triple(&mut self, triple: &RdfTriple) -> Result<()> {
        let store = self.store.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Store not initialized"))?;

        // Create subject
        let subject = if triple.subject.starts_with("http") {
            Subject::from(NamedNode::new(&triple.subject)?)
        } else {
            Subject::from(BlankNode::new(&triple.subject)?)
        };

        // Create predicate
        let predicate = NamedNode::new(&triple.predicate)?;

        // Create object
        let object = if triple.object.starts_with("http") {
            Term::from(NamedNode::new(&triple.object)?)
        } else {
            Term::from(Literal::new_simple_literal(&triple.object))
        };

        // Create quad (triple with optional graph)
        let quad = Quad::new(subject, predicate, object, GraphName::DefaultGraph);

        store.insert(&quad)?;
        debug!("Added triple to Oxigraph: {}", triple.to_ntriple());

        Ok(())
    }

    pub fn add_triples(&mut self, triples: &[RdfTriple]) -> Result<usize> {
        let mut count = 0;
        for triple in triples {
            self.add_triple(triple)?;
            count += 1;
        }
        info!("Added {} triples to Oxigraph store", count);
        Ok(count)
    }

    pub fn execute_sparql(&self, query: &str) -> Result<SimpleSparqlResults> {
        let store = self.store.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Store not initialized"))?;

        debug!("Executing SPARQL query: {}", query);

        let results = store.query(query)?;

        match results {
            QueryResults::Solutions(solutions) => {
                let mut rows = Vec::new();

                for solution in solutions {
                    let solution = solution?;
                    let mut row = HashMap::new();

                    for (var, term) in solution.iter() {
                        let value = match term {
                            Term::NamedNode(n) => n.as_str().to_string(),
                            Term::BlankNode(b) => b.as_str().to_string(),
                            Term::Literal(l) => l.value().to_string(),
                            Term::Triple(t) => format!("{:?}", t),
                        };
                        row.insert(var.as_str().to_string(), value);
                    }

                    rows.push(row);
                }

                Ok(SimpleSparqlResults::Solutions(rows))
            }
            QueryResults::Boolean(result) => {
                Ok(SimpleSparqlResults::Boolean(result))
            }
            _ => {
                anyhow::bail!("Unsupported query result type")
            }
        }
    }

    pub fn load_from_ntriples(&mut self, file_path: &str) -> Result<()> {
        let store = self.store.as_mut()
            .ok_or_else(|| anyhow::anyhow!("Store not initialized"))?;

        let file = std::fs::File::open(file_path)
            .with_context(|| format!("Failed to open file: {}", file_path))?;

        store.load_from_read(
            oxigraph::io::RdfFormat::NTriples,
            file,
            None,
            None,
        )?;

        info!("Loaded N-Triples from: {}", file_path);
        Ok(())
    }

    pub fn export_to_ntriples(&self, file_path: &str) -> Result<()> {
        let store = self.store.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Store not initialized"))?;

        let file = std::fs::File::create(file_path)
            .with_context(|| format!("Failed to create file: {}", file_path))?;

        store.dump_to_write(
            oxigraph::io::RdfFormat::NTriples,
            file,
        )?;

        info!("Exported to N-Triples: {}", file_path);
        Ok(())
    }

    pub fn count_triples(&self) -> Result<usize> {
        let store = self.store.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Store not initialized"))?;

        Ok(store.len()?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SimpleSparqlResults {
    Solutions(Vec<HashMap<String, String>>),
    Boolean(bool),
}