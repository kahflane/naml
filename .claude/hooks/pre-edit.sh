#!/bin/bash
#
# Pre-Edit Hook for Nam Project
# Runs before Claude edits any file
# Enforces multi-platform support, code style, and performance rules
#

FILE="$1"
NEW_CONTENT="$2"

# ===========================================
# RULE 1: Multi-Platform Check for Codegen
# ===========================================
if [[ "$FILE" == *"codegen"* ]]; then
    if echo "$NEW_CONTENT" | grep -q "match target" && \
       ! (echo "$NEW_CONTENT" | grep -q "Target::Browser" && \
          echo "$NEW_CONTENT" | grep -q "Target::Node" && \
          echo "$NEW_CONTENT" | grep -q "Target::Native"); then
        echo "ERROR: Codegen must handle ALL targets (Browser, Node, Native)"
        echo "Missing target variant in match expression"
        exit 1
    fi
fi

# ===========================================
# RULE 2: No web_sys in codegen (use extern)
# ===========================================
if [[ "$FILE" == *"codegen"* ]] && echo "$NEW_CONTENT" | grep -q "web_sys::"; then
    echo "ERROR: Do not use web_sys directly in codegen"
    echo "Use crate::__nam_print or extern functions instead"
    exit 1
fi

# ===========================================
# RULE 3: File size limit (1000 lines)
# ===========================================
LINE_COUNT=$(echo "$NEW_CONTENT" | wc -l)
if [ "$LINE_COUNT" -gt 1000 ]; then
    echo "ERROR: File exceeds 1000 lines ($LINE_COUNT lines)"
    echo "Split into smaller modules"
    exit 1
fi

# ===========================================
# RULE 4: No inline comments (except URLs)
# ===========================================
if echo "$NEW_CONTENT" | grep -E "^[^/\"']*[^/]//[^/!]" | grep -v "https\?://" | head -1 | grep -q .; then
    echo "WARNING: Inline comments detected"
    echo "Prefer block comments at the top of the file"
fi

# ===========================================
# RULE 5: Stdlib types must be lowercase
# ===========================================
if [[ "$FILE" == *"stdlib"* ]] || [[ "$FILE" == *".nam" ]]; then
    if echo "$NEW_CONTENT" | grep -qE "struct (Request|Response|File|TcpStream|HttpClient|WebSocket)"; then
        echo "ERROR: Stdlib types must be lowercase"
        echo "Use: request, response, file, tcp_stream, http_client, websocket"
        exit 1
    fi
fi

# ===========================================
# RULE 6: Check for allocation patterns in compiler
# ===========================================
if [[ "$FILE" == *"namc/src"* ]]; then
    if echo "$NEW_CONTENT" | grep -q 'format!.*output\.push_str'; then
        echo "WARNING: Avoid format! + push_str pattern"
        echo "Use write!() or direct string building instead"
    fi

    if echo "$NEW_CONTENT" | grep -qE '\.clone\(\).*\.clone\(\)'; then
        echo "WARNING: Multiple .clone() calls detected"
        echo "Consider using references instead"
    fi
fi

# ===========================================
# RULE 7: Browser platform constraints
# ===========================================
if [[ "$FILE" == *"codegen"* ]] || [[ "$FILE" == *"stdlib"* ]]; then
    if echo "$NEW_CONTENT" | grep -q "Target::Browser" && \
       echo "$NEW_CONTENT" | grep -qE "std::fs|std::net|std::process|std::env|thread::sleep"; then
        echo "ERROR: Browser target cannot use filesystem, network, process, or blocking calls"
        echo "These are only available on Native and Node targets"
        exit 1
    fi
fi

echo "Pre-edit checks passed for $FILE"
exit 0
