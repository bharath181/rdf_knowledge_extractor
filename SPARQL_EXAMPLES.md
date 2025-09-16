# SPARQL Query Examples for Company Intelligence

This document contains SPARQL queries that demonstrate how to extract specific information from the RDF knowledge graph to populate the Company Intelligence Report template.

## Prerequisites

The queries use the following ontology prefixes:
```sparql
PREFIX foaf: <http://xmlns.com/foaf/0.1/>
PREFIX sales: <http://sales.intelligence.org/ontology#>
PREFIX schema: <https://schema.org/>
PREFIX org: <http://www.w3.org/ns/org#>
PREFIX vcard: <http://www.w3.org/2006/vcard/ns#>
PREFIX cco: <https://www.commoncoreontologies.org/>
PREFIX dcterms: <http://purl.org/dc/terms/>
```

## Template Field Queries

### 1. Company Name and Priority Score
```sparql
# Get company name and priority score
SELECT ?company ?name ?score ?industry
WHERE {
  ?company a foaf:Organization ;
           foaf:name ?name ;
           sales:hasPriorityScore ?score ;
           schema:industry ?industry .
}
ORDER BY DESC(?score)
LIMIT 5
```

### 2. Company Intelligence (Revenue, Employees, Tech Stack)
```sparql
# Get company financial and operational data
SELECT ?company ?name ?revenue ?employees ?tech
WHERE {
  ?company a foaf:Organization ;
           foaf:name ?name .
  OPTIONAL { ?company schema:revenue ?revenue }
  OPTIONAL { ?company schema:numberOfEmployees ?employees }
  OPTIONAL { ?company sales:usesTechnology ?tech }
}
```

### 3. Recent Activity and News
```sparql
# Get recent funding, acquisitions, and news
SELECT ?company ?name ?activity ?date
WHERE {
  ?company a foaf:Organization ;
           foaf:name ?name ;
           sales:hasRecentActivity ?activity .
  OPTIONAL { ?activity dcterms:created ?date }
}
ORDER BY DESC(?date)
```

### 4. Pain Points and Business Challenges
```sparql
# Get business pain points and triggers
SELECT ?company ?name ?painpoint ?severity
WHERE {
  ?company a foaf:Organization ;
           foaf:name ?name ;
           sales:hasPainPoint ?painpoint .
  OPTIONAL { ?painpoint cco:hasQuality ?severity }
}
```

### 5. Decision Makers with Full Details
```sparql
# Get all decision maker information
SELECT ?person ?name ?title ?email ?phone ?linkedin
       ?preference ?interests ?recent_activity
WHERE {
  ?person a sales:DecisionMaker ;
          foaf:name ?name ;
          foaf:title ?title ;
          org:memberOf ?company .

  # Contact information
  OPTIONAL { ?person foaf:mbox ?email }
  OPTIONAL { ?person foaf:phone ?phone }
  OPTIONAL {
    ?person foaf:account ?account .
    FILTER(CONTAINS(STR(?account), "linkedin"))
    BIND(?account AS ?linkedin)
  }

  # Preferences and interests
  OPTIONAL { ?person sales:hasContactPreference ?preference }
  OPTIONAL { ?person foaf:interest ?interests }
  OPTIONAL { ?person sales:hasRecentPost ?recent_activity }
}
```

### 6. Influencers and Their Impact
```sparql
# Get influencer details with decision influence
SELECT ?person ?name ?title ?department
       ?influence_score ?interactions
WHERE {
  ?person a sales:Influencer ;
          foaf:name ?name ;
          foaf:title ?title ;
          org:memberOf ?company .

  OPTIONAL { ?person org:hasMember ?department }
  OPTIONAL { ?person sales:hasInfluenceScore ?influence_score }
  OPTIONAL { ?person sales:hasContactNotes ?interactions }
}
ORDER BY DESC(?influence_score)
```

### 7. Contact History Summary
```sparql
# Get aggregated contact history
SELECT ?company ?name
       (COUNT(?contact) AS ?touchpoints)
       (MAX(?date) AS ?last_contact)
       ?response_rate ?temperature
WHERE {
  ?company a foaf:Organization ;
           foaf:name ?name .

  OPTIONAL {
    ?contact sales:hasContactEvent ?company ;
             dcterms:created ?date .
  }

  OPTIONAL { ?company sales:hasResponseRate ?response_rate }
  OPTIONAL { ?company sales:hasRelationshipTemperature ?temperature }
}
GROUP BY ?company ?name ?response_rate ?temperature
```

