use anyhow::{Result, Context};
use handlebars::{Handlebars, RenderError};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::{debug, info, warn};

use crate::knowledge_graph::{KnowledgeGraph, SimpleSparqlResults};
use crate::core::llm_client::VllmClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub name: String,
    pub description: String,
    pub template_type: TemplateType,
    pub data_queries: Vec<DataQuery>,
    pub template_content: String,
    pub output_format: OutputFormat,
    pub llm_instructions: Option<String>,
    pub post_processing: Option<PostProcessingConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateType {
    Report,
    Summary,
    Form,
    Article,
    Email,
    Presentation,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    Markdown,
    Html,
    PlainText,
    Json,
    Pdf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataQuery {
    pub id: String,
    pub description: String,
    pub sparql_query: String,
    pub required: bool,
    pub transform: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostProcessingConfig {
    pub enhance_with_llm: bool,
    pub style_guide: Option<String>,
    pub word_limit: Option<usize>,
    pub include_sources: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateGenerationRequest {
    pub template_id: String,
    pub context: Option<HashMap<String, Value>>,
    pub override_queries: Option<HashMap<String, String>>,
    pub output_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedDocument {
    pub template_id: String,
    pub generated_content: String,
    pub metadata: DocumentMetadata,
    pub data_context: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub generation_timestamp: chrono::DateTime<chrono::Utc>,
    pub template_name: String,
    pub queries_executed: Vec<String>,
    pub word_count: usize,
    pub processing_time_seconds: f64,
    pub sources: Vec<String>,
}

pub struct TemplateManager {
    templates: HashMap<String, Template>,
    handlebars: Handlebars<'static>,
    knowledge_graph: KnowledgeGraph,
    llm_client: VllmClient,
}

impl TemplateManager {
    pub fn new(knowledge_graph: KnowledgeGraph, llm_client: VllmClient) -> Self {
        let mut handlebars = Handlebars::new();

        // Register custom helpers
        handlebars.register_helper("format_list", Box::new(format_list_helper));
        handlebars.register_helper("truncate", Box::new(truncate_helper));
        handlebars.register_helper("capitalize", Box::new(capitalize_helper));

        Self {
            templates: HashMap::new(),
            handlebars,
            knowledge_graph,
            llm_client,
        }
    }

    pub fn load_template(&mut self, template_path: &str) -> Result<()> {
        let content = fs::read_to_string(template_path)
            .with_context(|| format!("Failed to read template file: {}", template_path))?;

        let template: Template = if template_path.ends_with(".json") {
            serde_json::from_str(&content)?
        } else {
            serde_yaml::from_str(&content)?
        };

        info!("Loaded template: {} ({})", template.name, template.id);
        self.templates.insert(template.id.clone(), template);
        Ok(())
    }

    pub fn load_templates_from_directory(&mut self, dir_path: &str) -> Result<usize> {
        let dir = Path::new(dir_path);
        if !dir.exists() {
            anyhow::bail!("Template directory does not exist: {}", dir_path);
        }

        let mut loaded_count = 0;
        for entry in walkdir::WalkDir::new(dir) {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "yaml" || ext == "yml" || ext == "json") {
                if let Err(e) = self.load_template(path.to_str().unwrap()) {
                    warn!("Failed to load template {}: {}", path.display(), e);
                } else {
                    loaded_count += 1;
                }
            }
        }

        info!("Loaded {} templates from directory: {}", loaded_count, dir_path);
        Ok(loaded_count)
    }

    pub async fn generate_document(&self, request: &TemplateGenerationRequest) -> Result<GeneratedDocument> {
        let start_time = std::time::Instant::now();

        let template = self.templates.get(&request.template_id)
            .ok_or_else(|| anyhow::anyhow!("Template not found: {}", request.template_id))?;

        info!("Generating document from template: {}", template.name);

        // Execute data queries
        let mut data_context = Map::new();
        let mut queries_executed = Vec::new();
        let mut sources = Vec::new();

        for query in &template.data_queries {
            let sparql_query = if let Some(ref overrides) = request.override_queries {
                overrides.get(&query.id).unwrap_or(&query.sparql_query).clone()
            } else {
                query.sparql_query.clone()
            };

            debug!("Executing query '{}': {}", query.id, sparql_query);

            match self.knowledge_graph.execute_sparql(&sparql_query) {
                Ok(results) => {
                    let processed_data = self.process_query_results(results, query)?;
                    data_context.insert(query.id.clone(), processed_data);
                    queries_executed.push(query.id.clone());
                }
                Err(e) => {
                    if query.required {
                        return Err(anyhow::anyhow!("Required query '{}' failed: {}", query.id, e));
                    } else {
                        warn!("Optional query '{}' failed: {}", query.id, e);
                        data_context.insert(query.id.clone(), Value::Null);
                    }
                }
            }
        }

        // Add context from request
        if let Some(ref context) = request.context {
            for (key, value) in context {
                data_context.insert(key.clone(), value.clone());
            }
        }

        // Generate content using template
        let mut generated_content = self.handlebars.render_template(
            &template.template_content,
            &Value::Object(data_context.clone())
        ).with_context(|| "Failed to render template")?;

        // Apply LLM enhancement if configured
        if let Some(ref post_processing) = template.post_processing {
            if post_processing.enhance_with_llm {
                generated_content = self.enhance_with_llm(
                    &generated_content,
                    template,
                    post_processing
                ).await?;
            }
        }

        let processing_time = start_time.elapsed().as_secs_f64();

        let metadata = DocumentMetadata {
            generation_timestamp: chrono::Utc::now(),
            template_name: template.name.clone(),
            queries_executed,
            word_count: generated_content.split_whitespace().count(),
            processing_time_seconds: processing_time,
            sources,
        };

        Ok(GeneratedDocument {
            template_id: template.id.clone(),
            generated_content,
            metadata,
            data_context,
        })
    }

    fn process_query_results(&self, results: SimpleSparqlResults, query: &DataQuery) -> Result<Value> {
        match results {
            SimpleSparqlResults::Solutions(solutions) => {
                let mut processed_results = Vec::new();

                for solution in solutions {
                    let mut row = Map::new();

                    for (var, value_str) in solution {
                        // Try to parse as different types
                        let value = if let Ok(int_val) = value_str.parse::<i64>() {
                            Value::Number(serde_json::Number::from(int_val))
                        } else if let Ok(float_val) = value_str.parse::<f64>() {
                            Value::Number(serde_json::Number::from_f64(float_val).unwrap_or_else(|| serde_json::Number::from(0)))
                        } else if let Ok(bool_val) = value_str.parse::<bool>() {
                            Value::Bool(bool_val)
                        } else {
                            Value::String(value_str)
                        };

                        row.insert(var, value);
                    }

                    processed_results.push(Value::Object(row));
                }

                Ok(Value::Array(processed_results))
            }
            SimpleSparqlResults::Boolean(result) => {
                Ok(Value::Bool(result))
            }
        }
    }

    async fn enhance_with_llm(
        &self,
        content: &str,
        template: &Template,
        post_processing: &PostProcessingConfig,
    ) -> Result<String> {
        let mut enhancement_prompt = format!(
            "Please enhance and improve the following {} content:\n\n{}",
            template.template_type.to_string(),
            content
        );

        if let Some(ref style_guide) = post_processing.style_guide {
            enhancement_prompt.push_str(&format!("\n\nStyle Guide: {}", style_guide));
        }

        if let Some(word_limit) = post_processing.word_limit {
            enhancement_prompt.push_str(&format!("\n\nWord limit: {} words", word_limit));
        }

        if let Some(ref instructions) = template.llm_instructions {
            enhancement_prompt.push_str(&format!("\n\nAdditional instructions: {}", instructions));
        }

        enhancement_prompt.push_str("\n\nProvide the enhanced content as your response.");

        let system_prompt = "You are a skilled editor and writer. Your task is to enhance and improve the provided content while maintaining its core information and structure. Make the text more engaging, clear, and professional while preserving all important facts and data.";

        let response = self.llm_client.generate(&enhancement_prompt, Some(system_prompt)).await?;

        Ok(response.content)
    }

    pub fn list_templates(&self) -> Vec<&Template> {
        self.templates.values().collect()
    }

    pub fn get_template(&self, template_id: &str) -> Option<&Template> {
        self.templates.get(template_id)
    }
}

impl std::fmt::Display for TemplateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateType::Report => write!(f, "report"),
            TemplateType::Summary => write!(f, "summary"),
            TemplateType::Form => write!(f, "form"),
            TemplateType::Article => write!(f, "article"),
            TemplateType::Email => write!(f, "email"),
            TemplateType::Presentation => write!(f, "presentation"),
            TemplateType::Custom(name) => write!(f, "{}", name),
        }
    }
}

