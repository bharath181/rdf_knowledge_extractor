#!/bin/bash

# ============================================================================
# Script 1: Build Knowledge Graph from Source Data
# ============================================================================
# This script takes source documents and extracts RDF triples to build
# a knowledge graph using the LLM.
# ============================================================================

set -e  # Exit on error

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE} PHASE 1: BUILD KNOWLEDGE GRAPH${NC}"
echo -e "${BLUE}========================================${NC}"

# Configuration
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
CONFIG_FILE="${SCRIPT_DIR}/config.yaml"
SOURCE_DIR="${SCRIPT_DIR}/source-data"
OUTPUT_DIR="${SCRIPT_DIR}/output"
KG_DB="${OUTPUT_DIR}/knowledge_graph.db"
KG_EXPORT="${OUTPUT_DIR}/knowledge_graph.nt"
EXTRACTOR="$(dirname "$SCRIPT_DIR")/target/release/rdf_knowledge_extractor"

# Default vLLM server settings (can be overridden by environment variables)
VLLM_SERVER="${VLLM_SERVER:-http://localhost:8000}"
VLLM_MODEL="${VLLM_MODEL:-Qwen/Qwen2.5-32B-Instruct}"
VLLM_API_KEY="${VLLM_API_KEY:-}"

# Check if extractor exists
if [ ! -f "$EXTRACTOR" ]; then
    echo -e "${RED}Error: RDF Knowledge Extractor not found at $EXTRACTOR${NC}"
    echo "Please build the project first: cargo build --release"
    exit 1
fi

# Check if config exists
if [ ! -f "$CONFIG_FILE" ]; then
    echo -e "${RED}Error: Configuration file not found at $CONFIG_FILE${NC}"
    exit 1
fi

# Check if source files exist
if [ ! -d "$SOURCE_DIR" ] || [ -z "$(ls -A $SOURCE_DIR)" ]; then
    echo -e "${RED}Error: No source files found in $SOURCE_DIR${NC}"
    exit 1
fi

# Create output directory
mkdir -p "$OUTPUT_DIR"

# List source files
echo -e "\n${GREEN}Source Documents:${NC}"
for file in "$SOURCE_DIR"/*; do
    if [ -f "$file" ]; then
        echo "  - $(basename "$file")"
    fi
done

# Check vLLM server health
echo -e "\n${GREEN}Checking vLLM Server...${NC}"
echo "  Server URL: $VLLM_SERVER"
echo "  Model: $VLLM_MODEL"

$EXTRACTOR check-server --server-url "$VLLM_SERVER" > /dev/null 2>&1
if [ $? -eq 0 ]; then
    echo -e "  Status: ${GREEN}✓ Healthy${NC}"
else
    echo -e "  Status: ${RED}✗ Not responding${NC}"
    echo -e "\n${RED}Error: vLLM server is not responding at $VLLM_SERVER${NC}"
    echo "Please ensure your vLLM server is running."
    exit 1
fi

# Extract knowledge from documents
echo -e "\n${GREEN}Extracting Knowledge from Documents...${NC}"

# Build input file list
INPUT_FILES=""
for file in "$SOURCE_DIR"/*; do
    if [ -f "$file" ]; then
        INPUT_FILES="$INPUT_FILES -i $file"
    fi
done

# Run extraction
if [ -n "$VLLM_API_KEY" ]; then
    API_KEY_ARG="--api-key $VLLM_API_KEY"
else
    API_KEY_ARG=""
fi

$EXTRACTOR extract \
    -c "$CONFIG_FILE" \
    $INPUT_FILES \
    --kg-path "$KG_DB" \
    -o "$KG_EXPORT" \
    --format n-triples \
    --server-url "$VLLM_SERVER" \
    --model "$VLLM_MODEL" \
    $API_KEY_ARG \
    --validate \
    --merge

if [ $? -eq 0 ]; then
    echo -e "\n${GREEN}✓ Knowledge Graph Built Successfully${NC}"
else
    echo -e "\n${RED}✗ Failed to build knowledge graph${NC}"
    exit 1
fi

# Show statistics
echo -e "\n${GREEN}Knowledge Graph Statistics:${NC}"
if [ -f "$KG_EXPORT" ]; then
    TRIPLE_COUNT=$(wc -l < "$KG_EXPORT")
    echo "  Total Triples: $TRIPLE_COUNT"

    # Show sample triples
    echo -e "\n${GREEN}Sample Triples:${NC}"
    head -5 "$KG_EXPORT" | while IFS= read -r line; do
        # Truncate long lines for readability
        if [ ${#line} -gt 100 ]; then
            echo "  ${line:0:100}..."
        else
            echo "  $line"
        fi
    done
fi

echo -e "\n${GREEN}Output Files:${NC}"
echo "  - Knowledge Graph DB: $KG_DB"
echo "  - N-Triples Export: $KG_EXPORT"

echo -e "\n${BLUE}========================================${NC}"
echo -e "${GREEN}✓ Phase 1 Complete!${NC}"
echo -e "Knowledge graph built from $(ls -1 "$SOURCE_DIR" | wc -l) source documents"
echo -e "\nNext: Run ${BLUE}2_populate_template.sh${NC} to generate the sales report"
echo -e "${BLUE}========================================${NC}"