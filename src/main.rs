use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use std::path::PathBuf;
use tokio;
use tracing::{info, warn, error};
use tracing_subscriber;

use rdf_knowledge_extractor::{
    config::Configuration,
    core::{VllmClient, RdfExtractor},
    utils::RdfSerializer,
    knowledge_graph::{KnowledgeGraph, KnowledgeGraphConfig, SimpleSparqlResults},
    templates::{TemplateManager, TemplateGenerationRequest},
};

#[derive(Parser)]
#[command(
    name = "rdf_knowledge_extractor",
    about = "Extract structured RDF triples from documents using LLM",
    long_about = None,
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Enable debug logging
    #[arg(short, long, global = true)]
    debug: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// PHASE 1: Extract RDF triples from documents and store in knowledge graph
    Extract {
        /// Configuration file path
        #[arg(short, long)]
        config: PathBuf,

        /// Input documents or URLs
        #[arg(short, long, required = true)]
        input: Vec<String>,

        /// Knowledge graph database path
        #[arg(long, default_value = "knowledge_graph.db")]
        kg_path: String,

        /// Also export triples to file
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output format for export
        #[arg(short, long, value_enum, default_value = "turtle")]
        format: OutputFormatArg,

        /// vLLM server URL
        #[arg(long, default_value = "http://localhost:8000")]
        server_url: String,

        /// API key for vLLM server
        #[arg(long)]
        api_key: Option<String>,

        /// Model to use (overrides config)
        #[arg(long)]
        model: Option<String>,

        /// Merge results from multiple documents
        #[arg(long)]
        merge: bool,

        /// Validate extracted triples
        #[arg(long)]
        validate: bool,
    },

    /// PHASE 2: Generate documents from templates using knowledge graph
    Generate {
        /// Configuration file path
        #[arg(short, long)]
        config: PathBuf,

        /// Knowledge graph database path
        #[arg(long, default_value = "knowledge_graph.db")]
        kg_path: String,

        /// Template file or directory
        #[arg(short, long)]
        template: String,

        /// Template ID to use (required if template is directory)
        #[arg(long)]
        template_id: Option<String>,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// vLLM server URL
        #[arg(long, default_value = "http://localhost:8000")]
        server_url: String,

        /// API key for vLLM server
        #[arg(long)]
        api_key: Option<String>,

        /// Model to use (overrides config)
        #[arg(long)]
        model: Option<String>,

        /// Additional context as JSON
        #[arg(long)]
        context: Option<String>,

        /// Enable LLM enhancement
        #[arg(long)]
        enhance: bool,
    },

    /// Query the knowledge graph with SPARQL
    Query {
        /// Knowledge graph database path
        #[arg(long, default_value = "knowledge_graph.db")]
        kg_path: String,

        /// SPARQL query string
        #[arg(short, long)]
        query: Option<String>,

        /// SPARQL query file
        #[arg(short, long)]
        file: Option<PathBuf>,

        /// Output format
        #[arg(short, long, value_enum, default_value = "table")]
        format: QueryOutputFormat,
    },

    /// Show knowledge graph statistics
    Stats {
        /// Knowledge graph database path
        #[arg(long, default_value = "knowledge_graph.db")]
        kg_path: String,

        /// Configuration file path
        #[arg(short, long)]
        config: PathBuf,
    },

    /// Export knowledge graph to file
    Export {
        /// Knowledge graph database path
        #[arg(long, default_value = "knowledge_graph.db")]
        kg_path: String,

        /// Configuration file path
        #[arg(short, long)]
        config: PathBuf,

        /// Output file path
        #[arg(short, long)]
        output: PathBuf,

        /// Output format
        #[arg(short, long, value_enum, default_value = "turtle")]
        format: OutputFormatArg,
    },

    /// List available templates
    ListTemplates {
        /// Template directory
        #[arg(short, long, default_value = "templates")]
        template_dir: String,
    },

    /// Validate configuration file
    Validate {
        /// Configuration file path
        #[arg(short, long)]
        config: PathBuf,
    },

    /// Check vLLM server status
    CheckServer {
        /// vLLM server URL
        #[arg(long, default_value = "http://localhost:8000")]
        server_url: String,

        /// API key for vLLM server
        #[arg(long)]
        api_key: Option<String>,
    },

    /// Generate example configuration file
    GenerateConfig {
        /// Output path for configuration file
        #[arg(short, long)]
        output: PathBuf,

        /// Configuration format (yaml or json)
        #[arg(short, long, default_value = "yaml")]
        format: ConfigFormat,
    },

    /// Generate example templates
    GenerateTemplates {
        /// Output directory for templates
        #[arg(short, long, default_value = "templates")]
        output_dir: PathBuf,
    },
}

