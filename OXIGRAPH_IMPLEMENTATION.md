# Oxigraph Implementation - Real RDF Storage and SPARQL

## Overview

I've successfully implemented Oxigraph as the RDF triplestore, replacing the simplified JSON storage with a real RDF database that supports full SPARQL 1.1 queries.

## What Changed

### 1. **Dependencies** (`Cargo.toml`)
```toml
# Added real RDF processing
oxigraph = "0.4"
sparesults = "0.2"
```

### 2. **Knowledge Graph** (`src/knowledge_graph/mod.rs`)

#### Key Features:
- **Real RDF Store**: Uses Oxigraph's native Store for persistent RDF storage
- **Full SPARQL Support**: Execute any SPARQL 1.1 query including SELECT, CONSTRUCT, ASK, DESCRIBE
- **SPARQL Updates**: Support for INSERT, DELETE, and other update operations
- **Namespace Management**: Automatic prefix handling for FOAF, Schema.org, CCO, etc.
- **Multiple Export Formats**: Turtle, JSON-LD, N-Triples, RDF/XML

#### Core Methods:

```rust
pub struct KnowledgeGraph {
    store: Store,  // Oxigraph's native RDF store
    config: KnowledgeGraphConfig,
    schema: RdfSchema,
    namespaces: HashMap<String, String>,
}

// Add RDF triples to the store
pub fn add_triples(&mut self, triples: &[RdfTriple]) -> Result<usize>

// Execute real SPARQL queries
pub fn execute_sparql(&self, query: &str) -> Result<SimpleSparqlResults>

// Execute SPARQL UPDATE statements
pub fn execute_update(&mut self, update: &str) -> Result<()>

// Export in various formats
pub fn export_turtle(&self) -> Result<String>
pub fn export_jsonld(&self) -> Result<String>
```

### 3. **Serialization Utils** (`src/utils/serialization.rs`)

Updated to work with Oxigraph's export methods:
- Direct serialization from KnowledgeGraph using Oxigraph's native formats
- Support for all standard RDF serialization formats
- Proper namespace handling for FOAF, CCO, Schema.org

## How It Works Now

### 1. **Extraction Phase**
```rust
// LLM extracts triples from documents
let extractor = RdfExtractor::new(config, llm_client);
let results = extractor.extract_from_multiple(documents).await?;

// Store in Oxigraph (real RDF storage)
let mut knowledge_graph = KnowledgeGraph::new(kg_config, schema)?;
knowledge_graph.add_triples(&results.triples)?;
```

### 2. **Storage**
- Triples are stored as proper RDF quads in Oxigraph
- Persistent storage to disk or in-memory options
- Full URI expansion with namespace support
- Automatic type detection (URI vs Literal)

### 3. **SPARQL Queries**
```sparql
# Real SPARQL queries now work!
PREFIX foaf: <http://xmlns.com/foaf/0.1/>
PREFIX sales: <http://sales.intelligence.org/ontology#>

SELECT ?company ?name ?score
WHERE {
  ?company a foaf:Organization ;
           foaf:name ?name ;
           sales:hasPriorityScore ?score .
}
ORDER BY DESC(?score)
```

### 4. **Export Formats**
All standard RDF formats are supported:
- **Turtle**: Human-readable with prefixes
- **JSON-LD**: Web-friendly with context
- **N-Triples**: Simple line-based format
- **RDF/XML**: XML-based standard

## Example Usage

### Extract and Store
```bash
cargo run -- extract \
  -c company_intelligence_config.yaml \
  -i examples/acme_corp_intelligence.txt \
  --kg-path company_intelligence.db
```

### Query with Real SPARQL
```bash
# Complex SPARQL query
cargo run -- query \
  -k company_intelligence.db \
  --query "
    PREFIX foaf: <http://xmlns.com/foaf/0.1/>
    PREFIX org: <http://www.w3.org/ns/org#>

    SELECT ?person ?name ?title ?company
    WHERE {
      ?person a foaf:Person ;
              foaf:name ?name ;
              foaf:title ?title ;
              org:memberOf ?company .
      ?company foaf:name ?company_name .
      FILTER(CONTAINS(?title, 'Chief'))
    }
  "
```

### SPARQL Updates
```rust
// Insert new triples
knowledge_graph.execute_update("
  INSERT DATA {
    <http://example.org/company/newco>
      a foaf:Organization ;
      foaf:name 'NewCo Inc.' ;
      sales:hasPriorityScore 85 .
  }
")?;

// Delete triples
knowledge_graph.execute_update("
  DELETE WHERE {
    ?s sales:hasRelationshipTemperature 'Cold' .
  }
")?;
```

## Benefits of Oxigraph

1. **Real RDF Compliance**: Full W3C RDF and SPARQL 1.1 compliance
2. **Performance**: Fast native Rust implementation
3. **Persistence**: Reliable disk-based storage with ACID properties
4. **Full SPARQL**: Support for all SPARQL features including:
   - Aggregations (COUNT, SUM, AVG)
   - Subqueries
   - FILTER expressions
   - OPTIONAL patterns
   - Property paths
   - Named graphs
5. **Interoperability**: Export to any RDF format for use with other tools

## Integration with FOAF and CCO

The system now properly uses standard ontologies:

```rust
// Automatic namespace registration
namespaces.insert("foaf", "http://xmlns.com/foaf/0.1/");
namespaces.insert("cco", "https://www.commoncoreontologies.org/");
namespaces.insert("schema", "https://schema.org/");
namespaces.insert("org", "http://www.w3.org/ns/org#");

// Automatic URI expansion
"foaf:Person" → "http://xmlns.com/foaf/0.1/Person"
"schema:Organization" → "https://schema.org/Organization"
```

## What This Enables

1. **Complex Queries**: Full SPARQL means you can do joins, aggregations, and complex graph patterns
2. **Standards Compliance**: RDF data is fully portable to other semantic web tools
3. **Reasoning**: Can be extended with inference rules and OWL reasoning
4. **Federation**: Can query across multiple RDF sources
5. **Linked Data**: Properly formatted URIs enable linking to external knowledge bases

## Testing the Implementation

```bash
# 1. Extract RDF from documents (requires vLLM server)
cargo run -- extract -c config.yaml -i document.txt

# 2. Query the knowledge graph
cargo run -- query -k knowledge_graph.db \
  --query "SELECT * WHERE { ?s ?p ?o } LIMIT 10"

# 3. Export to different formats
cargo run -- export -k knowledge_graph.db \
  --format turtle -o output.ttl

# 4. Get statistics
cargo run -- stats -k knowledge_graph.db
```

## Next Steps

With Oxigraph implemented, you can now:
1. Use full SPARQL for complex queries
2. Integrate with other RDF tools and databases
3. Implement reasoning and inference
4. Build SPARQL endpoints for web access
5. Link to public knowledge graphs like DBpedia and Wikidata