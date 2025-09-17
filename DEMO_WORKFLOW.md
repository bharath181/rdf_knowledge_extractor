# Complete Workflow Demo: Source Data → Knowledge Graph → LLM-Populated Template

This demo shows the complete pipeline of your RDF Knowledge Extractor system.

## Overview

The system performs three main steps:

1. **Extract**: Process source documents (PDFs, text files, URLs) using an LLM to extract structured RDF triples
2. **Store**: Save triples in a knowledge graph (simplified SPARQL implementation)
3. **Populate**: Send template + knowledge graph data to LLM to generate populated reports

## Quick Start

### Prerequisites

1. Start your vLLM server (e.g., with Qwen2.5-32B):
```bash
docker run --rm -it \
  --device=/dev/kfd --device=/dev/dri \
  --group-add video --group-add render \
  --ipc=host --cap-add=SYS_PTRACE --security-opt seccomp=unconfined \
  -p 8000:8000 \
  rocm/vllm:latest \
  vllm serve Qwen/Qwen2.5-32B-Instruct \
  --host 0.0.0.0 \
  --port 8000 \
  --tensor-parallel-size 1
```

### Run the Complete Demo

Using existing knowledge graph data:
```bash
./target/release/rdf_knowledge_extractor demo --skip-extraction
```

Or extract fresh data from source documents:
```bash
./target/release/rdf_knowledge_extractor demo
```

## What Happens

### Phase 1: Extraction (if not skipping)
- Reads source documents from `example-sales-2/source-data/`
- Sends documents to LLM with extraction questions from config
- LLM returns structured RDF triples
- Triples are stored in knowledge graph

### Phase 2: Template Population
- Loads template from `templates/sales_llm_template.yaml`
- Executes SPARQL queries against knowledge graph
- Sends template + extracted data to LLM
- LLM populates all `[FIELD: ...]` placeholders with actual data
- Saves completed report to `populated_sales_report.md`

## Template Structure

The template contains placeholders that the LLM will fill:

```markdown
### Company Name: [FIELD: Company Name]
Priority Score: [FIELD: Score]/100 | Industry: [FIELD: Industry Type]

#### Company Intelligence
- Revenue: $[FIELD: Revenue Range]
- Pain Points/Triggers: [FIELD: Pain Points]

#### Key Attendees & Profiles
**Decision Maker:**
- Name: [FIELD: Decision Maker Name]
- Title: [FIELD: Title]
```

## Knowledge Graph Data

The system extracts triples like:
```
<TechCorpIndustriesInc> <hasName> "TechCorp Industries Inc."
<TechCorpIndustriesInc> <hasRevenue> "$500M-1B annually"
<TechCorpIndustriesInc> <hasDecisionMaker> "Jennifer Walsh"
<TechCorpIndustriesInc> <hasPainPoint> "Legacy ERP integration challenges"
```

## LLM Population Process

The LLM receives:
1. The template with placeholders
2. All relevant triples from the knowledge graph
3. Instructions on how to populate fields

The LLM returns:
- Fully populated report with real company names, people, revenue figures, etc.
- Professional formatting and structure
- "Not Available" for missing data

## Example Output

After running the demo, you'll get a populated sales report like:

```markdown
### TechCorp Industries Inc.
Priority Score: 95/100 | Industry: Enterprise Software

#### Company Intelligence
- Revenue: $500M-1B annually
- Pain Points/Triggers: Legacy ERP integration challenges, data silos across departments

#### Key Attendees & Profiles
**Decision Maker:**
- Name: Jennifer Walsh
- Title: Chief Technology Officer
- Contact Preference: LinkedIn
...
```

## Customization

- Edit `templates/sales_llm_template.yaml` to change the template
- Modify `example-sales-2/sales_intelligence_config.yaml` to change extraction rules
- Add new source documents to `example-sales-2/source-data/`

## Architecture

```
Source Documents → LLM Extraction → RDF Triples → Knowledge Graph
                                                         ↓
                                                   SPARQL Queries
                                                         ↓
Template + Data → LLM Population → Completed Report
```

This demonstrates the full power of combining:
- LLM-based information extraction
- Knowledge graph storage
- Template-based report generation
- LLM-powered template population