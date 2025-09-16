#!/bin/bash

# Sales Intelligence Template Filling Example
echo "🔍 Sales Intelligence Template Filling Example"
echo "============================================="

# Step 1: Extract RDF triples from source data
echo "📄 Step 1: Extracting knowledge from sales data..."
export PATH="$HOME/.cargo/bin:$PATH"
cargo run -- extract \
  --config example-sales-2/sales_intelligence_config.yaml \
  --input example-sales-2/source-data/crm_company_export.txt \
  --input example-sales-2/source-data/linkedin_sales_navigator_export.txt \
  --kg-path example-sales-2/knowledge_graph.db \
  --output example-sales-2/knowledge_graph.nt \
  --format n-triples \
  --server-url http://localhost:8000 \
  --merge

# Step 2: Generate template-based report
echo "📋 Step 2: Generating target companies report..."
cargo run -- generate \
  --config example-sales-2/sales_intelligence_config.yaml \
  --kg-path example-sales-2/knowledge_graph.db \
  --template templates/target_companies_template.yaml \
  --output example-sales-2/target_companies_report.md \
  --server-url http://localhost:8000 \
  --enhance

echo "✅ Complete! Check example-sales-2/target_companies_report.md"