#!/bin/bash

# RDF Knowledge Extractor - Complete Company Intelligence Workflow
# This script demonstrates the full pipeline from data extraction to report generation

echo "========================================="
echo "RDF Knowledge Extractor - Company Intelligence Pipeline"
echo "========================================="
echo ""

# Step 1: Ensure vLLM server is running
echo "Step 1: Checking vLLM server connection..."
echo "----------------------------------------"
cargo run -- check-server --server-url http://localhost:8000
echo ""

# Step 2: Extract knowledge from company intelligence documents
echo "Step 2: Extracting RDF triples from company intelligence data..."
echo "----------------------------------------"
echo "Processing: examples/acme_corp_intelligence.txt"
echo ""

cargo run -- extract \
  -c company_intelligence_config.yaml \
  -i examples/acme_corp_intelligence.txt \
  --kg-path company_intelligence.db \
  -o extracted_triples.ttl \
  --format turtle \
  --validate \
  --server-url http://localhost:8000

echo ""
echo "Extraction complete! Triples saved to:"
echo "  - Knowledge Graph: company_intelligence.db"
echo "  - RDF File: extracted_triples.ttl"
echo ""

# Step 3: Display sample of extracted RDF triples in Turtle format
echo "Step 3: Sample of extracted RDF triples (Turtle format)..."
echo "----------------------------------------"
head -50 extracted_triples.ttl
echo ""

# Step 4: Query the knowledge graph with SPARQL
echo "Step 4: Querying knowledge graph with SPARQL..."
echo "----------------------------------------"
echo ""

# Query 1: Get all companies and their priority scores
echo "Query 1: Companies and Priority Scores"
cargo run -- query \
  -k company_intelligence.db \
  --query "PREFIX sales: <http://sales.intelligence.org/ontology#> PREFIX foaf: <http://xmlns.com/foaf/0.1/> SELECT ?company ?name ?score WHERE { ?company a foaf:Organization ; foaf:name ?name ; sales:hasPriorityScore ?score } ORDER BY DESC(?score)" \
  --format table

echo ""

# Query 2: Get all decision makers
echo "Query 2: Decision Makers"
cargo run -- query \
  -k company_intelligence.db \
  --query "PREFIX foaf: <http://xmlns.com/foaf/0.1/> PREFIX sales: <http://sales.intelligence.org/ontology#> SELECT ?person ?name ?title ?email WHERE { ?person a sales:DecisionMaker ; foaf:name ?name ; foaf:title ?title . OPTIONAL { ?person foaf:mbox ?email } }" \
  --format table

echo ""

# Query 3: Get pain points
echo "Query 3: Business Pain Points"
cargo run -- query \
  -k company_intelligence.db \
  --query "PREFIX sales: <http://sales.intelligence.org/ontology#> PREFIX foaf: <http://xmlns.com/foaf/0.1/> SELECT ?company ?name ?pain WHERE { ?company a foaf:Organization ; foaf:name ?name ; sales:hasPainPoint ?pain }" \
  --format table

echo ""

# Step 5: Generate the sales intelligence report from template
echo "Step 5: Generating Company Intelligence Report..."
echo "----------------------------------------"
cargo run -- generate \
  -c company_intelligence_config.yaml \
  -k company_intelligence.db \
  -t templates/company_intelligence_report.yaml \
  -o generated_intelligence_report.md

echo ""
echo "Report generated: generated_intelligence_report.md"
echo ""

# Step 6: Display the generated report
echo "Step 6: Generated Company Intelligence Report Preview..."
echo "----------------------------------------"
head -100 generated_intelligence_report.md
echo ""

# Step 7: Export knowledge in different formats
echo "Step 7: Exporting knowledge in multiple formats..."
echo "----------------------------------------"

# Export as JSON-LD
cargo run -- export \
  -k company_intelligence.db \
  -c company_intelligence_config.yaml \
  -o company_intelligence.jsonld \
  --format json-ld

echo "Exported to JSON-LD: company_intelligence.jsonld"

# Export as N-Triples
cargo run -- export \
  -k company_intelligence.db \
  -c company_intelligence_config.yaml \
  -o company_intelligence.nt \
  --format n-triples

echo "Exported to N-Triples: company_intelligence.nt"
echo ""

# Step 8: Display knowledge graph statistics
echo "Step 8: Knowledge Graph Statistics..."
echo "----------------------------------------"
cargo run -- stats -k company_intelligence.db -c company_intelligence_config.yaml
echo ""

echo "========================================="
echo "Workflow Complete!"
echo "========================================="
echo ""
echo "Generated Artifacts:"
echo "  1. Knowledge Graph: company_intelligence.db"
echo "  2. RDF Triples (Turtle): extracted_triples.ttl"
echo "  3. RDF Triples (JSON-LD): company_intelligence.jsonld"
echo "  4. RDF Triples (N-Triples): company_intelligence.nt"
echo "  5. Intelligence Report: generated_intelligence_report.md"
echo ""
echo "The knowledge graph can now be:"
echo "  - Queried with SPARQL for specific information"
echo "  - Used to generate additional reports from different templates"
echo "  - Enriched with data from additional documents"
echo "  - Integrated with other semantic web applications"