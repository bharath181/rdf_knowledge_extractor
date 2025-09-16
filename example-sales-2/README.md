# Sales Intelligence Template Filling Example

This example demonstrates the complete workflow for using the RDF Knowledge Extractor to auto-populate sales templates with intelligence from various data sources.

## Overview

The system takes unstructured sales data (CRM exports, LinkedIn Sales Navigator data, etc.) and converts it into an RDF knowledge graph. This knowledge graph is then queried to automatically fill sales intelligence templates.

## Architecture

1. **Data Sources** → **RDF Extraction** → **Knowledge Graph** → **Template Filling** → **Intelligence Reports**

2. **Configuration-Driven**: All template logic is defined in YAML configuration files, not hard-coded

3. **LLM-Powered**: Uses vLLM for both RDF triple generation and intelligent template filling

## Files

### Source Data
- `source-data/crm_company_export.txt` - Synthetic CRM data with 5 target companies
- `source-data/linkedin_sales_navigator_export.txt` - Social selling insights and executive activity

### Configuration
- `sales_intelligence_config.yaml` - RDF extraction configuration with ontology and questions
- `../templates/target_companies_template.yaml` - Template configuration with SPARQL queries

### Generated Output
- `knowledge_graph.nt` - Extracted RDF triples in N-Triples format
- `target_companies_report.md` - Auto-generated sales intelligence report

## Workflow

### Step 1: RDF Extraction
```bash
cargo run --bin extract \
  --config example-sales-2/sales_intelligence_config.yaml \
  --input example-sales-2/source-data/ \
  --output example-sales-2/knowledge_graph.nt
```

The extractor:
- Reads all files in `source-data/`
- Applies competency questions to extract structured information
- Generates RDF triples aligned to the sales intelligence ontology
- Outputs N-Triples format for the knowledge graph

### Step 2: Template Generation
```bash
cargo run --bin generate-template \
  --template templates/target_companies_template.yaml \
  --output example-sales-2/target_companies_report.md
```

The template engine:
- Executes SPARQL queries defined in the template configuration
- Passes raw query results + template to the LLM
- LLM intelligently maps data to template structure
- Generates the final formatted report

## Key Features

- **No Hard-Coding**: Templates are purely configuration-driven
- **Intelligent Mapping**: LLM handles complex data structuring and synthesis
- **Extensible**: Easy to add new data sources, ontologies, and templates
- **Standards-Compliant**: Proper RDF/SPARQL with configurable validation

## Example Template Structure

The target companies template generates:
- Company intelligence (revenue, employees, tech stack)
- Key personnel profiles (decision makers, influencers)
- Engagement history and contact preferences
- Competitive positioning and contract opportunities

## Quick Start

```bash
cd example-sales-2
./run_example.sh
```

This will run the complete workflow and generate the sales intelligence report.