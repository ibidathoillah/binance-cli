#!/usr/bin/env bash
# =============================================================================
# Binance CLI - End-to-End Test Suite
# =============================================================================
# Public endpoints require no credentials.
# Private endpoints require BINANCE_API_KEY / BINANCE_API_SECRET or config.
#
# Usage:
#   ./scripts/e2e_test.sh              # Run public + private (private skipped without creds)
#   ./scripts/e2e_test.sh --public     # Run public tests only
#   ./scripts/e2e_test.sh --private    # Run private tests only
#   ./scripts/e2e_test.sh --ws         # Run bounded WebSocket smoke tests
#   ./scripts/e2e_test.sh --private-no-precheck
# =============================================================================

set -euo pipefail

BINARY="${BINANCE_BIN:-./target/debug/binance}"
PAIR="${BINANCE_TEST_PAIR:-BTCUSDT}"
PAIR_LOWER=$(echo "$PAIR" | tr '[:upper:]' '[:lower:]')
TEST_COIN="${BINANCE_TEST_COIN:-USDT}"

PASS=0
FAIL=0
SKIP=0
TOTAL=0

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

log_header() {
    echo ""
    echo -e "${CYAN}${BOLD}══════════════════════════════════════════════${NC}"
    echo -e "${CYAN}${BOLD}  $1${NC}"
    echo -e "${CYAN}${BOLD}══════════════════════════════════════════════${NC}"
}

run_test() {
    local description="$1"
    shift
    TOTAL=$((TOTAL + 1))
    echo -n "  [$TOTAL] $description ... "

    local output
    local exit_code=0
    output=$("$@" 2>&1) || exit_code=$?

    if [ $exit_code -eq 0 ]; then
        echo -e "${GREEN}PASS${NC}"
        PASS=$((PASS + 1))
    else
        echo -e "${RED}FAIL${NC} (exit=$exit_code)"
        echo "       CMD: $*"
        echo "       OUT: $(echo "$output" | head -3)"
        FAIL=$((FAIL + 1))
    fi
}

run_test_json() {
    local description="$1"
    shift
    TOTAL=$((TOTAL + 1))
    echo -n "  [$TOTAL] $description ... "

    local output
    local exit_code=0
    output=$("$@" 2>&1) || exit_code=$?

    if [ $exit_code -eq 0 ]; then
        if echo "$output" | python3 -c "import sys, json; json.load(sys.stdin)" 2>/dev/null; then
            echo -e "${GREEN}PASS${NC} (valid JSON)"
            PASS=$((PASS + 1))
        else
            echo -e "${RED}FAIL${NC} (invalid JSON)"
            echo "       OUT: $(echo "$output" | head -3)"
            FAIL=$((FAIL + 1))
        fi
    else
        echo -e "${RED}FAIL${NC} (exit=$exit_code)"
        echo "       CMD: $*"
        echo "       OUT: $(echo "$output" | head -3)"
        FAIL=$((FAIL + 1))
    fi
}

skip_test() {
    local description="$1"
    TOTAL=$((TOTAL + 1))
    SKIP=$((SKIP + 1))
    echo -e "  [$TOTAL] $description ... ${YELLOW}SKIP${NC}"
}

RUN_PUBLIC=true
RUN_PRIVATE=true
RUN_WS=false
SKIP_PRIVATE_PRECHECK=false

if [[ "${1:-}" == "--public" ]]; then
    RUN_PRIVATE=false
elif [[ "${1:-}" == "--private" ]]; then
    RUN_PUBLIC=false
elif [[ "${1:-}" == "--ws" ]]; then
    RUN_PUBLIC=false
    RUN_PRIVATE=false
    RUN_WS=true
elif [[ "${1:-}" == "--private-no-precheck" ]]; then
    RUN_PUBLIC=false
    SKIP_PRIVATE_PRECHECK=true
fi

echo -e "${BOLD}Building binance-cli ...${NC}"
cargo build 2>&1 | tail -1

if [ ! -f "$BINARY" ]; then
    echo -e "${RED}Binary not found at $BINARY${NC}"
    exit 1
fi

echo -e "${GREEN}Binary: $BINARY${NC}"
echo -e "Test pair: ${CYAN}$PAIR${NC}"
echo ""

if $RUN_PUBLIC; then
log_header "PUBLIC - Market Data"

run_test "ping (table)" \
    $BINARY ping

run_test "ping (json)" \
    $BINARY -o json ping

run_test "server-time (table)" \
    $BINARY server-time

run_test_json "server-time (json)" \
    $BINARY -o json server-time

run_test "exchange-info" \
    $BINARY -o json exchange-info

run_test "ticker $PAIR (table)" \
    $BINARY ticker "$PAIR"

run_test_json "ticker $PAIR (json)" \
    $BINARY -o json ticker "$PAIR"

run_test_json "price $PAIR" \
    $BINARY -o json price "$PAIR"

