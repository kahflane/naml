#!/bin/bash
#
# Pre-commit hook for Nam language project
# Enforces code quality rules before commits
#

set -e

echo "Running Nam pre-commit checks..."

# Check 1: No files over 1000 lines
echo "Checking file line counts..."
for file in $(find namc/src nam_stdlib/src -name "*.rs" -type f 2>/dev/null); do
    lines=$(wc -l < "$file")
    if [ "$lines" -gt 1000 ]; then
        echo "ERROR: $file exceeds 1000 lines ($lines lines)"
        exit 1
    fi
done
echo "✓ All files under 1000 lines"

# Check 2: No inline comments (// on same line as code)
echo "Checking for inline comments..."
for file in $(find namc/src nam_stdlib/src -name "*.rs" -type f 2>/dev/null); do
    if grep -n '[^/]//[^/!]' "$file" | grep -v '^\s*//' | grep -v 'http://' | grep -v 'https://' > /dev/null 2>&1; then
        echo "ERROR: Inline comments found in $file"
        grep -n '[^/]//[^/!]' "$file" | grep -v '^\s*//' | grep -v 'http://' | head -5
        exit 1
    fi
done
echo "✓ No inline comments found"

# Check 3: Block comment headers at file top
echo "Checking for block comment headers..."
for file in $(find namc/src nam_stdlib/src -name "*.rs" -type f 2>/dev/null); do
    if ! head -1 "$file" | grep -q '^/\*' ; then
        echo "ERROR: $file missing block comment header"
        exit 1
    fi
done
echo "✓ Block comments present"

# Check 4: Naming convention check
echo "Checking naming conventions..."
for file in $(find namc/src nam_stdlib/src -name "*.rs" -type f 2>/dev/null); do
    if grep -E 'pub (struct|enum) [a-z]' "$file" > /dev/null 2>&1; then
        echo "WARNING: Non-PascalCase type in $file"
        grep -n -E 'pub (struct|enum) [a-z]' "$file"
    fi
done
echo "✓ Naming conventions checked"

# Check 5: Run tests
echo "Running tests..."
if ! cargo test --quiet 2>&1; then
    echo "ERROR: Tests failed"
    exit 1
fi
echo "✓ All tests passed"

# Check 6: Run clippy
echo "Running clippy..."
if ! cargo clippy --quiet -- -D warnings 2>&1; then
    echo "ERROR: Clippy found issues"
    exit 1
fi
echo "✓ Clippy passed"

# Check 7: Multi-platform codegen check
echo "Checking multi-platform support in codegen..."
for file in $(find namc/src/codegen -name "*.rs" -type f 2>/dev/null); do
    if grep -q "match target" "$file"; then
        if ! (grep -q "Target::Browser" "$file" && \
              grep -q "Target::Node" "$file" && \
              grep -q "Target::Native" "$file"); then
            echo "WARNING: $file has 'match target' but may be missing target variants"
            echo "Ensure Browser, Node, and Native are all handled"
        fi
    fi
done
echo "✓ Multi-platform checks passed"

# Check 8: No web_sys in codegen
echo "Checking for forbidden patterns..."
for file in $(find namc/src/codegen -name "*.rs" -type f 2>/dev/null); do
    if grep -q "web_sys::" "$file"; then
        echo "ERROR: $file uses web_sys directly"
        echo "Use extern functions or crate::__nam_print instead"
        exit 1
    fi
done
echo "✓ No forbidden patterns"

# Check 9: Stdlib type naming (lowercase)
echo "Checking stdlib type naming..."
for file in $(find nam_stdlib -name "*.nam" -type f 2>/dev/null); do
    if grep -qE "struct (Request|Response|File|TcpStream)" "$file"; then
        echo "ERROR: $file has uppercase stdlib types"
        echo "Use lowercase: request, response, file, tcp_stream"
        exit 1
    fi
done
echo "✓ Stdlib naming correct"

echo ""
echo "All pre-commit checks passed!"
