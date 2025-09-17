# RDF Knowledge Extractor

A high-performance Rust application that extracts structured RDF triples from documents using Large Language Models (LLM). Designed to work with vLLM servers running models like Qwen2.5-32B-Instruct.

## Features

- **Multi-format Document Processing**: PDF, text files, URLs, and HTML content
- **vLLM Integration**: OpenAI-compatible API support for local LLM servers
- **RDF Output**: Multiple serialization formats (Turtle, JSON-LD, N-Triples, RDF/XML)
- **Configurable Extraction**: YAML/JSON configuration for questions and schemas
- **High Performance**: Async Rust implementation with concurrent processing
- **Validation**: Built-in RDF triple validation and schema checking
- **LLM-Powered Template Population**: Templates populated via LLM intelligence
- **Knowledge Graph Storage**: Persistent RDF triple storage and export

## Current Limitations

- **SPARQL Queries**: Simplified implementation - LLM does most query processing
- **Scalability**: Current approach efficient for small knowledge graphs only
- **Query Complexity**: Advanced SPARQL features not yet supported

## Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Documents     │    │   vLLM Server    │    │   RDF Output    │
│  PDF/Text/URLs  │───▶│ Qwen2.5-32B-Inst │───▶│ Turtle/JSON-LD  │
└─────────────────┘    └──────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│ Document Handler│    │  LLM Client      │    │ RDF Serializer  │
│ - PDF Extract   │    │  - HTTP Client   │    │ - Oxigraph      │
│ - Text Parser   │    │  - JSON Parsing  │    │ - Validation    │
│ - Web Scraper   │    │  - Prompt Build  │    │ - Multi-format  │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

## Installation

### Prerequisites

1. **Rust** (latest stable version)
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

2. **vLLM Server** running with your chosen model:
```bash
# Example: Running Qwen2.5-32B-Instruct with vLLM
docker run --gpus all -v ~/.cache/huggingface:/root/.cache/huggingface \
  --env "HUGGING_FACE_HUB_TOKEN=<your_token>" \
  -p 8000:8000 \
  --ipc=host \
  vllm/vllm-openai:latest \
  --model Qwen/Qwen2.5-32B-Instruct \
  --served-model-name Qwen2.5-32B-Instruct
```

### Build from Source

```bash
git clone <repository>
cd rdf_knowledge_extractor
cargo build --release
```

## Quick Start with vLLM Container

### Step 1: Start vLLM Server
```bash
# Start vLLM container with Qwen2.5-32B-Instruct (ROCm/AMD GPU)
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

# Alternative: NVIDIA GPU setup
docker run --rm -it \
  --gpus all \
  -v ~/.cache/huggingface:/root/.cache/huggingface \
  -p 8000:8000 \
  --ipc=host \
  vllm/vllm-openai:latest \
  --model Qwen/Qwen2.5-32B-Instruct \
  --host 0.0.0.0 \
  --port 8000
```

### Step 2: Build and Run the Rust System
```bash
# Clone and build
git clone <this-repository>
cd rdf_knowledge_extractor
cargo build --release

# Generate example configuration
cargo run -- generate-config config.yaml

# Check server connection
cargo run -- check-server --server-url http://localhost:8000

# Extract knowledge from documents
cargo run -- extract -c config.yaml -i document.pdf -k knowledge_graph.db

# Generate example templates
cargo run -- generate-templates templates/

# Generate documents from knowledge
cargo run -- generate -c config.yaml -k knowledge_graph.db -t templates/company_report.yaml
```

### Step 3: Complete Workflow Example
```bash
# 1. Test server health
cargo run -- check-server

# 2. Extract knowledge from multiple documents
cargo run -- extract \
  -c config.yaml \
  -i report.pdf document.txt https://example.com/article \
  -k knowledge_graph.db \
  --validate

# 3. View knowledge graph statistics
cargo run -- stats -k knowledge_graph.db -c config.yaml

# 4. Query the knowledge graph
cargo run -- query \
  -k knowledge_graph.db \
  --query "SELECT ?name WHERE { ?person hasName ?name }" \
  --format table

# 5. Generate professional documents
cargo run -- generate \
  -c config.yaml \
  -k knowledge_graph.db \
  -t templates/executive_summary.yaml \
  -o generated_summary.md

# 6. Export knowledge in different formats
cargo run -- export \
  -k knowledge_graph.db \
  -c config.yaml \
  -o knowledge_export.ttl \
  --format turtle
```

## Configuration

The system uses YAML or JSON configuration files to define:

### Extraction Questions
Define what information to extract:
```yaml
extraction_questions:
  - id: "company_names"
    question: "What companies are mentioned in the document?"
    description: "Extract all company names and organizations"
    expected_type: "string"
    constraints:
      - "Must be proper noun"
      - "Include full legal name"
```

