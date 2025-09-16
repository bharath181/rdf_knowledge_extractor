# Simple Sales Intelligence Workflow Demo

This demonstrates how the LLM does all the heavy lifting while the code is just a simple pipeline.

## Workflow Overview

```
Source Data → LLM → RDF Triples → Knowledge Graph → SPARQL Query → LLM → Populated Template
```

## Step 1: LLM Extracts RDF Triples from Source Data

### Input to LLM:
```
You are an RDF extraction expert. Convert this sales intelligence data to RDF triples using the sales intelligence ontology.

Use these prefixes:
- foaf: http://xmlns.com/foaf/0.1/
- si: http://sales.intelligence.org/ontology#
- company: http://sales.intelligence.org/data/company/
- person: http://sales.intelligence.org/data/person/

Source Data:
ACME Corporation - Sales Intelligence Data
==========================================
- Company Name: ACME Corporation
- Industry: Software & Technology
- Revenue: $50-100M annually
- Employees: 1,200
- Priority Score: 85/100
- Technology Stack: Salesforce Enterprise, AWS, Microsoft Azure
- Recent Activity: Series C funding round: $75M (March 2024)
- Pain Points: Legacy system integration issues causing data silos

Key Decision Makers:
1. Sarah Johnson - CEO
   - Email: sarah.johnson@acme.com
   - LinkedIn: linkedin.com/in/sarah-johnson-ceo
   - Contact Preference: LinkedIn messaging
   - Interests: AI/ML, sustainable technology

Generate RDF triples in Turtle format:
```

### LLM Output:
```turtle
@prefix foaf: <http://xmlns.com/foaf/0.1/> .
@prefix si: <http://sales.intelligence.org/ontology#> .
@prefix schema: <https://schema.org/> .
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
company:ACME_Corporation si:usesTechnology "Microsoft Azure" .
company:ACME_Corporation si:hasRecentActivity "Series C funding: $75M" .
company:ACME_Corporation si:hasPainPoint "Legacy system integration issues" .

# Decision maker data
person:SarahJohnson foaf:name "Sarah Johnson" .
person:SarahJohnson foaf:title "CEO" .
person:SarahJohnson foaf:mbox "sarah.johnson@acme.com" .
person:SarahJohnson foaf:account "linkedin.com/in/sarah-johnson-ceo" .
person:SarahJohnson org:memberOf company:ACME_Corporation .
person:SarahJohnson si:hasContactPreference "LinkedIn" .
person:SarahJohnson foaf:interest "AI/ML" .
person:SarahJohnson foaf:interest "sustainable technology" .
```

## Step 2: Store in Knowledge Graph (Rust tool)

```bash
# Simple command - code just loads the RDF
cargo run -- load --knowledge-graph sales.db --rdf-file extracted_data.ttl
```

## Step 3: Query Knowledge Graph (Rust tool)

```bash
# Get all company data as JSON
cargo run -- query --knowledge-graph sales.db --format json --query "
SELECT ?company ?name ?score ?industry ?revenue ?employees ?tech ?activity ?painpoint
       ?dm_name ?dm_title ?dm_email ?dm_linkedin ?dm_preference ?dm_interests
WHERE {
  ?company a foaf:Organization ;
           foaf:name ?name ;
           si:hasPriorityScore ?score .
  OPTIONAL { ?company schema:industry ?industry }
  OPTIONAL { ?company si:hasRevenueRange ?revenue }
  OPTIONAL { ?company si:hasEmployeeCount ?employees }
  OPTIONAL { ?company si:usesTechnology ?tech }
  OPTIONAL { ?company si:hasRecentActivity ?activity }
  OPTIONAL { ?company si:hasPainPoint ?painpoint }

  OPTIONAL {
    ?person org:memberOf ?company ;
            foaf:name ?dm_name ;
            foaf:title ?dm_title .
    OPTIONAL { ?person foaf:mbox ?dm_email }
    OPTIONAL { ?person foaf:account ?dm_linkedin }
    OPTIONAL { ?person si:hasContactPreference ?dm_preference }
    OPTIONAL { ?person foaf:interest ?dm_interests }
  }
}
ORDER BY DESC(?score)
"
```

