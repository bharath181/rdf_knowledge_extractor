use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info, warn};
use uuid::Uuid;
use std::fs;

use crate::config::RdfSchema;
use crate::core::RdfTriple;

// #[cfg(feature = "oxigraph")]
// pub mod oxigraph_store;
// #[cfg(feature = "oxigraph")]
// pub use oxigraph_store::OxigraphKnowledgeGraph;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SimpleSparqlResults {
    Solutions(Vec<HashMap<String, String>>),
    Boolean(bool),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraphConfig {
    pub storage_path: String,
    pub namespaces: HashMap<String, String>,
    pub default_graph: Option<String>,
}

impl Default for KnowledgeGraphConfig {
    fn default() -> Self {
        Self {
            storage_path: "knowledge_graph.db".to_string(),
            namespaces: HashMap::new(),
            default_graph: None,
        }
    }
}

pub struct KnowledgeGraph {
    triples: Vec<RdfTriple>,
    config: KnowledgeGraphConfig,
    schema: RdfSchema,
}

impl KnowledgeGraph {
    pub fn new(config: KnowledgeGraphConfig, schema: RdfSchema) -> Result<Self> {
        // Load existing triples if file exists
        let triples = if Path::new(&config.storage_path).exists() {
            let content = fs::read_to_string(&config.storage_path)
                .with_context(|| format!("Failed to read knowledge graph file: {}", config.storage_path))?;

            serde_json::from_str(&content)
                .with_context(|| "Failed to parse knowledge graph JSON")?
        } else {
            Vec::new()
        };

        info!("Knowledge graph initialized with {} triples from: {}", triples.len(), config.storage_path);

        Ok(Self {
            triples,
            config,
            schema,
        })
    }

    pub fn in_memory(schema: RdfSchema) -> Result<Self> {
        let config = KnowledgeGraphConfig {
            storage_path: ":memory:".to_string(),
            namespaces: HashMap::new(),
            default_graph: None,
        };

        Ok(Self {
            triples: Vec::new(),
            config,
            schema,
        })
    }

    fn save_to_disk(&self) -> Result<()> {
        if self.config.storage_path != ":memory:" {
            let json = serde_json::to_string_pretty(&self.triples)?;
            fs::write(&self.config.storage_path, json)
                .with_context(|| format!("Failed to save knowledge graph to: {}", self.config.storage_path))?;
        }
        Ok(())
    }

    pub fn add_triples(&mut self, triples: &[RdfTriple]) -> Result<usize> {
        let mut added_count = 0;

        for triple in triples {
            // Simple deduplication check
            let exists = self.triples.iter().any(|existing| {
                existing.subject == triple.subject
                    && existing.predicate == triple.predicate
                    && existing.object == triple.object
            });

            if !exists {
                self.triples.push(triple.clone());
                added_count += 1;
                debug!("Added triple: {}", triple.to_ntriple());
            }
        }

        // Save to disk
        self.save_to_disk()?;

        info!("Added {} triples to knowledge graph", added_count);
        Ok(added_count)
    }

    pub fn execute_sparql(&self, query: &str) -> Result<SimpleSparqlResults> {
        debug!("Executing simplified SPARQL query: {}", query);

        // Simple SPARQL implementation for basic SELECT queries
        if query.trim().to_lowercase().starts_with("select") {
            self.execute_select_query(query)
        } else {
            anyhow::bail!("Only SELECT queries are supported in this simplified implementation");
        }
    }

    fn execute_select_query(&self, query: &str) -> Result<SimpleSparqlResults> {
        // Very basic SPARQL SELECT implementation
        // This is a simplified version that handles basic patterns

        let mut results = Vec::new();

        // Parse basic SELECT queries like "SELECT ?var1 ?var2 WHERE { ?var1 predicate ?var2 }"
        if query.contains("?name") && query.contains("hasName") {
            // Handle name queries
            for triple in &self.triples {
                if triple.predicate.contains("hasName") {
                    let mut row = HashMap::new();
                    row.insert("name".to_string(), triple.object.clone());
                    row.insert("entity".to_string(), triple.subject.clone());
                    results.push(row);
                }
            }
        } else if query.contains("?role") && query.contains("hasRole") {
            // Handle role queries
            for triple in &self.triples {
                if triple.predicate.contains("hasRole") {
                    let mut row = HashMap::new();
                    row.insert("role".to_string(), triple.object.clone());
                    row.insert("person".to_string(), triple.subject.clone());
                    results.push(row);
                }
            }
        } else {
            // Generic query - return all triples as subject/predicate/object
            for triple in &self.triples {
                let mut row = HashMap::new();
                row.insert("subject".to_string(), triple.subject.clone());
                row.insert("predicate".to_string(), triple.predicate.clone());
                row.insert("object".to_string(), triple.object.clone());
                results.push(row);
            }
        }

        Ok(SimpleSparqlResults::Solutions(results))
    }

    pub fn get_entities_by_type(&self, entity_type: &str) -> Result<Vec<String>> {
        let type_uri = if entity_type.starts_with("http") {
            entity_type.to_string()
        } else {
            format!("{}{}", self.schema.namespace, entity_type)
        };

        let mut entities = Vec::new();

        // Look for triples with rdf:type predicate
        for triple in &self.triples {
            if triple.predicate.contains("type") && triple.object == type_uri {
                entities.push(triple.subject.clone());
            }
        }

        Ok(entities)
    }

