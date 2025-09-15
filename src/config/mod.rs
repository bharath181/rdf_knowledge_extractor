use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use anyhow::{Result, Context};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configuration {
    pub name: String,
    pub description: String,
    pub version: String,
    pub extraction_questions: Vec<ExtractionQuestion>,
    pub rdf_schema: RdfSchema,
    pub output_format: OutputFormat,
    pub llm_settings: LlmSettings,
    #[serde(default)]
    pub validation_rules: Vec<String>,
    #[serde(default)]
    pub post_processing: PostProcessing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionQuestion {
    pub id: String,
    pub question: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_type: Option<String>,
    #[serde(default)]
    pub constraints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RdfSchema {
    pub namespace: String,
    pub prefix: String,
    pub base_uri: String,
    #[serde(default)]
    pub predicates: HashMap<String, String>,
    #[serde(default)]
    pub classes: HashMap<String, String>,
    #[serde(default)]
    pub custom_vocabularies: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputFormat {
    Turtle,
    JsonLd,
    NTriples,
    RdfXml,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmSettings {
    pub base_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    pub model: String,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PostProcessing {
    #[serde(default = "default_true")]
    pub deduplicate: bool,
    #[serde(default = "default_true")]
    pub normalize_uris: bool,
}

fn default_temperature() -> f32 { 0.3 }
fn default_max_tokens() -> u32 { 4096 }
fn default_timeout() -> u64 { 120 }
fn default_true() -> bool { true }

impl Configuration {
    /// Load configuration from a YAML or JSON file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config = if path.extension().and_then(|s| s.to_str()) == Some("json") {
            serde_json::from_str(&content)?
        } else {
            serde_yaml::from_str(&content)?
        };

        Ok(config)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.extraction_questions.is_empty() {
            anyhow::bail!("No extraction questions defined");
        }

        if self.rdf_schema.base_uri.is_empty() {
            anyhow::bail!("No base URI defined for RDF schema");
        }

        for question in &self.extraction_questions {
            if question.id.is_empty() {
                anyhow::bail!("Question missing ID: {}", question.question);
            }
        }

        Ok(())
    }

    /// Create an example configuration
    pub fn example() -> Self {
        let mut predicates = HashMap::new();
        predicates.insert("hasName".to_string(), "Entity has name".to_string());
        predicates.insert("hasRole".to_string(), "Person has role".to_string());
        predicates.insert("worksFor".to_string(), "Person works for organization".to_string());
        predicates.insert("locatedIn".to_string(), "Entity is located in place".to_string());

        let mut classes = HashMap::new();
        classes.insert("Person".to_string(), "A human being".to_string());
        classes.insert("Organization".to_string(), "A company or institution".to_string());
        classes.insert("Role".to_string(), "A job title or position".to_string());

        Configuration {
            name: "Example RDF Extraction Config".to_string(),
            description: "Extract organization and person information from documents".to_string(),
            version: "1.0".to_string(),
            extraction_questions: vec![
                ExtractionQuestion {
                    id: "org_name".to_string(),
                    question: "What organizations are mentioned in the document?".to_string(),
                    description: Some("Extract names of companies, institutions, or organizations".to_string()),
                    expected_type: Some("string".to_string()),
                    constraints: vec![
                        "Must be proper noun".to_string(),
                        "Full organization name".to_string(),
                    ],
                },
                ExtractionQuestion {
                    id: "person_name".to_string(),
                    question: "What people are mentioned with their roles?".to_string(),
                    description: Some("Extract person names and their associated roles or titles".to_string()),
                    expected_type: Some("object".to_string()),
                    constraints: vec![
                        "Include full name".to_string(),
                        "Include job title if mentioned".to_string(),
                    ],
                },
            ],
            rdf_schema: RdfSchema {
                namespace: "http://example.org/ontology#".to_string(),
                prefix: "ex".to_string(),
                base_uri: "http://example.org/resource/".to_string(),
                predicates,
                classes,
                custom_vocabularies: HashMap::new(),
            },
            output_format: OutputFormat::Turtle,
            llm_settings: LlmSettings {
                base_url: "http://localhost:8000".to_string(),
                api_key: None,
                model: "Qwen/Qwen2.5-32B-Instruct".to_string(),
                temperature: 0.3,
                max_tokens: 4096,
                timeout: 120,
            },
            validation_rules: vec![
                "require_valid_uri".to_string(),
                "require_known_predicates".to_string(),
            ],
            post_processing: PostProcessing {
                deduplicate: true,
                normalize_uris: true,
            },
        }
    }
}