#[derive(clap::ValueEnum, Clone)]
enum OutputFormatArg {
    Turtle,
    JsonLd,
    NTriples,
    RdfXml,
    Json,
}

impl From<OutputFormatArg> for rdf_knowledge_extractor::config::OutputFormat {
    fn from(format: OutputFormatArg) -> Self {
        match format {
            OutputFormatArg::Turtle => Self::Turtle,
            OutputFormatArg::JsonLd => Self::JsonLd,
            OutputFormatArg::NTriples => Self::NTriples,
            OutputFormatArg::RdfXml => Self::RdfXml,
            OutputFormatArg::Json => Self::Json,
        }
    }
}

#[derive(clap::ValueEnum, Clone)]
enum ConfigFormat {
    Yaml,
    Json,
}

#[derive(clap::ValueEnum, Clone)]
enum QueryOutputFormat {
    Table,
    Json,
    Csv,
    Turtle,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging
    let log_level = if cli.debug {
        tracing::Level::DEBUG
    } else if cli.verbose {
        tracing::Level::INFO
    } else {
        tracing::Level::WARN
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .init();

    match cli.command {
        Commands::Extract {
            config,
            input,
            kg_path,
            output,
            format,
            server_url,
            api_key,
            model,
            merge,
            validate,
        } => {
            extract_command(
                config, input, kg_path, output, format, server_url, api_key, model, merge, validate,
            ).await
        }
        Commands::Generate {
            config,
            kg_path,
            template,
            template_id,
            output,
            server_url,
            api_key,
            model,
            context,
            enhance,
        } => {
            generate_command(
                config, kg_path, template, template_id, output, server_url, api_key, model, context, enhance,
            ).await
        }
        Commands::Query { kg_path, query, file, format } => {
            query_command(kg_path, query, file, format).await
        }
        Commands::Stats { kg_path, config } => {
            stats_command(kg_path, config).await
        }
        Commands::Export { kg_path, config, output, format } => {
            export_command(kg_path, config, output, format).await
        }
        Commands::ListTemplates { template_dir } => {
            list_templates_command(template_dir).await
        }
        Commands::Validate { config } => validate_command(config).await,
        Commands::CheckServer { server_url, api_key } => {
            check_server_command(server_url, api_key).await
        }
        Commands::GenerateConfig { output, format } => {
            generate_config_command(output, format).await
        }
        Commands::GenerateTemplates { output_dir } => {
            generate_templates_command(output_dir).await
        }
    }
}

async fn extract_command(
    config_path: PathBuf,
    input: Vec<String>,
    kg_path: String,
    output: Option<PathBuf>,
    format: OutputFormatArg,
    server_url: String,
    api_key: Option<String>,
    model_override: Option<String>,
    merge: bool,
    validate: bool,
) -> Result<()> {
    println!("{}", "Starting RDF extraction...".bright_blue().bold());

    // Load configuration
    let mut config = Configuration::from_file(&config_path)?;
    config.validate()?;

    // Override settings if provided
    if server_url != "http://localhost:8000" {
        config.llm_settings.base_url = server_url;
    }
    if let Some(key) = api_key {
        config.llm_settings.api_key = Some(key);
    }
    if let Some(model) = model_override {
        config.llm_settings.model = model;
    }

    println!(" Configuration: {}", config.name.bright_green());
    println!(" Questions: {}", config.extraction_questions.len());
    println!(" Documents: {}", input.len());

    // Create LLM client
    let llm_client = VllmClient::new(
        config.llm_settings.base_url.clone(),
        config.llm_settings.api_key.clone(),
        config.llm_settings.model.clone(),
        config.llm_settings.temperature,
        config.llm_settings.max_tokens,
        config.llm_settings.timeout,
    )?;

    // Check server health
    if !llm_client.check_health().await? {
        error!(" vLLM server is not responding at {}", config.llm_settings.base_url);
        return Err(anyhow::anyhow!("vLLM server health check failed"));
    }

    println!(" vLLM server is healthy");

    // Create knowledge graph
    let kg_config = KnowledgeGraphConfig {
        storage_path: kg_path.clone(),
        ..Default::default()
    };
    let mut knowledge_graph = KnowledgeGraph::new(kg_config, config.rdf_schema.clone())?;

    // Create extractor
    let extractor = RdfExtractor::new(config.clone(), llm_client);

    // Process documents
    let results = extractor.extract_from_multiple(input).await?;

    // Check for errors
    let mut has_errors = false;
    for result in &results {
        if !result.errors.is_empty() {
            has_errors = true;
            warn!(" Errors in {}: {}", result.document_source, result.errors.join(", "));
        }
    }

    // Merge results if requested
    let final_results = if merge && results.len() > 1 {
        println!(" Merging results...");
        vec![extractor.merge_results(results)?]
    } else {
        results
    };

    // Validate triples if requested
    if validate {
        for result in &final_results {
            let issues = rdf_knowledge_extractor::utils::validate_rdf_triples(&result.triples);
            if !issues.is_empty() {
                warn!(" Validation issues in {}: {}", result.document_source, issues.join(", "));
            }
        }
    }

    // Store triples in knowledge graph
    let mut total_stored = 0;
    for result in &final_results {
        let stored = knowledge_graph.add_triples(&result.triples)?;
        total_stored += stored;
    }
    println!(" Stored {} triples in knowledge graph: {}", total_stored.to_string().bright_cyan(), kg_path.bright_green());

    // Export to file if requested
    if let Some(output_path) = &output {
        let mut serializer = RdfSerializer::new();
        let output_format = format.into();

        for (i, result) in final_results.iter().enumerate() {
            let serialized = serializer.serialize(
                &result.triples,
                &output_format,
                &config.rdf_schema.namespace,
                &config.rdf_schema.prefix,
            )?;

            let final_path = if final_results.len() > 1 && !merge {
                let stem = output_path.file_stem().unwrap().to_str().unwrap();
                let extension = output_path.extension().unwrap_or_default().to_str().unwrap();
                output_path.with_file_name(format!("{}_{}.{}", stem, i + 1, extension))
            } else {
                output_path.clone()
            };

            tokio::fs::write(&final_path, &serialized).await?;
            println!(" Export written to: {}", final_path.display().to_string().bright_green());
        }
    }

    // Summary
    let total_triples: usize = final_results.iter().map(|r| r.triples.len()).sum();
    let total_time: f64 = final_results.iter().map(|r| r.processing_time_seconds).sum();

    println!("\n{}", " Extraction Summary".bright_green().bold());
    println!(" Total triples extracted: {}", total_triples.to_string().bright_cyan());
    println!(" Total processing time: {:.2}s", total_time);

    if has_errors {
        println!(" {} completed with some errors", "Extraction".bright_yellow());
    } else {
        println!(" {} completed successfully!", "Extraction".bright_green());
    }

    Ok(())
}

async fn validate_command(config_path: PathBuf) -> Result<()> {
    println!("{}", " Validating configuration...".bright_blue().bold());

    match Configuration::from_file(&config_path) {
        Ok(config) => {
            match config.validate() {
                Ok(()) => {
                    println!(" Configuration is valid!");
                    println!(" Name: {}", config.name.bright_green());
                    println!(" Version: {}", config.version);
                    println!(" Questions: {}", config.extraction_questions.len());
                    println!(" Namespace: {}", config.rdf_schema.namespace);
                    println!(" Model: {}", config.llm_settings.model);
                    Ok(())
                }
                Err(e) => {
                    error!(" Configuration validation failed: {}", e);
                    Err(e)
                }
            }
        }
        Err(e) => {
            error!(" Failed to load configuration: {}", e);
            Err(e)
        }
    }
}

async fn check_server_command(server_url: String, api_key: Option<String>) -> Result<()> {
    println!("{}", " Checking vLLM server...".bright_blue().bold());

    let client = VllmClient::new(
        server_url.clone(),
        api_key,
        "test".to_string(),
        0.3,
        1024,
        30,
    )?;

    // Check health
    let is_healthy = client.check_health().await?;
    if is_healthy {
        println!(" Server is healthy at {}", server_url.bright_green());
    } else {
        println!(" Server is not responding at {}", server_url.bright_red());
        return Ok(());
    }

    // List models
    match client.list_models().await {
        Ok(models) => {
            println!(" Available models:");
            for model in models {
                println!("  • {}", model.bright_cyan());
            }
        }
        Err(e) => {
            warn!(" Could not list models: {}", e);
        }
    }

    Ok(())
}

async fn generate_config_command(output_path: PathBuf, format: ConfigFormat) -> Result<()> {
    println!("{}", " Generating example configuration...".bright_blue().bold());

    let config = Configuration::example();

    let content = match format {
        ConfigFormat::Yaml => serde_yaml::to_string(&config)?,
        ConfigFormat::Json => serde_json::to_string_pretty(&config)?,
    };

    tokio::fs::write(&output_path, content).await?;

    println!(" Example configuration generated at: {}", output_path.display().to_string().bright_green());
    println!(" Edit the file to customize for your use case");

    Ok(())
}

async fn generate_command(
    config_path: PathBuf,
    kg_path: String,
    template_path: String,
    template_id: Option<String>,
    output: Option<PathBuf>,
    server_url: String,
    api_key: Option<String>,
    model_override: Option<String>,
    context: Option<String>,
    enhance: bool,
) -> Result<()> {
    println!("{}", " Starting document generation...".bright_blue().bold());

    // Load configuration
    let mut config = Configuration::from_file(&config_path)?;
    config.validate()?;

    // Override LLM settings if provided
    if server_url != "http://localhost:8000" {
        config.llm_settings.base_url = server_url;
    }
    if let Some(key) = api_key {
        config.llm_settings.api_key = Some(key);
    }
    if let Some(model) = model_override {
        config.llm_settings.model = model;
    }

    // Create LLM client
    let llm_client = VllmClient::new(
        config.llm_settings.base_url.clone(),
        config.llm_settings.api_key.clone(),
        config.llm_settings.model.clone(),
        config.llm_settings.temperature,
        config.llm_settings.max_tokens,
        config.llm_settings.timeout,
    )?;

    // Load knowledge graph
    let kg_config = KnowledgeGraphConfig {
        storage_path: kg_path.clone(),
        ..Default::default()
    };
    let mut knowledge_graph = KnowledgeGraph::new(kg_config, config.rdf_schema.clone())?;

    // Create template manager
    let mut template_manager = TemplateManager::new(knowledge_graph, llm_client);

    // Load templates
    if std::path::Path::new(&template_path).is_dir() {
        template_manager.load_templates_from_directory(&template_path)?;
    } else {
        template_manager.load_template(&template_path)?;
    }

    // Determine template ID
    let final_template_id = if let Some(id) = template_id {
        id
    } else if std::path::Path::new(&template_path).is_file() {
        // Extract template ID from file (use filename without extension)
        std::path::Path::new(&template_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("default")
            .to_string()
    } else {
        anyhow::bail!("Template ID required when template path is a directory");
    };

    // Parse additional context
    let additional_context = if let Some(ctx_str) = context {
        Some(serde_json::from_str(&ctx_str)?)
    } else {
        None
    };

    // Create generation request
    let request = TemplateGenerationRequest {
        template_id: final_template_id.clone(),
        context: additional_context,
        override_queries: None,
        output_path: output.as_ref().map(|p| p.to_string_lossy().to_string()),
    };

    println!(" Template: {}", final_template_id.bright_green());
    println!(" Knowledge graph: {}", kg_path.bright_cyan());

    // Generate document
    let generated = template_manager.generate_document(&request).await?;

    // Output or save result
    if let Some(output_path) = output {
        tokio::fs::write(&output_path, &generated.generated_content).await?;
        println!(" Generated document saved to: {}", output_path.display().to_string().bright_green());
    } else {
        println!("\n{}", " Generated Document:".bright_yellow().bold());
        println!("{}", generated.generated_content);
    }

    // Show metadata
    println!("\n{}", " Generation Metadata:".bright_green().bold());
    println!(" Word count: {}", generated.metadata.word_count.to_string().bright_cyan());
    println!(" Processing time: {:.2}s", generated.metadata.processing_time_seconds);
    println!(" Queries executed: {}", generated.metadata.queries_executed.len());

    Ok(())
}

async fn query_command(
    kg_path: String,
    query: Option<String>,
    file: Option<PathBuf>,
    format: QueryOutputFormat,
) -> Result<()> {
    println!("{}", " Executing SPARQL query...".bright_blue().bold());

    // Get query string
    let query_string = if let Some(q) = query {
        q
    } else if let Some(file_path) = file {
        tokio::fs::read_to_string(file_path).await?
    } else {
        anyhow::bail!("Either --query or --file must be provided");
    };

    // Load knowledge graph
    let kg_config = KnowledgeGraphConfig {
        storage_path: kg_path.clone(),
        ..Default::default()
    };
    // Create a minimal schema for the knowledge graph
    let minimal_schema = rdf_knowledge_extractor::config::RdfSchema {
        namespace: "http://example.org/".to_string(),
        prefix: "ex".to_string(),
        base_uri: "http://example.org/resource/".to_string(),
        predicates: std::collections::HashMap::new(),
        classes: std::collections::HashMap::new(),
        custom_vocabularies: std::collections::HashMap::new(),
    };
    let knowledge_graph = KnowledgeGraph::new(kg_config, minimal_schema)?;

    // Execute query
    let results = knowledge_graph.execute_sparql(&query_string)?;

    // Format and display results
    match format {
        QueryOutputFormat::Table => {
            println!("{}", " Query Results:".bright_yellow().bold());
            display_results_as_table(results)?;
        }
        QueryOutputFormat::Json => {
            println!("{}", " Query Results (JSON):".bright_yellow().bold());
            display_results_as_json(results)?;
        }
        QueryOutputFormat::Csv => {
            println!("{}", " Query Results (CSV):".bright_yellow().bold());
            display_results_as_csv(results)?;
        }
        QueryOutputFormat::Turtle => {
            println!("{}", " Query Results (Turtle):".bright_yellow().bold());
            display_results_as_turtle(results)?;
        }
    }

    Ok(())
}

async fn stats_command(kg_path: String, config_path: PathBuf) -> Result<()> {
    println!("{}", " Knowledge Graph Statistics".bright_blue().bold());

    // Load configuration for schema
    let config = Configuration::from_file(&config_path)?;

    // Load knowledge graph
    let kg_config = KnowledgeGraphConfig {
        storage_path: kg_path.clone(),
        ..Default::default()
    };
    let knowledge_graph = KnowledgeGraph::new(kg_config, config.rdf_schema)?;

    // Get statistics
    let stats = knowledge_graph.get_statistics()?;
    println!("{}", stats);

    Ok(())
}

async fn export_command(
    kg_path: String,
    config_path: PathBuf,
    output: PathBuf,
    format: OutputFormatArg,
) -> Result<()> {
    println!("{}", "📤 Exporting knowledge graph...".bright_blue().bold());

    // Load configuration for schema
    let config = Configuration::from_file(&config_path)?;

    // Load knowledge graph
    let kg_config = KnowledgeGraphConfig {
        storage_path: kg_path.clone(),
        ..Default::default()
    };
    let knowledge_graph = KnowledgeGraph::new(kg_config, config.rdf_schema)?;

    // Export to file
    let format_str = match format {
        OutputFormatArg::Turtle => "turtle",
        OutputFormatArg::JsonLd => "jsonld",
        OutputFormatArg::NTriples => "ntriples",
        OutputFormatArg::RdfXml => "rdfxml",
        OutputFormatArg::Json => "json",
    };

    knowledge_graph.export_to_file(output.to_str().unwrap(), format_str)?;

    println!(" Export completed: {}", output.display().to_string().bright_green());

    Ok(())
}

async fn list_templates_command(template_dir: String) -> Result<()> {
    println!("{}", " Available Templates".bright_blue().bold());

    if !std::path::Path::new(&template_dir).exists() {
        println!(" Template directory not found: {}", template_dir.bright_red());
        return Ok(());
    }

    // Create a dummy knowledge graph and LLM client for template manager
    let kg = KnowledgeGraph::in_memory(rdf_knowledge_extractor::config::RdfSchema {
        namespace: "http://example.org/".to_string(),
        prefix: "ex".to_string(),
        base_uri: "http://example.org/resource/".to_string(),
        predicates: std::collections::HashMap::new(),
        classes: std::collections::HashMap::new(),
        custom_vocabularies: std::collections::HashMap::new(),
    })?;
    let llm_client = VllmClient::new(
        "http://localhost:8000".to_string(),
        None,
        "test".to_string(),
        0.3,
        1024,
        30,
    )?;

    let mut template_manager = TemplateManager::new(kg, llm_client);

    match template_manager.load_templates_from_directory(&template_dir) {
        Ok(count) => {
            println!(" Found {} templates in {}", count.to_string().bright_cyan(), template_dir.bright_green());

            for template in template_manager.list_templates() {
                println!("\n {} ({})", template.name.bright_yellow(), template.id.bright_cyan());
                println!("    Type: {}", template.template_type.to_string());
                println!("    Description: {}", template.description);
                println!("    Queries: {}", template.data_queries.len());
            }
        }
        Err(e) => {
            println!(" Failed to load templates: {}", e);
        }
    }

    Ok(())
}

async fn generate_templates_command(output_dir: PathBuf) -> Result<()> {
    println!("{}", " Generating example templates...".bright_blue().bold());

    // Create output directory
    tokio::fs::create_dir_all(&output_dir).await?;

    // Generate company report template
    let company_report = r#"id: "company_report"
name: "Company Report"
description: "Generate a comprehensive report about companies and their employees"
template_type: "report"
data_queries:
  - id: "companies"
    description: "Get all companies with their basic information"
    sparql_query: |
      SELECT ?company ?name ?location WHERE {
        ?company biz:hasName ?name .
        OPTIONAL { ?company biz:basedIn ?location }
      }
    required: true

  - id: "people_roles"
    description: "Get people and their roles in companies"
    sparql_query: |
      SELECT ?person ?name ?role ?company WHERE {
        ?person biz:hasName ?name .
        OPTIONAL { ?person biz:hasRole ?role }
        OPTIONAL { ?person biz:worksFor ?company }
      }
    required: false

template_content: |
  # Company Report

  ## Companies Overview
  {{#each companies}}
  ### {{name}}
  {{#if location}}📍 **Location:** {{location}}{{/if}}

  {{/each}}

  ## People and Roles
  {{#each people_roles}}
  - **{{name}}**{{#if role}} - {{role}}{{/if}}{{#if company}} ({{company}}){{/if}}
  {{/each}}

  ---
  *Generated on {{generation_timestamp}}*

output_format: "markdown"
llm_instructions: "Enhance the report with professional language and clear structure"
post_processing:
  enhance_with_llm: true
  style_guide: "Professional business report style"
  include_sources: true
"#;

    let report_path = output_dir.join("company_report.yaml");
    tokio::fs::write(&report_path, company_report).await?;

    // Generate executive summary template
    let executive_summary = r#"id: "executive_summary"
name: "Executive Summary"
description: "Generate an executive summary from company data"
template_type: "summary"
data_queries:
  - id: "key_metrics"
    description: "Get key business metrics and relationships"
    sparql_query: |
      SELECT ?subject ?predicate ?object WHERE {
        ?subject ?predicate ?object .
        FILTER(
          ?predicate = biz:partneredWith ||
          ?predicate = biz:foundedBy ||
          ?predicate = biz:ceoOf
        )
      }
    required: true

template_content: |
  # Executive Summary

  ## Key Business Insights
  {{#each key_metrics}}
  - **{{subject}}** {{predicate}} **{{object}}**
  {{/each}}

  ## Strategic Overview
  *This section will be enhanced by the LLM to provide strategic insights based on the extracted data.*

output_format: "markdown"
llm_instructions: "Create a strategic executive summary with insights about business relationships, leadership, and growth opportunities. Write in a professional, executive-level tone."
post_processing:
  enhance_with_llm: true
  style_guide: "Executive-level strategic communication"
  word_limit: 500
  include_sources: false
"#;

    let summary_path = output_dir.join("executive_summary.yaml");
    tokio::fs::write(&summary_path, executive_summary).await?;

    println!(" Generated example templates:");
    println!("   {}", report_path.display().to_string().bright_green());
    println!("   {}", summary_path.display().to_string().bright_green());
    println!(" Edit these templates to customize for your use case");

    Ok(())
}

// Helper functions for query result display
fn display_results_as_table(results: SimpleSparqlResults) -> Result<()> {
    match results {
        SimpleSparqlResults::Solutions(solutions) => {
            let mut rows = Vec::new();
            let mut headers = std::collections::HashSet::new();

            // Collect all data
            for solution in solutions {
                let mut row = std::collections::HashMap::new();

                for (var_name, value) in solution {
                    headers.insert(var_name.clone());
                    row.insert(var_name, value);
                }
                rows.push(row);
            }

            // Print table
            let header_vec: Vec<String> = headers.into_iter().collect();
            println!("{}", header_vec.join(" | ").bright_cyan());
            println!("{}", "─".repeat(header_vec.len() * 20));

            for row in rows {
                let mut values = Vec::new();
                for header in &header_vec {
                    let value = row.get(header).map(|s| s.as_str()).unwrap_or("");
                    values.push(value.to_string());
                }
                println!("{}", values.join(" | "));
            }
        }
        SimpleSparqlResults::Boolean(result) => {
            println!("Result: {}", if result { " TRUE" } else { " FALSE" });
        }
    }

    Ok(())
}

fn display_results_as_json(results: SimpleSparqlResults) -> Result<()> {
    match results {
        SimpleSparqlResults::Solutions(solutions) => {
            let mut json_results = Vec::new();

            for solution in solutions {
                let mut row = serde_json::Map::new();

                for (var_name, value) in solution {
                    row.insert(var_name, serde_json::Value::String(value));
                }
                json_results.push(serde_json::Value::Object(row));
            }

            println!("{}", serde_json::to_string_pretty(&json_results)?);
        }
        SimpleSparqlResults::Boolean(result) => {
            let json_result = serde_json::json!({ "result": result });
            println!("{}", serde_json::to_string_pretty(&json_result)?);
        }
    }

    Ok(())
}

fn display_results_as_csv(results: SimpleSparqlResults) -> Result<()> {

    match results {
        SimpleSparqlResults::Solutions(solutions) => {
            let mut rows = Vec::new();
            let mut headers = std::collections::HashSet::new();

            // Collect all data
            for solution in solutions {
                let mut row = std::collections::HashMap::new();

                for (var_name, value) in solution {
                    headers.insert(var_name.clone());
                    row.insert(var_name, value);
                }
                rows.push(row);
            }

            // Print CSV
            let header_vec: Vec<String> = headers.into_iter().collect();
            println!("{}", header_vec.join(","));

            for row in rows {
                let mut values = Vec::new();
                for header in &header_vec {
                    let value = row.get(header).map(|s| s.as_str()).unwrap_or("");
                    values.push(if value.contains(',') { format!("\"{}\"", value) } else { value.to_string() });
                }
                println!("{}", values.join(","));
            }
        }
        SimpleSparqlResults::Boolean(result) => {
            println!("result\n{}", result);
        }
    }

    Ok(())
}

fn display_results_as_turtle(results: SimpleSparqlResults) -> Result<()> {
    match results {
        SimpleSparqlResults::Solutions(solutions) => {
            println!("# SPARQL Solutions as Turtle-like format");
            for solution in solutions {
                for (var, value) in solution {
                    println!("# {}: {}", var, value);
                }
                println!();
            }
        }
        SimpleSparqlResults::Boolean(result) => {
            println!("# Boolean result: {}", result);
        }
    }

    Ok(())
}
