use anyhow::{Result, Context};
use async_trait::async_trait;
use std::path::Path;
use std::collections::HashMap;
use reqwest;
use scraper::{Html, Selector};

#[async_trait]
pub trait DocumentHandler: Send + Sync {
    async fn extract_text(&self, source: &str) -> Result<String>;
    async fn get_metadata(&self, source: &str) -> Result<HashMap<String, String>>;
}

pub struct PdfHandler;

#[async_trait]
impl DocumentHandler for PdfHandler {
    async fn extract_text(&self, source: &str) -> Result<String> {
        let bytes = tokio::fs::read(source).await
            .with_context(|| format!("Failed to read PDF file: {}", source))?;

        // Use pdf-extract for text extraction
        let text = pdf_extract::extract_text_from_mem(&bytes)
            .with_context(|| "Failed to extract text from PDF")?;

        Ok(text)
    }

    async fn get_metadata(&self, source: &str) -> Result<HashMap<String, String>> {
        let mut metadata = HashMap::new();
        metadata.insert("source".to_string(), source.to_string());
        metadata.insert("type".to_string(), "pdf".to_string());

        // Get file size
        if let Ok(meta) = tokio::fs::metadata(source).await {
            metadata.insert("size".to_string(), meta.len().to_string());
        }

        Ok(metadata)
    }
}

pub struct TextHandler;

#[async_trait]
impl DocumentHandler for TextHandler {
    async fn extract_text(&self, source: &str) -> Result<String> {
        // Read file and detect encoding
        let bytes = tokio::fs::read(source).await
            .with_context(|| format!("Failed to read text file: {}", source))?;

        // Try to detect encoding
        let encoding = if let Some((enc, _)) = encoding_rs::Encoding::for_bom(&bytes) {
            enc
        } else {
            encoding_rs::UTF_8
        };

        let (text, _, had_errors) = encoding.decode(&bytes);
        if had_errors {
            tracing::warn!("Encoding errors detected in file: {}", source);
        }

        Ok(text.into_owned())
    }

    async fn get_metadata(&self, source: &str) -> Result<HashMap<String, String>> {
        let mut metadata = HashMap::new();
        metadata.insert("source".to_string(), source.to_string());
        metadata.insert("type".to_string(), "text".to_string());

        if let Ok(meta) = tokio::fs::metadata(source).await {
            metadata.insert("size".to_string(), meta.len().to_string());
        }

        Ok(metadata)
    }
}

pub struct UrlHandler {
    client: reqwest::Client,
}

impl UrlHandler {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }
}

#[async_trait]
impl DocumentHandler for UrlHandler {
    async fn extract_text(&self, source: &str) -> Result<String> {
        let response = self.client
            .get(source)
            .send()
            .await
            .with_context(|| format!("Failed to fetch URL: {}", source))?;

        let html = response.text().await
            .with_context(|| "Failed to read response body")?;

        // Parse HTML and extract text
        let document = Html::parse_document(&html);

        let mut text_parts = Vec::new();

        // Extract text using selector
        if let Ok(body_selector) = Selector::parse("body") {
            for element in document.select(&body_selector) {
                let text = element.text().collect::<Vec<_>>().join(" ");
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    text_parts.push(trimmed.to_string());
                }
            }
        }

        // If no body found, extract from whole document
        if text_parts.is_empty() {
            if let Ok(all_selector) = Selector::parse("*") {
                for element in document.select(&all_selector) {
                    if element.value().name() == "script" || element.value().name() == "style" {
                        continue;
                    }
                    let text = element.text().collect::<Vec<_>>().join(" ");
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        text_parts.push(trimmed.to_string());
                    }
                }
            }
        }

        Ok(text_parts.join("\n"))
    }

    async fn get_metadata(&self, source: &str) -> Result<HashMap<String, String>> {
        let mut metadata = HashMap::new();
        metadata.insert("source".to_string(), source.to_string());
        metadata.insert("type".to_string(), "url".to_string());

        // Try to fetch and parse metadata from HTML
        let response = self.client
            .get(source)
            .send()
            .await?;

        let html = response.text().await?;
        let document = Html::parse_document(&html);

        // Extract title
        if let Some(title_el) = document.select(&Selector::parse("title").unwrap()).next() {
            metadata.insert("title".to_string(), title_el.inner_html());
        }

        // Extract meta tags
        for meta in document.select(&Selector::parse("meta").unwrap()) {
            if let Some(name) = meta.value().attr("name") {
                if let Some(content) = meta.value().attr("content") {
                    match name {
                        "description" => metadata.insert("description".to_string(), content.to_string()),
                        "keywords" => metadata.insert("keywords".to_string(), content.to_string()),
                        "author" => metadata.insert("author".to_string(), content.to_string()),
                        _ => None,
                    };
                }
            }
        }

        Ok(metadata)
    }
}

pub struct DocumentProcessor {
    handlers: HashMap<String, Box<dyn DocumentHandler>>,
}

impl DocumentProcessor {
    pub fn new() -> Self {
        let mut handlers: HashMap<String, Box<dyn DocumentHandler>> = HashMap::new();

        // Register default handlers
        handlers.insert("pdf".to_string(), Box::new(PdfHandler));
        handlers.insert("txt".to_string(), Box::new(TextHandler));
        handlers.insert("text".to_string(), Box::new(TextHandler));
        handlers.insert("md".to_string(), Box::new(TextHandler));
        handlers.insert("url".to_string(), Box::new(UrlHandler::new()));

        Self { handlers }
    }

    pub async fn process(&self, source: &str) -> Result<ProcessedDocument> {
        let handler = self.get_handler(source)?;

        let text = handler.extract_text(source).await?;
        let metadata = handler.get_metadata(source).await?;

        Ok(ProcessedDocument {
            source: source.to_string(),
            text,
            metadata,
        })
    }

    fn get_handler(&self, source: &str) -> Result<&Box<dyn DocumentHandler>> {
        // Check if it's a URL
        if source.starts_with("http://") || source.starts_with("https://") {
            return self.handlers.get("url")
                .ok_or_else(|| anyhow::anyhow!("URL handler not found"));
        }

        // Get file extension
        let path = Path::new(source);
        let extension = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("txt");

        self.handlers.get(extension)
            .or_else(|| self.handlers.get("txt"))
            .ok_or_else(|| anyhow::anyhow!("No handler found for file type: {}", extension))
    }

    pub async fn process_multiple(&self, sources: Vec<String>) -> Vec<Result<ProcessedDocument>> {
        let mut results = Vec::new();

        for source in sources {
            results.push(self.process(&source).await);
        }

        results
    }
}

#[derive(Debug, Clone)]
pub struct ProcessedDocument {
    pub source: String,
    pub text: String,
    pub metadata: HashMap<String, String>,
}