### Query Result:
```json
{
  "bindings": [
    {
      "company": {"value": "http://sales.intelligence.org/data/company/ACME_Corporation"},
      "name": {"value": "ACME Corporation"},
      "score": {"value": "85"},
      "industry": {"value": "Software & Technology"},
      "revenue": {"value": "$50-100M"},
      "employees": {"value": "1200"},
      "tech": {"value": "Salesforce Enterprise"},
      "activity": {"value": "Series C funding: $75M"},
      "painpoint": {"value": "Legacy system integration issues"},
      "dm_name": {"value": "Sarah Johnson"},
      "dm_title": {"value": "CEO"},
      "dm_email": {"value": "sarah.johnson@acme.com"},
      "dm_linkedin": {"value": "linkedin.com/in/sarah-johnson-ceo"},
      "dm_preference": {"value": "LinkedIn"},
      "dm_interests": {"value": "AI/ML"}
    }
  ]
}
```

## Step 4: LLM Populates Template

### Input to LLM:
```
You are a sales intelligence analyst. Take this SPARQL query result and populate the Company Intelligence Report template.

SPARQL Results:
[JSON data from above]

Template:
# Company Intelligence Report

## Company 1 Template
**Company Name:** {{COMPANY_1_NAME}}
**Priority Score:** {{COMPANY_1_SCORE}}/100 | **Industry:** {{COMPANY_1_INDUSTRY}}

### Company Intelligence
- **Revenue:** {{COMPANY_1_REVENUE}} | **Employees:** {{COMPANY_1_EMPLOYEES}}
- **Technology Stack:** {{COMPANY_1_TECH_STACK}}
- **Recent Funding/News:** {{COMPANY_1_RECENT_ACTIVITY}}
- **Pain Points/Triggers:** {{COMPANY_1_PAIN_POINTS}}

### Key Decision Maker
**Name:** {{COMPANY_1_DM_NAME}} - {{COMPANY_1_DM_TITLE}}
- **Email:** {{COMPANY_1_DM_EMAIL}}
- **LinkedIn:** {{COMPANY_1_DM_LINKEDIN}}
- **Contact Preference:** {{COMPANY_1_DM_PREFERENCE}}
- **Interests:** {{COMPANY_1_DM_INTERESTS}}

Fill in all {{FIELD}} placeholders with data from the SPARQL results.
```

### LLM Output:
```markdown
# Company Intelligence Report

## Company 1 Template
**Company Name:** ACME Corporation
**Priority Score:** 85/100 | **Industry:** Software & Technology

### Company Intelligence
- **Revenue:** $50-100M | **Employees:** 1,200
- **Technology Stack:** Salesforce Enterprise, AWS, Microsoft Azure
- **Recent Funding/News:** Series C funding: $75M
- **Pain Points/Triggers:** Legacy system integration issues

### Key Decision Maker
**Name:** Sarah Johnson - CEO
- **Email:** sarah.johnson@acme.com
- **LinkedIn:** linkedin.com/in/sarah-johnson-ceo
- **Contact Preference:** LinkedIn
- **Interests:** AI/ML, sustainable technology
```

## Complete Workflow Script

```bash
#!/bin/bash
# example-sales/simple_workflow.sh

echo "=== Simple Sales Intelligence Workflow ==="

# Step 1: LLM extracts RDF triples from source data
echo "Step 1: LLM processes source data..."
# [Call LLM with source data + extraction prompt]
# [LLM returns RDF triples]
# [Save to extracted_data.ttl]

# Step 2: Load RDF into knowledge graph
echo "Step 2: Loading RDF into knowledge graph..."
cargo run -- load --knowledge-graph sales.db --rdf-file extracted_data.ttl

# Step 3: Query knowledge graph
echo "Step 3: Querying knowledge graph..."
QUERY_RESULTS=$(cargo run -- query --knowledge-graph sales.db --format json --query-file company_data_query.sparql)

# Step 4: LLM populates template
echo "Step 4: LLM populates template..."
# [Call LLM with query results + template + population prompt]
# [LLM returns populated template]
# [Save as final report]

echo "Workflow complete! Report generated."
```

## Key Benefits

1. **LLM does the intelligence** - extracts semantics, maps fields, handles variations
2. **Code is simple** - just loads RDF, runs queries, calls LLM
3. **Configuration-driven** - ontology and prompts control behavior
4. **Scalable** - add new data sources by changing prompts
5. **Flexible** - LLM adapts to different template formats

The code becomes a simple pipeline while the LLM handles all the complex data understanding and mapping!