    pub fn get_entity_properties(&self, entity_uri: &str) -> Result<HashMap<String, Vec<String>>> {
        let mut properties = HashMap::new();

        for triple in &self.triples {
            if triple.subject == entity_uri {
                properties.entry(triple.predicate.clone())
                    .or_insert_with(Vec::new)
                    .push(triple.object.clone());
            }
        }

        Ok(properties)
    }

    pub fn find_related_entities(&self, entity_uri: &str, max_depth: usize) -> Result<Vec<String>> {
        let mut related = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut to_visit = vec![(entity_uri.to_string(), 0)];

        while let Some((current_uri, depth)) = to_visit.pop() {
            if depth >= max_depth || visited.contains(&current_uri) {
                continue;
            }

            visited.insert(current_uri.clone());

            // Find related entities in both directions
            for triple in &self.triples {
                if triple.subject == current_uri {
                    // Object might be a related entity
                    if triple.object.starts_with("http") && !visited.contains(&triple.object) {
                        related.push(triple.object.clone());
                        to_visit.push((triple.object.clone(), depth + 1));
                    }
                } else if triple.object == current_uri && triple.object.starts_with("http") {
                    // Subject is a related entity
                    if !visited.contains(&triple.subject) {
                        related.push(triple.subject.clone());
                        to_visit.push((triple.subject.clone(), depth + 1));
                    }
                }
            }
        }

        Ok(related)
    }

    pub fn get_statistics(&self) -> Result<KnowledgeGraphStats> {
        let total_triples = self.triples.len();

        let mut unique_subjects = std::collections::HashSet::new();
        let mut unique_predicates = std::collections::HashSet::new();
        let mut unique_objects = std::collections::HashSet::new();

        for triple in &self.triples {
            unique_subjects.insert(&triple.subject);
            unique_predicates.insert(&triple.predicate);
            unique_objects.insert(&triple.object);
        }

        Ok(KnowledgeGraphStats {
            total_triples,
            unique_subjects: unique_subjects.len(),
            unique_predicates: unique_predicates.len(),
            unique_objects: unique_objects.len(),
        })
    }

    fn format_triple_as_ntriple(&self, triple: &RdfTriple) -> String {
        let subject = if triple.subject.starts_with("http") {
            format!("<{}>", triple.subject)
        } else {
            format!("\"{}\"", triple.subject)
        };

        let predicate = format!("<{}>", triple.predicate);

        let object = if triple.object.starts_with("http") {
            format!("<{}>", triple.object)
        } else {
            format!("\"{}\"", triple.object)
        };

        format!("{} {} {} .", subject, predicate, object)
    }

    pub fn export_to_file(&self, file_path: &str, format: &str) -> Result<()> {
        use std::fs::File;
        use std::io::Write;

        let mut file = File::create(file_path)
            .with_context(|| format!("Failed to create export file: {}", file_path))?;

        match format.to_lowercase().as_str() {
            "turtle" | "ttl" => {
                // Write turtle format with prefixes
                file.write_all(b"@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n")?;
                file.write_all(b"@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n")?;
                file.write_all(format!("@prefix {}: <{}> .\n\n", self.schema.prefix, self.schema.namespace).as_bytes())?;

                for triple in &self.triples {
                    let turtle_line = format!("{} {} {} .\n",
                        self.format_uri_or_literal(&triple.subject, true),
                        self.format_uri_or_literal(&triple.predicate, true),
                        self.format_uri_or_literal(&triple.object, false)
                    );
                    file.write_all(turtle_line.as_bytes())?;
                }
            }
            "ntriples" | "nt" => {
                for triple in &self.triples {
                    let ntriple = format!("{}\n", self.format_triple_as_ntriple(triple));
                    file.write_all(ntriple.as_bytes())?;
                }
            }
            "json" => {
                let json = serde_json::to_string_pretty(&self.triples)?;
                file.write_all(json.as_bytes())?;
            }
            _ => {
                anyhow::bail!("Unsupported export format: {}. Supported: turtle, ntriples, json", format);
            }
        }

        info!("Knowledge graph exported to: {} (format: {})", file_path, format);
        Ok(())
    }

    fn format_uri_or_literal(&self, value: &str, is_uri_context: bool) -> String {
        if value.starts_with("http") {
            // Try to use prefix if available
            if value.starts_with(&self.schema.namespace) {
                let local_name = &value[self.schema.namespace.len()..];
                format!("{}:{}", self.schema.prefix, local_name)
            } else {
                format!("<{}>", value)
            }
        } else if is_uri_context {
            // For subjects/predicates that should be URIs but aren't, wrap in quotes
            format!("\"{}\"", value)
        } else {
            // For objects, treat as literal
            format!("\"{}\"", value)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraphStats {
    pub total_triples: usize,
    pub unique_subjects: usize,
    pub unique_predicates: usize,
    pub unique_objects: usize,
}

impl std::fmt::Display for KnowledgeGraphStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "Knowledge Graph Statistics:\n\
             Total Triples: {}\n\
             Unique Subjects: {}\n\
             Unique Predicates: {}\n\
             Unique Objects: {}",
            self.total_triples,
            self.unique_subjects,
            self.unique_predicates,
            self.unique_objects
        )
    }
}