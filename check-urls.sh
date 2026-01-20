#!/bin/bash
# Comprehensive URL validation script for catalog.json
# Tests all download links and reports failures with details

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

CATALOG="catalog.json"
FAILED_URLS=()
REDIRECT_URLS=()
SUCCESS_COUNT=0
TOTAL_COUNT=0

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  ISO Download URL Validation${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Extract URLs with their associated distro names
while IFS= read -r line; do
    # Get distro name
    if echo "$line" | grep -q '"name"'; then
        DISTRO_NAME=$(echo "$line" | sed 's/.*"name": "\(.*\)".*/\1/')
    fi
    
    # Get download URL
    if echo "$line" | grep -q '"download_url"'; then
        URL=$(echo "$line" | sed 's/.*"download_url": "\(.*\)".*/\1/')
        TOTAL_COUNT=$((TOTAL_COUNT + 1))
        
        echo -ne "${BLUE}[$TOTAL_COUNT]${NC} Testing: ${DISTRO_NAME}...\n"
        echo -ne "    URL: $URL\n"
        
        # Perform HEAD request with redirects, timeout 15s
        HTTP_CODE=$(curl -o /dev/null -s -w "%{http_code}" -L -I "$URL" --max-time 15 --connect-timeout 10 2>&1 || echo "000")
        FINAL_URL=$(curl -s -L -I -w "%{url_effective}" -o /dev/null "$URL" --max-time 15 --connect-timeout 10 2>&1 || echo "$URL")
        
        if [ "$HTTP_CODE" = "200" ]; then
            echo -e "    ${GREEN}✓ OK${NC} (HTTP $HTTP_CODE)"
            SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
            
            # Check if URL was redirected
            if [ "$FINAL_URL" != "$URL" ]; then
                echo -e "    ${YELLOW}⚠ Redirected to:${NC} $FINAL_URL"
                REDIRECT_URLS+=("$DISTRO_NAME|$URL|$FINAL_URL")
            fi
        elif [ "$HTTP_CODE" = "302" ] || [ "$HTTP_CODE" = "301" ]; then
            echo -e "    ${YELLOW}⚠ REDIRECT${NC} (HTTP $HTTP_CODE)"
            echo -e "    ${YELLOW}→ Final URL:${NC} $FINAL_URL"
            REDIRECT_URLS+=("$DISTRO_NAME|$URL|$FINAL_URL")
            SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
        else
            echo -e "    ${RED}✗ FAILED${NC} (HTTP $HTTP_CODE)"
            FAILED_URLS+=("$DISTRO_NAME|$URL|$HTTP_CODE")
        fi
        echo ""
    fi
done < "$CATALOG"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Summary${NC}"
echo -e "${BLUE}========================================${NC}"
echo -e "Total URLs tested: ${BLUE}$TOTAL_COUNT${NC}"
echo -e "Successful: ${GREEN}$SUCCESS_COUNT${NC}"
echo -e "Failed: ${RED}${#FAILED_URLS[@]}${NC}"
echo -e "Redirects: ${YELLOW}${#REDIRECT_URLS[@]}${NC}"
echo ""

if [ ${#FAILED_URLS[@]} -gt 0 ]; then
    echo -e "${RED}========================================${NC}"
    echo -e "${RED}  Failed URLs (need fixing!)${NC}"
    echo -e "${RED}========================================${NC}"
    for entry in "${FAILED_URLS[@]}"; do
        IFS='|' read -r name url code <<< "$entry"
        echo -e "${RED}✗${NC} $name"
        echo -e "  URL: $url"
        echo -e "  Error: HTTP $code"
        echo ""
    done
fi

if [ ${#REDIRECT_URLS[@]} -gt 0 ]; then
    echo -e "${YELLOW}========================================${NC}"
    echo -e "${YELLOW}  Redirected URLs (consider updating)${NC}"
    echo -e "${YELLOW}========================================${NC}"
    for entry in "${REDIRECT_URLS[@]}"; do
        IFS='|' read -r name url final <<< "$entry"
        echo -e "${YELLOW}⚠${NC} $name"
        echo -e "  Old: $url"
        echo -e "  New: $final"
        echo ""
    done
fi

if [ ${#FAILED_URLS[@]} -eq 0 ]; then
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}  ✓ All URLs valid!${NC}"
    echo -e "${GREEN}========================================${NC}"
    exit 0
else
    echo -e "${RED}========================================${NC}"
    echo -e "${RED}  ✗ ${#FAILED_URLS[@]} URLs need attention!${NC}"
    echo -e "${RED}========================================${NC}"
    exit 1
fi
