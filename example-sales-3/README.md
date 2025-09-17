# Example Sales Intelligence Pipeline

This directory demonstrates the complete two-phase workflow of the RDF Knowledge Extractor system.

## Quick Start

### Prerequisites
1. Start your vLLM server:
```bash
docker run --rm -it \
  --device=/dev/kfd --device=/dev/dri \
  --group-add video --group-add render \
  --ipc=host --cap-add=SYS_PTRACE --security-opt seccomp=unconfined \
  -p 8000:8000 \
  rocm/vllm:latest \
  vllm serve Qwen/Qwen2.5-32B-Instruct \
  --host 0.0.0.0 \
  --port 8000
```

2. Build the project:
```bash
cd .. # Go to project root
cargo build --release
```

### Run the Pipeline

#### Phase 1: Extract knowledge from source documents
```bash
./1_build_knowledge_graph.sh
```

#### Phase 2: Populate template with extracted knowledge
```bash
./2_populate_template.sh
```

## Directory Structure

```
example-sales-3/
├── 1_build_knowledge_graph.sh    # Script for Phase 1
├── 2_populate_template.sh        # Script for Phase 2
├── CURRENT_STATUS.md              # System status and gaps
├── README.md                      # This file
├── config.yaml                    # Extraction configuration
├── source-data/                   # Input documents
│   ├── crm_company_export.txt
│   └── linkedin_sales_navigator_export.txt
├── templates/                     # Report templates
│   └── sales_llm_template.yaml
└── output/                        # Generated files
    ├── knowledge_graph.db         # Knowledge graph storage
    ├── knowledge_graph.nt         # N-Triples export
    ├── sales_report_*.md          # Generated reports
    └── latest_report.md           # Symlink to latest report
```

## Environment Variables

You can customize the vLLM server settings:

```bash
export VLLM_SERVER="http://localhost:8000"
export VLLM_MODEL="Qwen/Qwen2.5-32B-Instruct"
export VLLM_API_KEY="your-api-key"  # Optional
```

## What Actually Happens

### Phase 1: Knowledge Extraction (WORKING)
1. Reads source documents from `source-data/`
2. Sends documents + extraction questions to LLM
3. LLM returns structured RDF triples
4. Saves triples to knowledge graph database
5. Exports to N-Triples format for inspection

### Phase 2: Template Population (BROKEN SPARQL)
1. Loads the knowledge graph from Phase 1
2. **IGNORES SPARQL queries** - dumps ALL triples to LLM instead
3. Sends template + ALL raw triples to LLM
4. **LLM does all the work**: parsing, filtering, grouping, populating
5. Saves the completed sales report

**Note**: SPARQL queries in templates are currently decorative. The LLM manually processes all data rather than using a proper query engine.

## Expected Output

After running both phases, you'll have:
- A knowledge graph with company data, decision makers, pain points
- A professional sales report with all fields populated from actual data
- Export files for further processing

## Troubleshooting

### vLLM Server Issues
```bash
# Check if server is running
curl http://localhost:8000/health

# Check server logs
docker logs <container-id>
```

### Build Issues
```bash
# Rebuild if needed
cd .. && cargo build --release
```

### Permission Issues
```bash
# Make scripts executable
chmod +x *.sh
```

## Customization

- **Add new source documents**: Place files in `source-data/`
- **Modify extraction rules**: Edit `config.yaml`
- **Change template**: Edit `templates/sales_llm_template.yaml`
- **Adjust LLM settings**: Set environment variables

## Next Steps

See `CURRENT_STATUS.md` for:
- Detailed system architecture
- Known limitations and gaps
- Development priorities
- Performance optimization opportunities