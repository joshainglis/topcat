#!/usr/bin/env bash
#
# Clippy check script for Topcat
#
# This script runs clippy with appropriate flags and provides
# helpful output for fixing issues.
#
# Usage:
#   ./scripts/clippy-check.sh [OPTIONS]
#
# Options:
#   --fix          Run clippy with auto-fix
#   --strict       Treat all warnings as errors
#   --test         Run tests after clippy
#   --verbose      Show verbose output

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Parse arguments
FIX=false
STRICT=false
RUN_TESTS=false
VERBOSE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --fix)
            FIX=true
            shift
            ;;
        --strict)
            STRICT=true
            shift
            ;;
        --test)
            RUN_TESTS=true
            shift
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --fix       Run clippy with auto-fix"
            echo "  --strict    Treat all warnings as errors"
            echo "  --test      Run tests after clippy"
            echo "  --verbose   Show verbose output"
            echo "  --help      Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Run '$0 --help' for usage information"
            exit 1
            ;;
    esac
done

# Print header
echo -e "${BLUE}================================================${NC}"
echo -e "${BLUE}  Topcat Clippy Check${NC}"
echo -e "${BLUE}================================================${NC}"
echo ""

# Build clippy command
CLIPPY_CMD="cargo clippy --all-targets"

if [ "$FIX" = true ]; then
    echo -e "${YELLOW}Running clippy with auto-fix...${NC}"
    CLIPPY_CMD="$CLIPPY_CMD --fix --allow-dirty --allow-staged"
else
    echo -e "${YELLOW}Running clippy check...${NC}"
fi

# Add strict flags if requested
if [ "$STRICT" = true ]; then
    echo -e "${YELLOW}Strict mode: treating warnings as errors${NC}"
    CLIPPY_CMD="$CLIPPY_CMD -- -D warnings"
fi

echo ""

# Run clippy
if [ "$VERBOSE" = true ]; then
    echo -e "${BLUE}Command:${NC} $CLIPPY_CMD"
    echo ""
fi

if $CLIPPY_CMD; then
    echo ""
    echo -e "${GREEN}✓ Clippy check passed!${NC}"
    CLIPPY_SUCCESS=true
else
    echo ""
    echo -e "${RED}✗ Clippy found issues${NC}"
    CLIPPY_SUCCESS=false
fi

# Run tests if requested and clippy passed
if [ "$RUN_TESTS" = true ]; then
    echo ""
    echo -e "${BLUE}------------------------------------------------${NC}"
    echo -e "${YELLOW}Running tests...${NC}"
    echo ""

    if cargo test --lib --tests; then
        echo ""
        echo -e "${GREEN}✓ Tests passed!${NC}"
        TESTS_SUCCESS=true
    else
        echo ""
        echo -e "${RED}✗ Tests failed${NC}"
        TESTS_SUCCESS=false
    fi
fi

# Summary
echo ""
echo -e "${BLUE}================================================${NC}"
echo -e "${BLUE}  Summary${NC}"
echo -e "${BLUE}================================================${NC}"

if [ "$CLIPPY_SUCCESS" = true ]; then
    echo -e "${GREEN}Clippy: PASSED${NC}"
else
    echo -e "${RED}Clippy: FAILED${NC}"
fi

if [ "$RUN_TESTS" = true ]; then
    if [ "$TESTS_SUCCESS" = true ]; then
        echo -e "${GREEN}Tests:  PASSED${NC}"
    else
        echo -e "${RED}Tests:  FAILED${NC}"
    fi
fi

echo ""

# Exit with appropriate code
if [ "$CLIPPY_SUCCESS" = true ] && ([ "$RUN_TESTS" = false ] || [ "$TESTS_SUCCESS" = true ]); then
    echo -e "${GREEN}All checks passed! ✓${NC}"
    exit 0
else
    echo -e "${RED}Some checks failed! ✗${NC}"
    echo ""
    echo "Next steps:"
    if [ "$CLIPPY_SUCCESS" = false ]; then
        echo "  1. Review clippy warnings above"
        echo "  2. Fix issues manually or run: $0 --fix"
        echo "  3. Re-run: $0 --strict"
    fi
    if [ "$RUN_TESTS" = true ] && [ "$TESTS_SUCCESS" = false ]; then
        echo "  4. Review test failures"
        echo "  5. Run: cargo test -- --nocapture (for detailed output)"
    fi
    exit 1
fi
