# Sales Intelligence Workflow Demonstration

This demonstrates the complete workflow using the existing RDF Knowledge Extractor tool.

## Workflow Overview

1. **Source Data → RDF Extraction → Knowledge Graph → SPARQL Queries → LLM Template Population**

## Step 1: Source Data Processing

### Sample Input Data
We have sample files in `source-data/`:
- `sample_company_data.txt` - ACME Corporation intelligence
- `techcorp_intelligence.txt` - TechCorp Industries intelligence

### RDF Extraction (Using existing tool)
```bash
# Extract RDF triples from source data
cargo run -- extract \
    --config example-sales/extraction_config.yaml \
    --input example-sales/source-data/sample_company_data.txt \
    --output example-sales/rdf-output/acme_corp.ttl

# This would generate RDF triples like:
```

### Expected RDF Output
```turtle
@prefix foaf: <http://xmlns.com/foaf/0.1/> .
@prefix si: <http://sales.intelligence.org/ontology#> .
@prefix company: <http://sales.intelligence.org/data/company/> .
@prefix person: <http://sales.intelligence.org/data/person/> .

# Company data
company:ACME_Corporation foaf:name "ACME Corporation" .
company:ACME_Corporation schema:industry "Software & Technology" .
company:ACME_Corporation si:hasPriorityScore 85 .
company:ACME_Corporation si:hasRevenueRange "$50-100M" .
company:ACME_Corporation si:hasEmployeeCount 1200 .
company:ACME_Corporation si:usesTechnology "Salesforce Enterprise" .
company:ACME_Corporation si:usesTechnology "AWS" .
company:ACME_Corporation si:hasRecentActivity "Series C funding: $75M" .
company:ACME_Corporation si:hasPainPoint "Legacy system integration issues" .

# Decision Maker data
person:SarahJohnson foaf:name "Sarah Johnson" .
person:SarahJohnson foaf:title "CEO" .
person:SarahJohnson foaf:mbox "sarah.johnson@acme.com" .
person:SarahJohnson foaf:account "linkedin.com/in/sarah-johnson-ceo" .
person:SarahJohnson org:memberOf company:ACME_Corporation .
person:SarahJohnson si:hasContactPreference "LinkedIn" .
person:SarahJohnson foaf:interest "AI/ML" .
person:SarahJohnson si:hasRecentPost "Posted about AI ethics" .

# Contact history
company:ACME_Corporation si:hasResponseRate 45.0 .
company:ACME_Corporation si:hasRelationshipTemperature "Warm" .
company:ACME_Corporation si:hasContactNotes "Interested in analytics platform" .

# Competitive info
company:ACME_Corporation si:hasCurrentSolution "Salesforce" .
company:ACME_Corporation si:hasContractRenewal "2024-12-31" .
company:ACME_Corporation si:hasCompetitiveAdvantage "Better API integration than Microsoft" .
```

## Step 2: SPARQL Queries for Data Retrieval

### Query Top Companies
```sparql
SELECT ?company ?name ?score ?industry
WHERE {
  ?company a foaf:Organization ;
           foaf:name ?name ;
           si:hasPriorityScore ?score .
  OPTIONAL { ?company schema:industry ?industry }
}
ORDER BY DESC(?score)
LIMIT 5
```

### Query Results (JSON)
```json
{
  "bindings": [
    {
      "company": {"value": "http://sales.intelligence.org/data/company/TechCorp_Industries"},
      "name": {"value": "TechCorp Industries"},
      "score": {"value": "92"},
      "industry": {"value": "Manufacturing Technology"}
    },
    {
      "company": {"value": "http://sales.intelligence.org/data/company/ACME_Corporation"},
      "name": {"value": "ACME Corporation"},
      "score": {"value": "85"},
      "industry": {"value": "Software & Technology"}
    }
  ]
}
```

## Step 3: LLM Template Population

### LLM Prompt
```
You are a sales intelligence analyst. Use the provided RDF query results to populate the Company Intelligence Report template.

SPARQL Query Results:
[Full JSON data from knowledge graph]

Template to populate:
[Company Intelligence Template]

Instructions:
1. Extract the top 5 companies by priority score
2. For each company, fill in ALL template fields using the RDF data
3. If data is missing, use 'Unknown' or appropriate default values
4. Replace all {{FIELD}} placeholders with actual data

Generate the complete populated report.
```

### Expected Populated Output
```markdown
# Company Intelligence Report

Generated on: 2024-01-30 14:30:00

## Target Companies Section

### Company 1 Template
**Company Name:** TechCorp Industries
**Priority Score:** 92/100 | **Industry:** Manufacturing Technology

#### Company Intelligence
- **Revenue:** $200-500M | **Employees:** 3,500
- **Technology Stack:** SAP S/4HANA, Google Cloud Platform, Siemens PLM
- **Recent Funding/News:** IPO filing submitted, $200M facility expansion
- **Pain Points/Triggers:** Manufacturing data integration, quality control delays

#### Key Attendees & Profiles
**Decision Maker:** Robert Martinez - CEO & Founder
- **LinkedIn:** linkedin.com/in/robert-martinez-ceo
- **Contact Preference:** Phone
- **Interests:** Industry 4.0, automation, workforce development
- **Recent Activity:** Keynote at Manufacturing Summit Detroit

**Influencer:** Dr. Ahmed Hassan - VP of Operations
- **Department:** Manufacturing Operations
- **Decision Influence:** 80%
- **Previous Interactions:** Attended our webinar on manufacturing analytics

#### Historical Contact Summary
- **Total Touchpoints (12mo):** 8 via Email, Phone, In-person
- **Last Contact:** 2024-01-28 by Sarah Chen
- **Previous Response Rate:** 62%
- **Relationship Temperature:** Hot

#### Competitive Intelligence
- **Current Solutions:** SAP Analytics Cloud
- **Contract Renewal:** 2024-06-01
- **Competitive Advantage:** Manufacturing expertise, Siemens connectors

---

### Company 2 Template
**Company Name:** ACME Corporation
**Priority Score:** 85/100 | **Industry:** Software & Technology
[... and so on for remaining companies]
```

## Key Benefits of This Approach

1. **Leverages Existing Tool**: Uses the robust RDF Knowledge Extractor already built
2. **Ontology-Driven**: Ensures consistent data modeling across different sources
3. **LLM Intelligence**: The LLM handles complex mapping and fills gaps intelligently
4. **Scalable**: Can process multiple data sources and combine them in the knowledge graph
5. **Queryable**: SPARQL allows complex queries across the entire knowledge base
6. **Template Flexibility**: LLM can adapt to different template formats

## Running the Complete Workflow

```bash
# Run the complete workflow
./example-sales/workflow_example.sh

# Or run individual steps:
# 1. Extract data to RDF
cargo run -- extract --config example-sales/extraction_config.yaml --input source-data/ --output rdf-output/

# 2. Build knowledge graph
cargo run -- load --knowledge-graph sales_intelligence.db --rdf-files rdf-output/*.ttl

# 3. Query for template data
cargo run -- query --knowledge-graph sales_intelligence.db --query-file sparql_queries/template_population_queries.sparql --format json

# 4. LLM populates template
# [Use the query results + template with your LLM]
```

This approach gives you a powerful, flexible system that can handle various data sources and automatically populate professional sales intelligence reports.