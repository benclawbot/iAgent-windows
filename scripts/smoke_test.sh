#!/bin/bash
# Smoke test script for jcode binary
# This script performs basic sanity checks to verify the binary works.
# It is designed to run quickly (<30 seconds) as a pre-build check in CI.

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track failures
FAILURES=0

echo "========================================"
echo "jcode Binary Smoke Test"
echo "========================================"
echo ""

# Determine the binary path
# Priority: 1. JCODE_BINARY env var, 2. target/release/jcode, 3. cargo run
if [[ -n "${JCODE_BINARY:-}" ]]; then
    BINARY="$JCODE_BINARY"
    echo "Using binary from JCODE_BINARY env var: $BINARY"
elif [[ -x "target/release/jcode" ]]; then
    BINARY="target/release/jcode"
    echo "Using pre-built release binary: $BINARY"
elif [[ -x "target/debug/jcode" ]]; then
    BINARY="target/debug/jcode"
    echo "Using pre-built debug binary: $BINARY"
else
    echo -e "${YELLOW}No pre-built binary found, building...${NC}"
    # Build the binary (this might take a while, but smoke tests should be fast)
    if command -v cargo &> /dev/null; then
        echo "Running: cargo build --release -p jcode"
        cargo build --release -p jcode 2>&1 | tail -5
        BINARY="target/release/jcode"
    else
        echo -e "${RED}Error: cargo not found and no pre-built binary available${NC}"
        exit 1
    fi
fi

# Verify binary exists and is executable
if [[ ! -x "$BINARY" ]]; then
    echo -e "${RED}Error: Binary not found or not executable: $BINARY${NC}"
    exit 1
fi

echo ""
echo "Testing binary: $BINARY"
echo "Binary size: $(du -h "$BINARY" 2>/dev/null | cut -f1 || echo 'unknown')"
echo ""

# =============================================================================
# Test 1: --version
# =============================================================================
echo "Test 1: jcode --version"
echo "----------------------------------------"
OUTPUT=$("$BINARY" --version 2>&1) || STATUS=$?
if [[ ${STATUS:-0} -eq 0 ]]; then
    echo -e "${GREEN}✓ Exit code: 0${NC}"
    if echo "$OUTPUT" | grep -qi "jcode"; then
        echo -e "${GREEN}✓ Output contains 'jcode'${NC}"
    else
        echo -e "${RED}✗ Output does not contain 'jcode': $OUTPUT${NC}"
        FAILURES=$((FAILURES + 1))
    fi
    echo "Output: $OUTPUT"
else
    echo -e "${RED}✗ Exit code: ${STATUS:-unknown}${NC}"
    echo "Output: $OUTPUT"
    FAILURES=$((FAILURES + 1))
fi
echo ""

# =============================================================================
# Test 2: --help
# =============================================================================
echo "Test 2: jcode --help | head -20"
echo "----------------------------------------"
OUTPUT=$("$BINARY" --help 2>&1) || STATUS=$?
if [[ ${STATUS:-0} -eq 0 ]]; then
    echo -e "${GREEN}✓ Exit code: 0${NC}"
    HELP_LINES=$(echo "$OUTPUT" | head -20)
    LINE_COUNT=$(echo "$HELP_LINES" | wc -l)
    echo -e "${GREEN}✓ Help output has $LINE_COUNT lines (first 20 shown)${NC}"
    echo "--- First 20 lines ---"
    echo "$HELP_LINES"
    echo "---"
    if echo "$OUTPUT" | grep -qiE "(usage|options|commands|subcommand)"; then
        echo -e "${GREEN}✓ Help output contains usage/options/commands${NC}"
    else
        echo -e "${YELLOW}⚠ Help output may not contain expected keywords${NC}"
    fi
else
    echo -e "${RED}✗ Exit code: ${STATUS:-unknown}${NC}"
    echo "Output: $OUTPUT"
    FAILURES=$((FAILURES + 1))
fi
echo ""

# =============================================================================
# Test 3: --version with timeout (quick test)
# =============================================================================
echo "Test 3: jcode --version (with 5s timeout)"
echo "----------------------------------------"
if command -v timeout &> /dev/null; then
    if timeout 5 "$BINARY" --version > /dev/null 2>&1; then
        echo -e "${GREEN}✓ Binary responds within 5 seconds${NC}"
    else
        echo -e "${RED}✗ Binary did not respond within 5 seconds${NC}"
        FAILURES=$((FAILURES + 1))
    fi
else
    echo -e "${YELLOW}⚠ timeout command not available, skipping${NC}"
fi
echo ""

# =============================================================================
# Summary
# =============================================================================
echo "========================================"
echo "Smoke Test Summary"
echo "========================================"
if [[ $FAILURES -eq 0 ]]; then
    echo -e "${GREEN}All tests passed! ✓${NC}"
    exit 0
else
    echo -e "${RED}Failed tests: $FAILURES${NC}"
    exit 1
fi