#!/bin/bash

# Sales Intelligence Workflow using RDF Knowledge Extractor
# This script demonstrates the complete workflow from source data to populated template

set -e

echo "=== Sales Intelligence Workflow ==="
echo "Using existing RDF Knowledge Extractor to process sales data"
echo

# Configuration
KNOWLEDGE_GRAPH="example-sales/sales_intelligence.db"
CONFIG_FILE="example-sales/extraction_config.yaml"
ONTOLOGY_FILE="example-sales/sales_intelligence_ontology.ttl"
TEMPLATE_FILE="example-sales/templates/company_intelligence_template.md"
OUTPUT_DIR="example-sales/generated-reports"
SOURCE_DATA_DIR="example-sales/source-data"

# Ensure output directory exists
mkdir -p "$OUTPUT_DIR"

echo "Step 1: Extract RDF triples from source data using LLM"
echo "Processing files in $SOURCE_DATA_DIR..."

# Process each source file and extract RDF triples
for source_file in "$SOURCE_DATA_DIR"/*.txt "$SOURCE_DATA_DIR"/*.md "$SOURCE_DATA_DIR"/*.json; do
    if [ -f "$source_file" ]; then
        echo "  Processing: $(basename "$source_file")"

        # Use the existing RDF extractor tool
        cargo run -- extract \
            --config "$CONFIG_FILE" \
            --input "$source_file" \
            --output "example-sales/rdf-output/$(basename "$source_file" .txt).ttl" \
            --knowledge-graph "$KNOWLEDGE_GRAPH"
    fi
done

echo
echo "Step 2: Load ontology into knowledge graph"
cargo run -- load \
    --knowledge-graph "$KNOWLEDGE_GRAPH" \
    --rdf-file "$ONTOLOGY_FILE"

echo
echo "Step 3: Query knowledge graph for template data"

# Get all company data as JSON for LLM processing
echo "  Querying top companies..."
COMPANIES_DATA=$(cargo run -- query \
    --knowledge-graph "$KNOWLEDGE_GRAPH" \
    --query "SELECT ?company ?name ?score ?industry WHERE {
        ?company a foaf:Organization ;
                 foaf:name ?name ;
                 si:hasPriorityScore ?score .
        OPTIONAL { ?company schema:industry ?industry }
    } ORDER BY DESC(?score) LIMIT 5" \
    --format json)

echo "  Querying complete company profiles..."
FULL_DATA=$(cargo run -- query \
    --knowledge-graph "$KNOWLEDGE_GRAPH" \
    --query "SELECT * WHERE {
        ?s ?p ?o .
        ?s a foaf:Organization
    }" \
    --format json)

echo
echo "Step 4: Use LLM to populate template with queried data"

# Create a prompt for the LLM to fill the template
POPULATE_PROMPT="You are a sales intelligence analyst. Use the provided RDF query results to populate the Company Intelligence Report template.

SPARQL Query Results:
$FULL_DATA

Template to populate:
$(cat "$TEMPLATE_FILE")

Instructions:
1. Extract the top 5 companies by priority score
2. For each company, fill in ALL template fields using the RDF data
3. If data is missing, use 'Unknown' or appropriate default values
4. Maintain the exact template format and structure
5. Replace all {{FIELD}} placeholders with actual data
6. Ensure the report is professional and complete

Generate the complete populated report:"

# Use the LLM to populate the template (using existing LLM integration)
echo "  Generating populated report..."
cargo run -- generate \
    --config "$CONFIG_FILE" \
    --prompt "$POPULATE_PROMPT" \
    --output "$OUTPUT_DIR/company_intelligence_report_$(date +%Y%m%d_%H%M%S).md"

echo
echo "Step 5: Validate and export results"

# Export knowledge graph for review
echo "  Exporting knowledge graph as Turtle..."
cargo run -- export \
    --knowledge-graph "$KNOWLEDGE_GRAPH" \
    --format turtle \
    --output "$OUTPUT_DIR/sales_knowledge_graph.ttl"

echo "  Exporting knowledge graph as JSON-LD..."
cargo run -- export \
    --knowledge-graph "$KNOWLEDGE_GRAPH" \
    --format jsonld \
    --output "$OUTPUT_DIR/sales_knowledge_graph.jsonld"

# Generate statistics
echo "  Generating knowledge graph statistics..."
cargo run -- stats \
    --knowledge-graph "$KNOWLEDGE_GRAPH" \
    --output "$OUTPUT_DIR/kg_statistics.json"

echo
echo "=== Workflow Complete ==="
echo "Generated files:"
echo "  - Populated report: $OUTPUT_DIR/company_intelligence_report_*.md"
echo "  - Knowledge graph (Turtle): $OUTPUT_DIR/sales_knowledge_graph.ttl"
echo "  - Knowledge graph (JSON-LD): $OUTPUT_DIR/sales_knowledge_graph.jsonld"
echo "  - Statistics: $OUTPUT_DIR/kg_statistics.json"
echo

echo "Alternative: Manual LLM Template Population"
echo "==========================================="
echo "You can also directly ask the LLM to populate the template:"
echo
echo "1. Run SPARQL queries to get data:"
echo "   cargo run -- query -k $KNOWLEDGE_GRAPH --query 'SELECT * WHERE { ?s ?p ?o }' --format json > all_data.json"
echo
echo "2. Give LLM the data + template:"
echo "   'Here is RDF data: [paste all_data.json]"
echo "    Here is template: [paste template]"
echo "    Please populate the template with the RDF data.'"
echo
echo "This approach lets the LLM intelligently map RDF data to template fields!"