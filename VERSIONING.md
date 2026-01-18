# Automatic Versioning System

Etch uses an automatic versioning system based on git history and conventional commits.

## How It Works

1. **Base Version**: Starts from the latest git tag (or 0.1.0 if no tags exist)
2. **Commit Analysis**: Analyzes commit messages to determine version bump
3. **Auto-Increment**: Generates new version at compile time

## Commit Message Convention

Use these prefixes in your commit messages to control version bumps:

### MAJOR Version (Breaking Changes)
- `BREAKING:` or `!:` prefix
- `major:` prefix
- Example: `BREAKING: Remove deprecated API`
- Increments: `1.2.3` → `2.0.0`

### MINOR Version (New Features)
- `feat:` or `feature:` prefix
- `add:` prefix
- Example: `feat: Add dark mode support`
- Increments: `1.2.3` → `1.3.0`

### PATCH Version (Bug Fixes)
- `fix:` or `bugfix:` prefix
- `patch:` prefix
- Example: `fix: Resolve icon scaling issue`
- Increments: `1.2.3` → `1.2.4`

### Default Behavior
- Any commit without specific prefix: PATCH bump
- Multiple changes: Highest priority wins (MAJOR > MINOR > PATCH)

## Version Display

The version is displayed in format: `{VERSION} · {BRANCH} ({GIT_HASH})`

Examples:
- `0.2.1 · NIGHTLY (6c5b2ed)`
- `1.0.0 · STABLE (abc1234)`
- `0.3.0-dev · DEV (def5678)`

## Branch Classifications

- `main` or `master` → STABLE
- `*nightly*` → NIGHTLY
- `*dev*` → DEV
- Other branches → Branch name displayed

## Creating Releases

To create a new release:

1. Ensure your commits follow the convention
2. Build the project: `cargo build --release`
3. Create git tag: `git tag v{VERSION}`
4. Push tag: `git push origin v{VERSION}`
5. Create GitHub release with tag and attach binary

The auto-updater will detect new releases based on git tags.

## Manual Version Override

To manually set a version tag:
```bash
git tag v1.0.0
```

The build system will use this as the base and calculate bumps from there.