run_test_json "book-ticker $PAIR" \
    $BINARY -o json book-ticker "$PAIR"

run_test_json "orderbook $PAIR" \
    $BINARY -o json orderbook "$PAIR" --count 5

run_test_json "trades $PAIR" \
    $BINARY -o json trades "$PAIR" --count 5

run_test_json "agg-trades $PAIR" \
    $BINARY -o json agg-trades "$PAIR" --count 5

run_test_json "ohlc $PAIR" \
    $BINARY -o json ohlc "$PAIR" --interval 1m --count 5

log_header "PUBLIC - CLI Features"

run_test "--help" \
    $BINARY --help

run_test "--version" \
    $BINARY --version

run_test "order --help" \
    $BINARY order --help

run_test "deposit --help" \
    $BINARY deposit --help

run_test "withdrawal --help" \
    $BINARY withdrawal --help

run_test "ws --help" \
    $BINARY ws --help

run_test "auth --help" \
    $BINARY auth --help
fi

if $RUN_PRIVATE; then
log_header "PRIVATE - Account & Funding (requires credentials)"

HAS_CREDS=false
AUTH_TEST_OUTPUT=""
AUTH_TEST_EXIT=0

if $SKIP_PRIVATE_PRECHECK; then
    HAS_CREDS=true
    echo -e "  ${YELLOW}Skipping credential precheck (--private-no-precheck)${NC}"
else
    AUTH_TEST_OUTPUT=$($BINARY auth test 2>&1) || AUTH_TEST_EXIT=$?
    if [ $AUTH_TEST_EXIT -eq 0 ]; then
        HAS_CREDS=true
    else
        echo -e "  ${YELLOW}Credential precheck failed - skipping private tests${NC}"
        echo -e "  Reason: $(echo "$AUTH_TEST_OUTPUT" | head -1)"
        echo -e "  Configure with: ${CYAN}binance auth set --api-key KEY --api-secret SECRET${NC}"
    fi
fi

if $HAS_CREDS; then
    run_test "auth test" \
        $BINARY auth test

    run_test "auth show" \
        $BINARY auth show

    run_test_json "account-info" \
        $BINARY -o json account-info

    run_test_json "balance" \
        $BINARY -o json balance

    run_test_json "trades-history $PAIR" \
        $BINARY -o json trades-history "$PAIR" --count 5

    run_test_json "order open-orders" \
        $BINARY -o json order open-orders --pair "$PAIR"

    run_test_json "order all-orders $PAIR" \
        $BINARY -o json order all-orders "$PAIR" --count 5

    run_test_json "deposit status $TEST_COIN" \
        $BINARY -o json deposit status --asset "$TEST_COIN"

    run_test_json "withdrawal status $TEST_COIN" \
        $BINARY -o json withdrawal status --asset "$TEST_COIN"
else
    skip_test "auth test"
    skip_test "auth show"
    skip_test "account-info"
    skip_test "balance"
    skip_test "trades-history"
    skip_test "order open-orders"
    skip_test "order all-orders"
    skip_test "deposit status"
    skip_test "withdrawal status"
fi
fi

if $RUN_WS; then
log_header "WEBSOCKET - Market & User Streams"

run_test "ws ticker $PAIR_LOWER" \
    $BINARY -o json ws ticker "$PAIR_LOWER" --limit 1 --seconds 15

run_test "ws depth $PAIR_LOWER" \
    $BINARY -o json ws depth "$PAIR_LOWER" --limit 1 --seconds 15

AUTH_TEST_OUTPUT=""
AUTH_TEST_EXIT=0
AUTH_TEST_OUTPUT=$($BINARY auth test 2>&1) || AUTH_TEST_EXIT=$?
if [ "${AUTH_TEST_EXIT:-0}" -eq 0 ]; then
    run_test "ws user" \
        $BINARY -o json ws user --limit 1 --seconds 5
else
    echo -e "  ${YELLOW}Credential precheck failed - skipping private WebSocket tests${NC}"
    echo -e "  Reason: $(echo "$AUTH_TEST_OUTPUT" | head -1)"
    skip_test "ws user"
fi
fi

echo ""
echo -e "${BOLD}══════════════════════════════════════════════${NC}"
echo -e "${BOLD}  E2E Test Results${NC}"
echo -e "${BOLD}══════════════════════════════════════════════${NC}"
echo -e "  Total:   ${BOLD}$TOTAL${NC}"
echo -e "  Passed:  ${GREEN}${BOLD}$PASS${NC}"
echo -e "  Failed:  ${RED}${BOLD}$FAIL${NC}"
echo -e "  Skipped: ${YELLOW}${BOLD}$SKIP${NC}"
echo -e "${BOLD}══════════════════════════════════════════════${NC}"

if [ $FAIL -gt 0 ]; then
    echo -e "${RED}${BOLD}SOME TESTS FAILED${NC}"
    exit 1
else
    echo -e "${GREEN}${BOLD}ALL TESTS PASSED${NC}"
    exit 0
fi
