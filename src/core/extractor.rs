use anyhow::{Result, Context};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::config::Configuration;
use crate::handlers::DocumentProcessor;
use crate::core::llm_client::{VllmClient, PromptBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RdfTriple {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    #[serde(default = "default_confidence")]
    pub confidence: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

fn default_confidence() -> f32 { 1.0 }

impl RdfTriple {
    pub fn new(subject: String, predicate: String, object: String) -> Self {
        Self {
            subject,
            predicate,
            object,
            confidence: 1.0,
            source: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_source(mut self, source: String) -> Self {
        self.source = Some(source);
        self
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence;
        self
    }

    pub fn to_ntriple(&self) -> String {
        let object = if self.object.starts_with("http://") || self.object.starts_with("https://") {
            format!("<{}>", self.object)
        } else {
            format!("\"{}\"", self.object.replace("\"", "\\\""))
        };
        format!("<{}> <{}> {} .", self.subject, self.predicate, object)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    pub id: String,
    pub triples: Vec<RdfTriple>,
    pub document_source: String,
    pub extraction_timestamp: DateTime<Utc>,
    pub processing_time_seconds: f64,
    pub metadata: HashMap<String, String>,
    #[serde(default)]
    pub errors: Vec<String>,
    pub config_name: String,
}

impl ExtractionResult {
    pub fn new(
        document_source: String,
        config_name: String,
        processing_time_seconds: f64,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            triples: Vec::new(),
            document_source,
            extraction_timestamp: Utc::now(),
            processing_time_seconds,
            metadata: HashMap::new(),
            errors: Vec::new(),
            config_name,
        }
    }

    pub fn with_triples(mut self, triples: Vec<RdfTriple>) -> Self {
        self.triples = triples;
        self
    }

    pub fn with_error(mut self, error: String) -> Self {
        self.errors.push(error);
        self
    }

    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }
}

pub struct RdfExtractor {
    config: Configuration,
    llm_client: VllmClient,
    document_processor: DocumentProcessor,
}

impl RdfExtractor {
    pub fn new(config: Configuration, llm_client: VllmClient) -> Self {
        Self {
            config,
            llm_client,
            document_processor: DocumentProcessor::new(),
        }
    }

    pub async fn extract_from_document(&self, source: &str) -> Result<ExtractionResult> {
        let start_time = Instant::now();

        info!("Starting extraction from document: {}", source);

        // Process document
        let processed_doc = match self.document_processor.process(source).await {
            Ok(doc) => doc,
            Err(e) => {
                let error_msg = format!("Failed to process document: {}", e);
                warn!("{}", error_msg);
                let processing_time = start_time.elapsed().as_secs_f64();
                return Ok(ExtractionResult::new(
                    source.to_string(),
                    self.config.name.clone(),
                    processing_time,
                ).with_error(error_msg));
            }
        };

        debug!("Document processed, text length: {}", processed_doc.text.len());

        // Build extraction prompt
        let prompt = PromptBuilder::build_extraction_prompt(
            &processed_doc.text,
            &self.config.extraction_questions,
            &self.config.rdf_schema,
        );

        // Extract with LLM
        let llm_response = match self.llm_client
            .generate_structured(&prompt, Some(PromptBuilder::get_system_prompt()))
            .await {
            Ok(response) => response,
            Err(e) => {
                let error_msg = format!("LLM extraction failed: {}", e);
                warn!("{}", error_msg);
                let processing_time = start_time.elapsed().as_secs_f64();
                return Ok(ExtractionResult::new(
                    source.to_string(),
                    self.config.name.clone(),
                    processing_time,
                ).with_error(error_msg));
            }
        };

        debug!("LLM response received: {:?}", llm_response);

        // Parse triples from LLM response
        let triples = self.parse_llm_response(&llm_response, source)?;

        // Apply post-processing
        let processed_triples = self.post_process_triples(triples);

        let processing_time = start_time.elapsed().as_secs_f64();

        // Build metadata
        let mut metadata = processed_doc.metadata;
        metadata.insert("extraction_config".to_string(), self.config.name.clone());
        metadata.insert("llm_model".to_string(), self.llm_client.model.clone());
        metadata.insert("num_questions".to_string(), self.config.extraction_questions.len().to_string());

        info!(
            "Extraction completed: {} triples extracted in {:.2}s",
            processed_triples.len(),
            processing_time
        );

        Ok(ExtractionResult::new(
            source.to_string(),
            self.config.name.clone(),
            processing_time,
        )
        .with_triples(processed_triples)
        .with_metadata(metadata))
    }

    pub async fn extract_from_multiple(&self, sources: Vec<String>) -> Result<Vec<ExtractionResult>> {
        let mut results = Vec::new();

        for source in sources {
            let result = self.extract_from_document(&source).await?;
            results.push(result);
        }

        Ok(results)
    }

    pub fn merge_results(&self, results: Vec<ExtractionResult>) -> Result<ExtractionResult> {
        if results.is_empty() {
            anyhow::bail!("Cannot merge empty results");
        }

        let mut all_triples = Vec::new();
        let mut all_errors = Vec::new();
        let mut total_time = 0.0;
        let mut sources = Vec::new();

        for result in &results {
            all_triples.extend(result.triples.clone());
            all_errors.extend(result.errors.clone());
            total_time += result.processing_time_seconds;
            sources.push(result.document_source.clone());
        }

        // Deduplicate triples if enabled
        if self.config.post_processing.deduplicate {
            all_triples = self.deduplicate_triples(all_triples);
        }

        let mut metadata = HashMap::new();
        metadata.insert("source_count".to_string(), results.len().to_string());
        metadata.insert("sources".to_string(), sources.join(", "));
        metadata.insert("total_triples".to_string(), all_triples.len().to_string());

        Ok(ExtractionResult::new(
            "merged".to_string(),
            self.config.name.clone(),
            total_time,
        )
        .with_triples(all_triples)
        .with_metadata(metadata))
    }

    fn parse_llm_response(&self, response: &serde_json::Value, source: &str) -> Result<Vec<RdfTriple>> {
        let triples_array = if response.is_array() {
            response.as_array().unwrap()
        } else if let Some(triples) = response.get("triples") {
            triples.as_array().context("'triples' field is not an array")?
        } else {
            // Try to find any array in the response
            return Ok(Vec::new());
        };

        let mut triples = Vec::new();

        for triple_value in triples_array {
            if let Some(triple_obj) = triple_value.as_object() {
                let subject = triple_obj.get("subject")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string();

                let predicate = triple_obj.get("predicate")
                    .and_then(|p| p.as_str())
                    .unwrap_or("")
                    .to_string();

                let object = triple_obj.get("object")
                    .and_then(|o| o.as_str())
                    .unwrap_or("")
                    .to_string();

                if !subject.is_empty() && !predicate.is_empty() && !object.is_empty() {
                    let mut triple = RdfTriple::new(
                        self.normalize_uri(subject),
                        self.normalize_predicate(predicate),
                        object,
                    ).with_source(source.to_string());

                    // Extract confidence if present
                    if let Some(conf) = triple_obj.get("confidence").and_then(|c| c.as_f64()) {
                        triple = triple.with_confidence(conf as f32);
                    }

                    triples.push(triple);
                }
            }
        }

        Ok(triples)
    }

    fn normalize_uri(&self, uri: String) -> String {
        if uri.starts_with("http://") || uri.starts_with("https://") {
            uri
        } else {
            format!("{}{}", self.config.rdf_schema.base_uri, uri)
        }
    }

    fn normalize_predicate(&self, predicate: String) -> String {
        if predicate.starts_with("http://") || predicate.starts_with("https://") {
            predicate
        } else {
            format!("{}{}", self.config.rdf_schema.namespace, predicate)
        }
    }

    fn post_process_triples(&self, triples: Vec<RdfTriple>) -> Vec<RdfTriple> {
        let mut processed = triples;

        // Apply deduplication
        if self.config.post_processing.deduplicate {
            processed = self.deduplicate_triples(processed);
        }

        // Apply validation rules
        if !self.config.validation_rules.is_empty() {
            processed = self.apply_validation_rules(processed);
        }

        processed
    }

    fn deduplicate_triples(&self, triples: Vec<RdfTriple>) -> Vec<RdfTriple> {
        let mut unique_triples = Vec::new();

        for triple in triples {
            let is_duplicate = unique_triples.iter().any(|existing: &RdfTriple| {
                existing.subject == triple.subject
                    && existing.predicate == triple.predicate
                    && existing.object == triple.object
            });

            if !is_duplicate {
                unique_triples.push(triple);
            }
        }

        unique_triples
    }

    fn apply_validation_rules(&self, triples: Vec<RdfTriple>) -> Vec<RdfTriple> {
        let mut valid_triples = Vec::new();

        for triple in triples {
            let mut is_valid = true;

            for rule in &self.config.validation_rules {
                match rule.as_str() {
                    "require_valid_uri" => {
                        if !triple.subject.starts_with("http") {
                            is_valid = false;
                            break;
                        }
                    }
                    "require_known_predicates" => {
                        let predicate_name = triple.predicate
                            .split('/')
                            .last()
                            .unwrap_or("")
                            .split('#')
                            .last()
                            .unwrap_or("");

                        if !self.config.rdf_schema.predicates.contains_key(predicate_name) {
                            is_valid = false;
                            break;
                        }
                    }
                    _ => {}
                }
            }

            if is_valid {
                valid_triples.push(triple);
            }
        }

        valid_triples
    }
}