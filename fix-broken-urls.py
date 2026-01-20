#!/usr/bin/env python3
"""
Automated URL fixer for catalog.json
Finds working download URLs for broken entries using multiple strategies
ONLY adds URLs that have been verified to work
"""

import json
import requests
import sys
from urllib.parse import urlparse, urljoin
import re
import time
from bs4 import BeautifulSoup

def test_url(url, timeout=15):
    """Test if a URL is valid and returns 200"""
    try:
        resp = requests.head(url, allow_redirects=True, timeout=timeout, 
                           headers={'User-Agent': 'Mozilla/5.0'})
        is_valid = resp.status_code == 200
        return is_valid, resp.url if is_valid else None
    except:
        return False, None

def scrape_download_page(url, pattern=None):
    """Scrape a download page for ISO/IMG links"""
    try:
        resp = requests.get(url, timeout=15, headers={'User-Agent': 'Mozilla/5.0'})
        if resp.status_code != 200:
            return None
        
        soup = BeautifulSoup(resp.text, 'html.parser')
        
        # Find all links
        for link in soup.find_all('a', href=True):
            href = link['href']
            
            # Look for ISO/IMG files
            if re.search(r'\.(iso|img|img\.xz|img\.gz)$', href, re.I):
                if pattern and not re.search(pattern, href, re.I):
                    continue
                
                # Make absolute URL
                if not href.startswith('http'):
                    href = urljoin(url, href)
                
                # Validate it works
                print(f"    Found link: {href[:80]}...")
                is_valid, final = test_url(href)
                if is_valid:
                    print(f"    ✓ Validated")
                    return final
        
        return None
    except:
        return None

def fix_kubuntu(url):
    """Find current Kubuntu URL"""
    try:
        # Scrape releases page
        resp = requests.get('https://cdimage.ubuntu.com/kubuntu/releases/', timeout=10,
                          headers={'User-Agent': 'Mozilla/5.0'})
        soup = BeautifulSoup(resp.text, 'html.parser')
        
        # Find latest version directory
        versions = []
        for link in soup.find_all('a', href=True):
            if re.match(r'\d+\.\d+', link['href']):
                versions.append(link['href'].rstrip('/'))
        
        if versions:
            latest = sorted(versions, reverse=True)[0]
            check_url = f"https://cdimage.ubuntu.com/kubuntu/releases/{latest}/release/"
            return scrape_download_page(check_url, r'desktop.*amd64\.iso')
    except:
        pass
    return None

def fix_popos(url):
    """Find current Pop!_OS URL"""
    # Pop!_OS URL pattern: https://iso.pop-os.org/{version}/amd64/{type}/{build}/
    try:
        # Try recent versions
        for version in ['24.04', '22.04']:
            for build_type in ['intel', 'nvidia']:
                for build in range(60, 40, -1):  # Try builds 60 down to 40
                    test = f"https://iso.pop-os.org/{version}/amd64/{build_type}/{build}/pop-os_{version}_amd64_{build_type}_{build}.iso"
                    is_valid, final = test_url(test)
                    if is_valid:
                        return final
    except:
        pass
    return None

def fix_zorin(url):
    """Find current Zorin OS URL"""
    try:
        # Try SourceForge
        resp = requests.get('https://sourceforge.net/projects/zorinos/files/', timeout=10,
                          headers={'User-Agent': 'Mozilla/5.0'})
        return scrape_download_page('https://sourceforge.net/projects/zorinos/files/', r'Core.*64.*\.iso')
    except:
        pass
    return None

def fix_nobara(url):
    """Find current Nobara URL"""
    try:
        # Nobara uses direct download links now
        for version in [40, 41]:
            for month in ['2024-11', '2024-12', '2025-01']:
                test = f"https://nobaraproject.org/Nobara-{version}-Official-{month}.iso"
                is_valid, final = test_url(test)
                if is_valid:
                    return final
    except:
        pass
    return None

def fix_bazzite(name):
    """Find current Bazzite ISO"""
    try:
        # Check actual GitHub releases page
        resp = requests.get('https://github.com/ublue-os/bazzite/releases', timeout=10,
                          headers={'User-Agent': 'Mozilla/5.0'})
        soup = BeautifulSoup(resp.text, 'html.parser')
        
        # Look for ISO download links
        variant = 'gnome' if 'gnome' in name.lower() else 'deck'
        for link in soup.find_all('a', href=True):
            href = link['href']
            if f'bazzite-{variant}' in href and '.iso' in href:
                full_url = f"https://github.com{href}"
                is_valid, final = test_url(full_url)
                if is_valid:
                    return final
    except:
        pass
    return None

