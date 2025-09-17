use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

use crate::core::llm_client::VllmClient;
use crate::knowledge_graph::SimpleSparqlResults;

/// Represents a template field that needs to be populated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateField {
    pub field_name: String,
    pub field_type: String,  // e.g., "text", "dropdown", "date", "checklist"
    pub description: String,
    pub required: bool,
}

/// Request to populate a template with data from knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplatePopulationRequest {
    pub template_text: String,
    pub extracted_data: HashMap<String, SimpleSparqlResults>,
    pub template_fields: Vec<TemplateField>,
    pub additional_context: Option<String>,
}

pub struct TemplatePopulator {
    llm_client: VllmClient,
}

impl TemplatePopulator {
    pub fn new(llm_client: VllmClient) -> Self {
        Self { llm_client }
    }

    /// Populates a template by sending the template and extracted data to the LLM
    pub async fn populate_template(
        &self,
        template: &str,
        query_results: &HashMap<String, SimpleSparqlResults>,
        instructions: Option<&str>,
    ) -> Result<String> {
        info!("Populating template with LLM");

        // Build the prompt for the LLM
        let prompt = self.build_population_prompt(template, query_results, instructions)?;

        // System prompt that instructs the LLM on how to populate templates
        let system_prompt = r#"You are a professional document generator specializing in sales intelligence reports.
Your task is to populate a template with actual data from a knowledge graph.

Instructions:
1. Replace ALL placeholder fields marked with [FIELD: ...] with actual data from the provided triples
2. For fields marked with [DROPDOWN: ...], choose the most appropriate option based on the data
3. For fields marked with [CHECKLIST: ...], select all applicable options
4. For fields marked with [DATE FIELD], use dates from the data or write "Not Available"
5. For fields marked with [TEXT AREA: ...], write comprehensive content based on the data
6. Maintain professional tone and formatting
7. If data is missing for a required field, write "Information Not Available"
8. Preserve the template structure and headings
9. Add specific details from the knowledge graph data to make the report actionable

Return ONLY the completed template with all fields populated."#;

        // Send to LLM for population
        let response = self.llm_client.generate(&prompt, Some(system_prompt)).await?;

        debug!("Template populated successfully");
        Ok(response.content)
    }

    /// Builds the prompt containing template and data for the LLM
    fn build_population_prompt(
        &self,
        template: &str,
        query_results: &HashMap<String, SimpleSparqlResults>,
        instructions: Option<&str>,
    ) -> Result<String> {
        let mut prompt = String::new();

        // Add the template
        prompt.push_str("## Template to Populate\n\n");
        prompt.push_str(template);
        prompt.push_str("\n\n");

        // Add the extracted data from SPARQL queries
        prompt.push_str("## Extracted Data from Knowledge Graph\n\n");

        for (query_id, results) in query_results {
            prompt.push_str(&format!("### Query: {}\n", query_id));

            match results {
                SimpleSparqlResults::Solutions(rows) => {
                    if rows.is_empty() {
                        prompt.push_str("No results found.\n\n");
                    } else {
                        // Format the results as a readable list
                        for (idx, row) in rows.iter().enumerate() {
                            prompt.push_str(&format!("Result {}:\n", idx + 1));
                            for (key, value) in row {
                                prompt.push_str(&format!("  - {}: {}\n", key, value));
                            }
                        }
                        prompt.push_str("\n");
                    }
                }
                SimpleSparqlResults::Boolean(result) => {
                    prompt.push_str(&format!("Boolean result: {}\n\n", result));
                }
            }
        }

        // Add any additional instructions
        if let Some(instructions) = instructions {
            prompt.push_str("## Additional Instructions\n\n");
            prompt.push_str(instructions);
            prompt.push_str("\n\n");
        }

        prompt.push_str("## Task\n\n");
        prompt.push_str("Populate the template above with the actual data from the knowledge graph. ");
        prompt.push_str("Replace all placeholder fields with real values from the extracted data. ");
        prompt.push_str("Make the report professional and actionable.\n");

        Ok(prompt)
    }

    /// Extracts fields from a template that need to be populated
    pub fn extract_template_fields(template: &str) -> Vec<TemplateField> {
        let mut fields = Vec::new();
        let field_regex = regex::Regex::new(r"\[FIELD: ([^\]]+)\]").unwrap();
        let dropdown_regex = regex::Regex::new(r"\[DROPDOWN: ([^\]]+)\]").unwrap();
        let date_regex = regex::Regex::new(r"\[DATE FIELD\]").unwrap();
        let checklist_regex = regex::Regex::new(r"\[CHECKLIST: ([^\]]+)\]").unwrap();
        let textarea_regex = regex::Regex::new(r"\[TEXT AREA: ([^\]]+)\]").unwrap();

        // Extract regular fields
        for cap in field_regex.captures_iter(template) {
            fields.push(TemplateField {
                field_name: cap[1].to_string(),
                field_type: "text".to_string(),
                description: format!("Text field: {}", &cap[1]),
                required: true,
            });
        }

        // Extract dropdown fields
        for cap in dropdown_regex.captures_iter(template) {
            fields.push(TemplateField {
                field_name: format!("Dropdown: {}", &cap[1]),
                field_type: "dropdown".to_string(),
                description: format!("Select from: {}", &cap[1]),
                required: true,
            });
        }

        // Extract date fields
        for _ in date_regex.find_iter(template) {
            fields.push(TemplateField {
                field_name: "Date".to_string(),
                field_type: "date".to_string(),
                description: "Date field".to_string(),
                required: false,
            });
        }

        // Extract checklist fields
        for cap in checklist_regex.captures_iter(template) {
            fields.push(TemplateField {
                field_name: format!("Checklist: {}", &cap[1]),
                field_type: "checklist".to_string(),
                description: format!("Multiple selection: {}", &cap[1]),
                required: false,
            });
        }

        // Extract text area fields
        for cap in textarea_regex.captures_iter(template) {
            fields.push(TemplateField {
                field_name: cap[1].to_string(),
                field_type: "textarea".to_string(),
                description: format!("Long text: {}", &cap[1]),
                required: false,
            });
        }

        fields
    }
}