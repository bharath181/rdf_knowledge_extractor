use anyhow::{Result, Context};
use reqwest;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: f32,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionChoice {
    pub message: ChatMessage,
    pub finish_reason: String,
    pub index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatCompletionChoice>,
    pub usage: Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub owned_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsResponse {
    pub object: String,
    pub data: Vec<Model>,
}

#[derive(Debug)]
pub struct LlmResponse {
    pub content: String,
    pub usage: Usage,
    pub model: String,
    pub finish_reason: String,
    pub response_time: Duration,
}

pub struct VllmClient {
    client: reqwest::Client,
    base_url: String,
    pub model: String,
    temperature: f32,
    max_tokens: u32,
    timeout: Duration,
}

impl VllmClient {
    pub fn new(
        base_url: String,
        api_key: Option<String>,
        model: String,
        temperature: f32,
        max_tokens: u32,
        timeout: u64,
    ) -> Result<Self> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );

        if let Some(key) = api_key {
            headers.insert(
                reqwest::header::AUTHORIZATION,
                reqwest::header::HeaderValue::from_str(&format!("Bearer {}", key))?,
            );
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout))
            .default_headers(headers)
            .build()?;

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            model,
            temperature,
            max_tokens,
            timeout: Duration::from_secs(timeout),
        })
    }

    pub async fn check_health(&self) -> Result<bool> {
        let url = format!("{}/health", self.base_url);
        let response = self.client
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await;

        match response {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    pub async fn list_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/v1/models", self.base_url);

        let response = self.client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch models")?;

        if !response.status().is_success() {
            anyhow::bail!("API returned error: {}", response.status());
        }

        let models: ModelsResponse = response.json().await
            .context("Failed to parse models response")?;

        Ok(models.data.into_iter().map(|m| m.id).collect())
    }

    pub async fn generate(
        &self,
        prompt: &str,
        system_prompt: Option<&str>,
    ) -> Result<LlmResponse> {
        let start_time = Instant::now();

        let mut messages = Vec::new();

        if let Some(system) = system_prompt {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: system.to_string(),
            });
        }

        messages.push(ChatMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        });

        let request = ChatCompletionRequest {
            model: self.model.clone(),
            messages,
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            top_p: Some(0.9),
            frequency_penalty: Some(0.0),
            presence_penalty: Some(0.0),
            stop: None,
        };

        debug!("Sending request to vLLM: {:?}", request);

        let url = format!("{}/v1/chat/completions", self.base_url);
        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to vLLM")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("vLLM API error {}: {}", status, error_text);
        }

        let completion: ChatCompletionResponse = response.json().await
            .context("Failed to parse completion response")?;

        let choice = completion.choices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No choices in response"))?;

        let response_time = start_time.elapsed();

        Ok(LlmResponse {
            content: choice.message.content,
            usage: completion.usage,
            model: completion.model,
            finish_reason: choice.finish_reason,
            response_time,
        })
    }

    pub async fn generate_structured(
        &self,
        prompt: &str,
        system_prompt: Option<&str>,
    ) -> Result<serde_json::Value> {
        // Add JSON instruction to prompt
        let json_prompt = format!(
            "{}\n\nPlease respond with valid JSON only. Do not include any markdown formatting or explanation text.",
            prompt
        );

        let response = self.generate(&json_prompt, system_prompt).await?;

        // Try to parse JSON from response
        let content = response.content.trim();

        // Handle common cases where LLM wraps JSON in markdown
        let json_content = if content.starts_with("```json") && content.ends_with("```") {
            &content[7..content.len() - 3].trim()
        } else if content.starts_with("```") && content.ends_with("```") {
            &content[3..content.len() - 3].trim()
        } else {
            content
        };

        serde_json::from_str(json_content)
            .with_context(|| format!("Failed to parse JSON response: {}", json_content))
    }
}

pub struct PromptBuilder;

impl PromptBuilder {
    pub fn build_extraction_prompt(
        document_text: &str,
        questions: &[crate::config::ExtractionQuestion],
        schema: &crate::config::RdfSchema,
    ) -> String {
        let mut prompt = String::new();

        // Document content (truncated to prevent token overflow)
        prompt.push_str("## Document Content\n");
        let truncated_text = if document_text.len() > 8000 {
            &document_text[..8000]
        } else {
            document_text
        };
        prompt.push_str(truncated_text);
        prompt.push_str("\n\n");

        // Extraction questions
        prompt.push_str("## Information to Extract\n");
        for question in questions {
            prompt.push_str(&format!("- {}: {}\n", question.id, question.question));
            if !question.constraints.is_empty() {
                prompt.push_str(&format!("  Constraints: {}\n", question.constraints.join(", ")));
            }
        }
        prompt.push_str("\n");

        // Schema information
        prompt.push_str("## RDF Schema\n");
        prompt.push_str(&format!("Base URI: {}\n", schema.base_uri));
        prompt.push_str(&format!("Namespace: {}\n", schema.namespace));

        if !schema.predicates.is_empty() {
            prompt.push_str("\nAvailable Predicates:\n");
            for (pred, desc) in &schema.predicates {
                prompt.push_str(&format!("- {}: {}\n", pred, desc));
            }
        }

        // Instructions
        prompt.push_str("\n## Instructions\n");
        prompt.push_str(r#"
Extract the requested information from the document and return it as RDF triples.
Each triple should have:
- subject: The entity being described (use URIs from the base URI)
- predicate: The relationship or property (use predicates from the schema)
- object: The value or related entity

Return the triples as a JSON array with objects containing 'subject', 'predicate', and 'object' fields.
Only extract information that directly answers the specified questions.
If information is not found in the document, do not create triples for it.

Example format:
[
  {
    "subject": "http://example.org/resource/company1",
    "predicate": "http://example.org/ontology#hasName",
    "object": "Acme Corporation"
  }
]
"#);

        prompt
    }

    pub fn get_system_prompt() -> &'static str {
        r#"You are an expert knowledge extraction system specializing in converting unstructured text into structured RDF triples.

Your task is to:
1. Carefully read and understand the provided document
2. Extract only the information that directly answers the specified questions
3. Structure the extracted information as valid RDF triples
4. Ensure all URIs are properly formatted using the provided base URI
5. Use only the predicates defined in the schema
6. Be precise and avoid inferring information not explicitly stated

Return your response as a JSON array of triple objects."#
    }
}