def fix_chimeraos(url):
    """Find current ChimeraOS URL"""
    try:
        resp = requests.get('https://github.com/ChimeraOS/chimeraos/releases', timeout=10,
                          headers={'User-Agent': 'Mozilla/5.0'})
        soup = BeautifulSoup(resp.text, 'html.parser')
        
        for link in soup.find_all('a', href=True):
            href = link['href']
            if 'chimeraos' in href and '.iso' in href:
                full_url = f"https://github.com{href}"
                is_valid, final = test_url(full_url)
                if is_valid:
                    return final
    except:
        pass
    return None

def find_github_release_url(repo_url, filename_pattern, distro_name):
    """Try to find latest GitHub release with matching filename"""
    try:
        # Extract owner/repo from URL
        match = re.search(r'github\.com/([^/]+/[^/]+)', repo_url)
        if not match:
            return None
        
        repo = match.group(1)
        
        # Try latest release first
        api_url = f"https://api.github.com/repos/{repo}/releases/latest"
        print(f"    Checking GitHub API: {api_url}")
        
        resp = requests.get(api_url, timeout=10, 
                          headers={'User-Agent': 'Mozilla/5.0'})
        if resp.status_code != 200:
            print(f"    API returned {resp.status_code}, trying all releases...")
            # Try all releases
            api_url = f"https://api.github.com/repos/{repo}/releases"
            resp = requests.get(api_url, timeout=10,
                              headers={'User-Agent': 'Mozilla/5.0'})
            if resp.status_code != 200:
                return None
            releases = resp.json()
        else:
            releases = [resp.json()]
        
        # Search through releases
        for release in releases[:5]:  # Check up to 5 most recent
            for asset in release.get('assets', []):
                asset_name = asset['name']
                asset_url = asset['browser_download_url']
                
                # Try multiple matching strategies
                if (re.search(filename_pattern, asset_name, re.IGNORECASE) or
                    asset_name.lower().endswith('.iso') or
                    asset_name.lower().endswith('.img.xz')):
                    
                    print(f"    Found potential match: {asset_name}")
                    print(f"    Testing URL: {asset_url}")
                    
                    # CRITICAL: Validate before returning
                    is_valid, final_url = test_url(asset_url)
                    if is_valid:
                        print(f"    ✓ URL validated successfully")
                        return final_url
                    else:
                        print(f"    ✗ URL validation failed")
        
        return None
    except Exception as e:
        print(f"    Error searching GitHub: {e}")
        return None

def find_sourceforge_url(project_name, filename_hint):
    """Try to find working SourceForge download URL"""
    try:
        # Try RSS feed for latest files
        api_url = f"https://sourceforge.net/projects/{project_name}/rss"
        resp = requests.get(api_url, timeout=10,
                          headers={'User-Agent': 'Mozilla/5.0'})
        if resp.status_code == 200:
            # Parse RSS for file links
            links = re.findall(r'https://sourceforge\.net/projects/[^"]+/files/[^"]+', resp.text)
            for link in links[:5]:
                if filename_hint.lower() in link.lower() or '.iso' in link.lower():
                    download_url = f"{link}/download"
                    print(f"    Testing SourceForge URL: {download_url}")
                    is_valid, final_url = test_url(download_url)
                    if is_valid:
                        print(f"    ✓ URL validated successfully")
                        return final_url
        return None
    except:
        return None

