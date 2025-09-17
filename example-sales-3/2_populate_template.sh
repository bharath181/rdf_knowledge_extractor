#!/bin/bash

# ============================================================================
# Script 2: Populate Template Using Knowledge Graph
# ============================================================================
# This script takes the knowledge graph built in Phase 1 and uses it to
# populate a sales template via the LLM, generating a complete report.
# ============================================================================

set -e  # Exit on error

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE} PHASE 2: POPULATE TEMPLATE${NC}"
echo -e "${BLUE}========================================${NC}"

# Configuration
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
CONFIG_FILE="${SCRIPT_DIR}/config.yaml"
TEMPLATE_DIR="${SCRIPT_DIR}/templates"
OUTPUT_DIR="${SCRIPT_DIR}/output"
KG_DB="${OUTPUT_DIR}/knowledge_graph.db"
KG_EXPORT="${OUTPUT_DIR}/knowledge_graph.nt"
TEMPLATE_FILE="${TEMPLATE_DIR}/sales_llm_template.yaml"
REPORT_OUTPUT="${OUTPUT_DIR}/sales_report_$(date +%Y%m%d_%H%M%S).md"
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

# Check if knowledge graph exists
if [ ! -f "$KG_DB" ] && [ ! -f "$KG_EXPORT" ]; then
    echo -e "${RED}Error: Knowledge graph not found${NC}"
    echo "Please run 1_build_knowledge_graph.sh first to build the knowledge graph"
    exit 1
fi

# Check if template exists
if [ ! -f "$TEMPLATE_FILE" ]; then
    echo -e "${RED}Error: Template file not found at $TEMPLATE_FILE${NC}"
    exit 1
fi

# Show knowledge graph info
echo -e "\n${GREEN}Knowledge Graph Information:${NC}"
if [ -f "$KG_EXPORT" ]; then
    TRIPLE_COUNT=$(wc -l < "$KG_EXPORT")
    echo "  Total Triples: $TRIPLE_COUNT"

    # Count unique companies
    COMPANY_COUNT=$(grep "hasName" "$KG_EXPORT" | wc -l)
    echo "  Companies: ~$COMPANY_COUNT"
else
    echo "  Using database: $KG_DB"
fi

# Show template info
echo -e "\n${GREEN}Template Information:${NC}"
echo "  Template: $(basename "$TEMPLATE_FILE")"
echo "  Type: Sales Intelligence Report"

# Extract template metadata
TEMPLATE_NAME=$(grep "^name:" "$TEMPLATE_FILE" | cut -d'"' -f2)
echo "  Name: $TEMPLATE_NAME"

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

# Run template population using the demo command
echo -e "\n${GREEN}Populating Template with Knowledge Graph Data...${NC}"
echo "  This may take 30-60 seconds..."

if [ -n "$VLLM_API_KEY" ]; then
    API_KEY_ARG="--api-key $VLLM_API_KEY"
else
    API_KEY_ARG=""
fi

# Use the demo command with skip-extraction since we already have the knowledge graph
cd "$(dirname "$SCRIPT_DIR")"
$EXTRACTOR demo \
    --skip-extraction \
    --template "$TEMPLATE_FILE" \
    --output "$REPORT_OUTPUT" \
    --server-url "$VLLM_SERVER" \
    --model "$VLLM_MODEL"

if [ $? -eq 0 ]; then
    echo -e "\n${GREEN}✓ Template Populated Successfully${NC}"
else
    echo -e "\n${RED}✗ Failed to populate template${NC}"
    exit 1
fi

# Show report preview
if [ -f "$REPORT_OUTPUT" ]; then
    echo -e "\n${GREEN}Report Preview:${NC}"
    echo "----------------------------------------"
    head -30 "$REPORT_OUTPUT"
    echo "----------------------------------------"
    echo "(Showing first 30 lines)"

    # Count populated fields
    FIELD_COUNT=$(grep -o "\[FIELD:" "$REPORT_OUTPUT" | wc -l)
    if [ $FIELD_COUNT -gt 0 ]; then
        echo -e "\n${YELLOW}Warning: $FIELD_COUNT fields remain unpopulated${NC}"
    fi

    # Show report statistics
    LINE_COUNT=$(wc -l < "$REPORT_OUTPUT")
    WORD_COUNT=$(wc -w < "$REPORT_OUTPUT")
    echo -e "\n${GREEN}Report Statistics:${NC}"
    echo "  Lines: $LINE_COUNT"
    echo "  Words: $WORD_COUNT"
    echo "  Companies: $(grep -c "^### " "$REPORT_OUTPUT" || true)"
fi

echo -e "\n${GREEN}Output Files:${NC}"
echo "  - Generated Report: $REPORT_OUTPUT"

# Create a symlink to the latest report
LATEST_LINK="${OUTPUT_DIR}/latest_report.md"
ln -sf "$(basename "$REPORT_OUTPUT")" "$LATEST_LINK"
echo "  - Latest Report Link: $LATEST_LINK"

echo -e "\n${BLUE}========================================${NC}"
echo -e "${GREEN}✓ Phase 2 Complete!${NC}"
echo -e "Sales report generated successfully!"
echo -e "\nView the report: ${BLUE}cat $REPORT_OUTPUT${NC}"
echo -e "Or open: ${BLUE}$REPORT_OUTPUT${NC}"
echo -e "${BLUE}========================================${NC}"