### RDF Schema
Define your ontology and predicates:
```yaml
rdf_schema:
  namespace: "http://example.org/ontology#"
  prefix: "ex"
  base_uri: "http://example.org/resource/"
  predicates:
    hasName: "Entity has name"
    worksFor: "Person works for organization"
  classes:
    Person: "A human being"
    Organization: "A company or institution"
```

### LLM Settings
Configure your vLLM server connection:
```yaml
llm_settings:
  base_url: "http://localhost:8000"
  model: "Qwen/Qwen2.5-32B-Instruct"
  temperature: 0.3
  max_tokens: 4096
  timeout: 120
```

## Usage Examples

### Basic Extraction
```bash
# Extract from single document
rdf_knowledge_extractor extract -c config.yaml -i document.pdf -o output.ttl

# Extract from multiple documents
rdf_knowledge_extractor extract -c config.yaml -i doc1.pdf doc2.txt -o output.ttl

# Extract from web URL
rdf_knowledge_extractor extract -c config.yaml -i https://example.com/article -o output.ttl
```

### Advanced Options
```bash
# Merge results from multiple documents
rdf_knowledge_extractor extract -c config.yaml -i doc1.pdf doc2.pdf --merge -o merged.ttl

# Different output formats
rdf_knowledge_extractor extract -c config.yaml -i document.pdf --format json-ld -o output.jsonld

# Validate extracted triples
rdf_knowledge_extractor extract -c config.yaml -i document.pdf --validate -o output.ttl

# Override LLM settings
rdf_knowledge_extractor extract -c config.yaml -i document.pdf \
  --server-url http://different-server:8000 \
  --model "different-model" \
  -o output.ttl
```

### Validation and Testing
```bash
# Validate configuration file
rdf_knowledge_extractor validate -c config.yaml

# Check server status and available models
rdf_knowledge_extractor check-server --server-url http://localhost:8000
```

## Output Formats

The system supports multiple RDF serialization formats:

- **Turtle** (`.ttl`) - Human-readable, compact
- **JSON-LD** (`.jsonld`) - JSON-based, web-friendly
- **N-Triples** (`.nt`) - Simple, line-based
- **RDF/XML** (`.rdf`) - XML-based standard
- **JSON** (`.json`) - Raw triple objects

## Performance Tips

1. **Batch Processing**: Process multiple documents in one command for better efficiency
2. **Temperature Settings**: Use lower temperatures (0.1-0.3) for more consistent extraction
3. **Token Limits**: Adjust `max_tokens` based on document complexity
4. **Concurrent Processing**: The system automatically processes documents concurrently

## Integration Examples

### Shell Script Integration
```bash
#!/bin/bash
CONFIG="config.yaml"
INPUT_DIR="documents/"
OUTPUT_DIR="rdf_output/"

for file in "$INPUT_DIR"*.pdf; do
  filename=$(basename "$file" .pdf)
  rdf_knowledge_extractor extract \
    -c "$CONFIG" \
    -i "$file" \
    -o "$OUTPUT_DIR/$filename.ttl" \
    --validate
done
```

### Python Integration
```python
import subprocess
import json

def extract_rdf(config_path, input_file, output_format="json"):
    result = subprocess.run([
        "rdf_knowledge_extractor", "extract",
        "-c", config_path,
        "-i", input_file,
        "--format", output_format
    ], capture_output=True, text=True)

    if output_format == "json":
        return json.loads(result.stdout)
    return result.stdout
```

## Troubleshooting

### Common Issues

1. **vLLM Server Not Responding**:
   - Check server status: `rdf_knowledge_extractor check-server`
   - Verify server URL and port
   - Ensure model is loaded

2. **Document Processing Errors**:
   - Check file permissions and accessibility
   - For PDFs: ensure they contain extractable text
   - For URLs: verify they're accessible and not behind authentication

3. **Invalid RDF Output**:
   - Use `--validate` flag to check triples
   - Review configuration schema and predicates
   - Check extraction questions for clarity

4. **Performance Issues**:
   - Reduce `max_tokens` for faster processing
   - Increase server timeout for large documents
   - Process documents in smaller batches

### Debugging

Enable verbose logging:
```bash
rdf_knowledge_extractor extract -c config.yaml -i document.pdf --verbose
```

Enable debug logging:
```bash
rdf_knowledge_extractor extract -c config.yaml -i document.pdf --debug
```

## Development

### Project Structure
```
src/
├── config/         # Configuration loading and validation
├── core/           # Core extraction engine and LLM client
├── handlers/       # Document processing handlers
├── utils/          # RDF serialization and utilities
└── main.rs         # CLI interface
```

### Running Tests
```bash
cargo test
```

### Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass
5. Submit a pull request

## License

MIT License - see LICENSE file for details.

## Acknowledgments

- Built with [Rust](https://www.rust-lang.org/)
- RDF processing with [Oxigraph](https://github.com/oxigraph/oxigraph)
- LLM integration designed for [vLLM](https://github.com/vllm-project/vllm)
- CLI framework: [Clap](https://github.com/clap-rs/clap)