def main():
    with open('catalog.json', 'r') as f:
        data = json.load(f)
    
    # Handle both array and object with "distros" key
    if isinstance(data, dict) and 'distros' in data:
        catalog = data['distros']
        has_wrapper = True
    else:
        catalog = data
        has_wrapper = False
    
    print("=" * 80)
    print("  Automated URL Fixer - SAFE MODE")
    print("  Only updates URLs that have been validated to work")
    print("=" * 80)
    print()
    
    fixed_count = 0
    redirect_count = 0
    failed_distros = []
    skipped_working = 0
    
    for idx, distro in enumerate(catalog):
        name = distro.get('name', 'Unknown')
        url = distro.get('download_url', '')
        
        if not url:
            continue
        
        print(f"[{idx+1}/{len(catalog)}] {name}")
        print(f"  Current: {url[:80]}...")
        
        is_valid, final_url = test_url(url)
        
        if is_valid:
            if final_url != url:
                print(f"  ⚠ Redirects to: {final_url[:80]}...")
                # Update to direct URL to avoid redirect overhead
                distro['download_url'] = final_url
                redirect_count += 1
            else:
                print(f"  ✓ OK - working fine")
                skipped_working += 1
        else:
            print(f"  ✗ BROKEN - searching for replacement...")
            
            new_url = None
            
            # Distro-specific fixes (high success rate)
            if 'kubuntu' in name.lower():
                new_url = fix_kubuntu(url)
            elif 'pop' in name.lower() and 'os' in name.lower():
                new_url = fix_popos(url)
            elif 'zorin' in name.lower():
                new_url = fix_zorin(url)
            elif 'nobara' in name.lower():
                new_url = fix_nobara(url)
            elif 'bazzite' in name.lower():
                new_url = fix_bazzite(name)
            elif 'chimera' in name.lower():
                new_url = fix_chimeraos(url)
            
            # Generic strategies
            if not new_url:
                # Strategy 1: GitHub releases
                if 'github.com' in url:
                    filename = url.split('/')[-1]
                    base_pattern = re.sub(r'[-_]\d+.*', '', filename.split('.')[0])
                    new_url = find_github_release_url(url, base_pattern, name)
                    
                # Strategy 2: SourceForge
                elif 'sourceforge.net' in url:
                    match = re.search(r'sourceforge\.net/projects/([^/]+)', url)
                    if match:
                        project = match.group(1)
                        filename = url.split('/')[-2] if '/download' in url else url.split('/')[-1]
                        new_url = find_sourceforge_url(project, filename)
                
                # Strategy 3: Try version bumps for versioned URLs
                elif re.search(r'\d+\.\d+', url):
                    print(f"  → Trying version bumps...")
                    # Extract and try newer versions
                    versions = re.findall(r'(\d+)\.(\d+)', url)
                    if versions:
                        major, minor = map(int, versions[0])
                        for m in range(major, major + 2):
                            for n in range(minor, min(minor + 5, 50)):
                                test_url_str = re.sub(r'\d+\.\d+', f'{m}.{n}', url, count=1)
                                is_valid, final = test_url(test_url_str)
                                if is_valid:
                                    new_url = final
                                    break
                            if new_url:
                                break
                
                # Strategy 4: Fedora/Debian mirrors
                elif 'download.fedoraproject.org' in url or 'cdimage.debian.org' in url:
                    print(f"  → Mirrors may work - skipping for now")
                
            if new_url:
                # CRITICAL: Double-check the new URL works
                print(f"  Validating replacement URL...")
                is_new_valid, verified_url = test_url(new_url)
                
                if is_new_valid:
                    print(f"  ✓✓ FIXED with verified URL")
                    print(f"  New: {verified_url[:80]}...")
                    distro['download_url'] = verified_url
                    distro['verified'] = False  # Mark as unverified checksum
                    fixed_count += 1
                else:
                    print(f"  ✗✗ Replacement URL failed validation - keeping original")
                    failed_distros.append((name, url, "Found replacement but validation failed"))
            else:
                print(f"  ✗ Could not find working replacement")
                failed_distros.append((name, url, "No replacement found"))
        
        print()
        time.sleep(0.5)  # Rate limiting
    
    # Save updated catalog if any changes were made
    if fixed_count > 0 or redirect_count > 0:
        # Backup original first
        import shutil
        shutil.copy('catalog.json', 'catalog.json.backup')
        
        # Save with same structure as input
        if has_wrapper:
            output_data = data  # Keep original wrapper
        else:
            output_data = catalog
        
        with open('catalog.json.new', 'w') as f:
            json.dump(output_data, f, indent=2)
        
        print("=" * 80)
        print(f"✓ Successfully processed catalog:")
        print(f"  - Fixed broken URLs: {fixed_count}")
        print(f"  - Updated redirects: {redirect_count}")
        print(f"  - Already working: {skipped_working}")
        print(f"  - Still broken: {len(failed_distros)}")
        print()
        print(f"  Original backed up to: catalog.json.backup")
        print(f"  Updated catalog saved to: catalog.json.new")
        print(f"  Review changes and then: mv catalog.json.new catalog.json")
        print("=" * 80)
    else:
        print("=" * 80)
        print("No fixes made - all working URLs or no replacements found")
        print("=" * 80)
    
    if failed_distros:
        print()
        print("=" * 80)
        print(f"  Still broken ({len(failed_distros)} distros require manual fixes):")
        print("=" * 80)
        for name, url, reason in failed_distros[:20]:  # Show first 20
            print(f"  ✗ {name}")
            print(f"    URL: {url[:70]}...")
            print(f"    Reason: {reason}")
            print()
        if len(failed_distros) > 20:
            print(f"  ... and {len(failed_distros) - 20} more")
    
    return 0 if len(failed_distros) == 0 else 1

if __name__ == '__main__':
    sys.exit(main())
