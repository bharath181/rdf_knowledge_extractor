use anyhow::{Result, Context};
use std::collections::HashMap;

use crate::config::OutputFormat;
use crate::core::RdfTriple;

pub struct RdfSerializer;

impl RdfSerializer {
    pub fn new() -> Self {
        Self
    }

    pub fn serialize(
        &mut self,
        triples: &[RdfTriple],
        format: &OutputFormat,
        namespace: &str,
        prefix: &str,
    ) -> Result<String> {
        match format {
            OutputFormat::Turtle => self.serialize_turtle(triples, namespace, prefix),
            OutputFormat::JsonLd => self.serialize_json_ld(triples, namespace, prefix),
            OutputFormat::NTriples => self.serialize_ntriples(triples),
            OutputFormat::RdfXml => self.serialize_rdf_xml(triples, namespace, prefix),
            OutputFormat::Json => self.serialize_json(triples),
        }
    }

    fn serialize_turtle(&self, triples: &[RdfTriple], namespace: &str, prefix: &str) -> Result<String> {
        let mut output = String::new();

        // Add prefix declarations
        output.push_str(&format!("@prefix {}: <{}> .\n", prefix, namespace));
        output.push_str("@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n");
        output.push_str("@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\n");

        // Add triples
        for triple in triples {
            let subject = self.format_uri_for_turtle(&triple.subject, namespace, prefix);
            let predicate = self.format_uri_for_turtle(&triple.predicate, namespace, prefix);
            let object = self.format_object_for_turtle(&triple.object);

            output.push_str(&format!("{} {} {} .\n", subject, predicate, object));
        }

        Ok(output)
    }

    fn serialize_json_ld(&self, triples: &[RdfTriple], namespace: &str, prefix: &str) -> Result<String> {
        let mut context = serde_json::Map::new();
        context.insert(prefix.to_string(), serde_json::Value::String(namespace.to_string()));

        let mut graph = Vec::new();
        let mut subjects: HashMap<String, serde_json::Map<String, serde_json::Value>> = HashMap::new();

        for triple in triples {
            let subject_entry = subjects.entry(triple.subject.clone()).or_insert_with(|| {
                let mut map = serde_json::Map::new();
                map.insert("@id".to_string(), serde_json::Value::String(triple.subject.clone()));
                map
            });

            let predicate_key = if triple.predicate.starts_with(namespace) {
                format!("{}:{}", prefix, &triple.predicate[namespace.len()..])
            } else {
                triple.predicate.clone()
            };

            let object_value = if triple.object.starts_with("http://") || triple.object.starts_with("https://") {
                serde_json::json!({"@id": triple.object})
            } else {
                serde_json::Value::String(triple.object.clone())
            };

            subject_entry.insert(predicate_key, object_value);
        }

        for (_, subject_data) in subjects {
            graph.push(serde_json::Value::Object(subject_data));
        }

        let json_ld = serde_json::json!({
            "@context": context,
            "@graph": graph
        });

        serde_json::to_string_pretty(&json_ld)
            .context("Failed to serialize JSON-LD")
    }

    fn serialize_ntriples(&self, triples: &[RdfTriple]) -> Result<String> {
        let mut output = String::new();

        for triple in triples {
            let subject = format!("<{}>", triple.subject);
            let predicate = format!("<{}>", triple.predicate);
            let object = if triple.object.starts_with("http://") || triple.object.starts_with("https://") {
                format!("<{}>", triple.object)
            } else {
                format!("\"{}\"", triple.object.replace("\"", "\\\""))
            };

            output.push_str(&format!("{} {} {} .\n", subject, predicate, object));
        }

        Ok(output)
    }

