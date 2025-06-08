#!/bin/bash

# Quick coverage check for development
echo "🏃‍♂️ Quick Coverage Check"
echo "========================"

# Check if tarpaulin is available
if ! command -v cargo-tarpaulin &> /dev/null; then
    echo "❌ cargo-tarpaulin not found. Install with: cargo install cargo-tarpaulin"
    exit 1
fi

# Run quick coverage without cleaning
echo "Running quick coverage analysis..."
cargo tarpaulin --skip-clean --target-dir target/tarpaulin --timeout 120 | \
    grep -E "(coverage|Tested/Total|src/.*\.rs:)" | \
    sed 's/||//' | \
    while IFS= read -r line; do
        if [[ $line == *"coverage"* ]]; then
            echo "📊 $line"
        elif [[ $line == *"Tested/Total"* ]]; then
            echo "📋 $line"
        else
            echo "📁 $line"
        fi
    done

echo ""
echo "✅ Quick coverage check complete!"
echo "💡 For detailed analysis, run: ./scripts/coverage.sh"