### 8. Contact Methods Used
```sparql
# Get contact methods breakdown
SELECT ?company ?name ?method (COUNT(?event) AS ?count)
WHERE {
  ?company a foaf:Organization ;
           foaf:name ?name .
  ?event sales:hasContactEvent ?company ;
         sales:hasContactMethod ?method .
}
GROUP BY ?company ?name ?method
```

### 9. Competitive Intelligence
```sparql
# Get competitive landscape data
SELECT ?company ?name ?competitor ?solution
       ?renewal_date ?contract_value
WHERE {
  ?company a foaf:Organization ;
           foaf:name ?name .

  OPTIONAL {
    ?company schema:competitor ?competitor .
    ?competitor foaf:name ?solution .
  }

  OPTIONAL { ?company sales:hasContractRenewal ?renewal_date }
  OPTIONAL { ?contract schema:price ?contract_value }
}
```

### 10. Competitive Advantages
```sparql
# Get our competitive advantages for each company
SELECT ?company ?name ?advantage ?category
WHERE {
  ?company a foaf:Organization ;
           foaf:name ?name ;
           sales:hasCompetitiveAdvantage ?advantage .

  OPTIONAL { ?advantage cco:hasQuality ?category }
}
```

## Complex Aggregation Queries

### Complete Company Profile
```sparql
# Get complete company profile with all related entities
CONSTRUCT {
  ?company ?p ?o .
  ?person ?pp ?po .
  ?contact ?cp ?co .
}
WHERE {
  {
    ?company a foaf:Organization ;
             foaf:name "ACME Corporation" ;
             ?p ?o .
  }
  UNION
  {
    ?person org:memberOf ?company ;
            ?pp ?po .
    ?company foaf:name "ACME Corporation" .
  }
  UNION
  {
    ?contact sales:hasContactEvent ?company ;
             ?cp ?co .
    ?company foaf:name "ACME Corporation" .
  }
}
```

### Relationship Network
```sparql
# Get the relationship network around a company
SELECT ?subject ?predicate ?object
WHERE {
  {
    # Direct relationships
    ?company foaf:name "ACME Corporation" .
    ?company ?predicate ?object .
    BIND(?company AS ?subject)
  }
  UNION
  {
    # People relationships
    ?person org:memberOf ?company .
    ?company foaf:name "ACME Corporation" .
    ?person ?predicate ?object .
    BIND(?person AS ?subject)
  }
}
```

### Priority Scoring Factors
```sparql
# Get all factors contributing to priority score
SELECT ?company ?name ?score ?revenue ?employees
       (COUNT(?painpoint) AS ?pain_count)
       ?temperature ?renewal
WHERE {
  ?company a foaf:Organization ;
           foaf:name ?name ;
           sales:hasPriorityScore ?score .

  OPTIONAL { ?company schema:revenue ?revenue }
  OPTIONAL { ?company schema:numberOfEmployees ?employees }
  OPTIONAL { ?company sales:hasPainPoint ?painpoint }
  OPTIONAL { ?company sales:hasRelationshipTemperature ?temperature }
  OPTIONAL { ?company sales:hasContractRenewal ?renewal }
}
GROUP BY ?company ?name ?score ?revenue ?employees ?temperature ?renewal
ORDER BY DESC(?score)
```

## Template Population Example

To populate the Company Intelligence Report template, execute these queries in sequence:

1. **Get top 5 companies** (Query 1)
2. **For each company, get:**
   - Company intelligence data (Query 2)
   - Recent activities (Query 3)
   - Pain points (Query 4)
   - Decision makers (Query 5)
   - Influencers (Query 6)
   - Contact history (Query 7)
   - Competitive intelligence (Query 9)
   - Competitive advantages (Query 10)

The results are then merged and formatted according to the template structure.

## Usage with the RDF Knowledge Extractor

```bash
# Execute a query from command line
cargo run -- query \
  -k company_intelligence.db \
  --query "SELECT ?company ?name WHERE { ?company a foaf:Organization ; foaf:name ?name }" \
  --format table

# Execute a query from file
cargo run -- query \
  -k company_intelligence.db \
  --query-file queries/get_decision_makers.sparql \
  --format json
```