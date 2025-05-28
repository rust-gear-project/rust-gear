#!/bin/bash

set -e

# check if at least one package name is provided
# Usage: ./scripts/publish.sh <package1,package2,package3>
if [ $# -eq 0 ]; then 
    echo "Usage: $0 <package1,package2,package3>"
    echo "Example: $0 glob,another-package"
    exit 1
fi

# check if script is run from project root directory
if [ ! -f "package.json" ]; then
    echo "Error: Must be run from project root directory"
    exit 1
fi

PACKAGES="$1"
echo "Publishing packages: $PACKAGES"

IFS=',' read -ra PACKAGE_LIST <<< "$PACKAGES"
for package in "${PACKAGE_LIST[@]}"; do
    echo "=== Processing package: $package ==="
    
    if [ -d "packages/$package/npm" ]; then
        echo "Found npm directory for $package"
        cd "packages/$package/npm"
        
        # napi binary publish
        for platform_dir in */; do
            if [ -d "$platform_dir" ]; then
                platform_name="${platform_dir%/}"
                echo "Publishing $platform_name platform package"
                
                cd "$platform_dir"
                
                # check if package.json exists
                if [ -f "package.json" ]; then
                    npm publish --access public --provenance
                    echo "✅ Published $package-$platform_name"
                else
                    echo "⚠️  No package.json found in $platform_dir, skipping"
                fi
                
                cd ..
            fi
        done
        
        cd ../../..
        echo "✅ Completed publishing $package"
    else
        echo "❌ npm directory not found for $package, skipping"
    fi
    
    echo ""
done

echo "🎉 All packages published successfully!"