    fn serialize_rdf_xml(&self, triples: &[RdfTriple], namespace: &str, prefix: &str) -> Result<String> {
        let mut output = String::new();

        // XML header and RDF root
        output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        output.push_str(&format!(
            "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\" xmlns:{}=\"{}\">\n",
            prefix, namespace
        ));

        // Group triples by subject
        let mut subjects: HashMap<String, Vec<&RdfTriple>> = HashMap::new();
        for triple in triples {
            subjects.entry(triple.subject.clone()).or_default().push(triple);
        }

        // Generate RDF/XML for each subject
        for (subject, subject_triples) in subjects {
            output.push_str(&format!("  <rdf:Description rdf:about=\"{}\">\n", subject));

            for triple in subject_triples {
                let predicate_name = if triple.predicate.starts_with(namespace) {
                    format!("{}:{}", prefix, &triple.predicate[namespace.len()..])
                } else {
                    triple.predicate.split('#').last().unwrap_or(&triple.predicate).to_string()
                };

                if triple.object.starts_with("http://") || triple.object.starts_with("https://") {
                    output.push_str(&format!("    <{} rdf:resource=\"{}\"/>\n", predicate_name, triple.object));
                } else {
                    output.push_str(&format!("    <{}>{}</{}>\n",
                        predicate_name,
                        html_escape::encode_text(&triple.object),
                        predicate_name
                    ));
                }
            }

            output.push_str("  </rdf:Description>\n");
        }

        output.push_str("</rdf:RDF>\n");

        Ok(output)
    }

    fn serialize_json(&self, triples: &[RdfTriple]) -> Result<String> {
        serde_json::to_string_pretty(triples)
            .context("Failed to serialize to JSON")
    }

    fn format_uri_for_turtle(&self, uri: &str, namespace: &str, prefix: &str) -> String {
        if uri.starts_with(namespace) {
            format!("{}:{}", prefix, &uri[namespace.len()..])
        } else {
            format!("<{}>", uri)
        }
    }

    fn format_object_for_turtle(&self, object: &str) -> String {
        if object.starts_with("http://") || object.starts_with("https://") {
            format!("<{}>", object)
        } else {
            format!("\"{}\"", object.replace("\"", "\\\""))
        }
    }
}

pub fn validate_rdf_triples(triples: &[RdfTriple]) -> Vec<String> {
    let mut issues = Vec::new();

    for (i, triple) in triples.iter().enumerate() {
        // Validate subject URI
        if !triple.subject.starts_with("http://") && !triple.subject.starts_with("https://") {
            issues.push(format!("Triple {}: Invalid subject URI: {}", i, triple.subject));
        }

        // Validate predicate URI
        if !triple.predicate.starts_with("http://") && !triple.predicate.starts_with("https://") {
            issues.push(format!("Triple {}: Invalid predicate URI: {}", i, triple.predicate));
        }

        // Check for empty values
        if triple.subject.is_empty() {
            issues.push(format!("Triple {}: Empty subject", i));
        }
        if triple.predicate.is_empty() {
            issues.push(format!("Triple {}: Empty predicate", i));
        }
        if triple.object.is_empty() {
            issues.push(format!("Triple {}: Empty object", i));
        }
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_rdf_triples() {
        let triples = vec![
            RdfTriple::new(
                "http://example.org/person1".to_string(),
                "http://example.org/hasName".to_string(),
                "John Doe".to_string(),
            ),
            RdfTriple::new(
                "invalid_uri".to_string(),
                "http://example.org/hasAge".to_string(),
                "30".to_string(),
            ),
        ];

        let issues = validate_rdf_triples(&triples);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].contains("Invalid subject URI"));
    }

    #[test]
    fn test_serialize_json() {
        let mut serializer = RdfSerializer::new();
        let triples = vec![
            RdfTriple::new(
                "http://example.org/person1".to_string(),
                "http://example.org/hasName".to_string(),
                "John Doe".to_string(),
            ),
        ];

        let result = serializer.serialize(
            &triples,
            &OutputFormat::Json,
            "http://example.org/",
            "ex"
        );

        assert!(result.is_ok());
    }
}