// Handlebars helpers
fn format_list_helper(
    h: &handlebars::Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut handlebars::RenderContext,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
    if let Some(param) = h.param(0) {
        if let Some(array) = param.value().as_array() {
            let separator = h.param(1)
                .and_then(|p| p.value().as_str())
                .unwrap_or(", ");

            let formatted: Vec<String> = array.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect();

            out.write(&formatted.join(separator))?;
        }
    }
    Ok(())
}

fn truncate_helper(
    h: &handlebars::Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut handlebars::RenderContext,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
    if let Some(param) = h.param(0) {
        if let Some(text) = param.value().as_str() {
            let limit = h.param(1)
                .and_then(|p| p.value().as_u64())
                .unwrap_or(100) as usize;

            let truncated = if text.len() > limit {
                format!("{}...", &text[..limit])
            } else {
                text.to_string()
            };

            out.write(&truncated)?;
        }
    }
    Ok(())
}

fn capitalize_helper(
    h: &handlebars::Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut handlebars::RenderContext,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
    if let Some(param) = h.param(0) {
        if let Some(text) = param.value().as_str() {
            let capitalized = text.chars()
                .enumerate()
                .map(|(i, c)| if i == 0 { c.to_uppercase().collect() } else { c.to_string() })
                .collect::<String>();

            out.write(&capitalized)?;
        }
    }
    Ok(())
}