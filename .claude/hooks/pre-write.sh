#!/bin/bash
#
# Pre-Write Hook for Nam Project
# Runs before Claude creates a new file
# Enforces documentation and platform requirements
#

FILE="$1"
CONTENT="$2"

# ===========================================
# RULE 1: Block comment required at top
# ===========================================
if [[ "$FILE" == *.rs ]] || [[ "$FILE" == *.nam ]]; then
    if ! echo "$CONTENT" | head -20 | grep -q "/\*"; then
        echo "ERROR: New files must have block comment at top"
        echo "Describe module purpose, key types, and functions"
        echo ""
        echo "Example:"
        echo "/*"
        echo " * Module: module_name"
        echo " *"
        echo " * Description of what this module does."
        echo " *"
        echo " * Key Types:"
        echo " * - TypeName: Description"
        echo " *"
        echo " * Key Functions:"
        echo " * - function_name(): Description"
        echo " */"
        exit 1
    fi
fi

# ===========================================
# RULE 2: New codegen files must have tests
# ===========================================
if [[ "$FILE" == *"codegen"* ]] && [[ "$FILE" == *.rs ]] && [[ "$FILE" != *"tests"* ]]; then
    echo "REMINDER: Add tests for all three targets in tests.rs"
    echo "  - test_feature_native()"
    echo "  - test_feature_node()"
    echo "  - test_feature_browser()"
fi

# ===========================================
# RULE 3: Stdlib files need platform annotation
# ===========================================
if [[ "$FILE" == *"stdlib"* ]] && [[ "$FILE" == *.nam ]]; then
    if ! echo "$CONTENT" | grep -q "#\[platforms"; then
        echo "WARNING: Stdlib files should declare platform support"
        echo "Add one of:"
        echo "  #[platforms(all)]           - works on all platforms"
        echo "  #[platforms(native, node)]  - NO browser support"
        echo "  #[platforms(browser)]       - browser only (async)"
    fi
fi

# ===========================================
# RULE 4: Check file size
# ===========================================
LINE_COUNT=$(echo "$CONTENT" | wc -l)
if [ "$LINE_COUNT" -gt 1000 ]; then
    echo "ERROR: New file exceeds 1000 lines ($LINE_COUNT lines)"
    echo "Split into smaller modules"
    exit 1
fi

# ===========================================
# RULE 5: No TODO/FIXME in new files
# ===========================================
if echo "$CONTENT" | grep -qE "TODO|FIXME"; then
    echo "WARNING: New file contains TODO/FIXME comments"
    echo "Consider completing these before creating the file"
fi

# ===========================================
# RULE 6: Stdlib types must be lowercase
# ===========================================
if [[ "$FILE" == *"stdlib"* ]] || [[ "$FILE" == *".nam" ]]; then
    if echo "$CONTENT" | grep -qE "struct (Request|Response|File|TcpStream|HttpClient|WebSocket)"; then
        echo "ERROR: Stdlib types must be lowercase"
        echo "Use: request, response, file, tcp_stream, http_client, websocket"
        exit 1
    fi
fi

echo "Pre-write checks passed for $FILE"
exit 0
