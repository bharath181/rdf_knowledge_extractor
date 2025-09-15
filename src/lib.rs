pub mod config;
pub mod core;
pub mod handlers;
pub mod utils;
pub mod knowledge_graph;
pub mod templates;

pub use config::Configuration;
pub use core::{RdfExtractor, ExtractionResult};
pub use handlers::DocumentProcessor;
pub use knowledge_graph::KnowledgeGraph;
pub use templates::TemplateManager;