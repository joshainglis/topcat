# Continuous Integration Setup

## GitHub Actions

### Basic Test Pipeline

```yaml
# .github/workflows/test.yml
name: Tests

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

jobs:
  test:
    runs-on: ubuntu-latest

    steps:
    - uses: actions/checkout@v3

    - name: Setup Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        override: true

    - name: Cache cargo
      uses: actions/cache@v3
      with:
        path: |
          ~/.cargo/registry
          ~/.cargo/git
          target
        key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

    - name: Run tests
      run: cargo test --all-features

    - name: Run clippy
      run: cargo clippy -- -D warnings

    - name: Check formatting
      run: cargo fmt -- --check
```

### Multi-Platform Testing

```yaml
name: Cross-Platform Tests

on: [push, pull_request]

jobs:
  test:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        rust: [stable, beta, nightly]

    runs-on: ${{ matrix.os }}

    steps:
    - uses: actions/checkout@v3

    - uses: actions-rs/toolchain@v1
      with:
        toolchain: ${{ matrix.rust }}
        override: true

    - run: cargo test --all-features

    continue-on-error: ${{ matrix.rust == 'nightly' }}
```

### Coverage Reporting

```yaml
name: Coverage

on: [push]

jobs:
  coverage:
    runs-on: ubuntu-latest

    steps:
    - uses: actions/checkout@v3

    - name: Install tarpaulin
      run: cargo install cargo-tarpaulin

    - name: Generate coverage
      run: cargo tarpaulin --out Xml

    - name: Upload to codecov
      uses: codecov/codecov-action@v3
      with:
        files: ./cobertura.xml
```

## Pre-commit Hooks

### Setup husky

```json
// package.json
{
  "scripts": {
    "prepare": "husky install"
  },
  "devDependencies": {
    "husky": "^8.0.0"
  }
}
```

```bash
# .husky/pre-commit
#!/bin/sh
. "$(dirname "$0")/_/husky.sh"

cargo fmt --check
cargo clippy -- -D warnings
cargo test --lib
```

### Git hooks (native)

```bash
# .git/hooks/pre-commit
#!/bin/sh
set -e

echo "Running pre-commit checks..."

# Format check
cargo fmt -- --check

# Lint
cargo clippy -- -D warnings

# Quick tests
cargo test --lib

echo "Pre-commit checks passed!"
```

## Nix CI

```nix
# flake.nix CI job
{
  checks = {
    tests = pkgs.runCommand "tests" {} ''
      ${topcat}/bin/topcat --version
      cargo test --all-features
      cargo clippy -- -D warnings
    '';
  };
}
```

## Release Automation

### GitHub Release

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    runs-on: ubuntu-latest

    steps:
    - uses: actions/checkout@v3

    - name: Build release
      run: cargo build --release

    - name: Create Release
      uses: actions/create-release@v1
      env:
        GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
      with:
        tag_name: ${{ github.ref }}
        release_name: Release ${{ github.ref }}
        draft: false
        prerelease: false

    - name: Upload Release Asset
      uses: actions/upload-release-asset@v1
      env:
        GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
      with:
        upload_url: ${{ steps.create_release.outputs.upload_url }}
        asset_path: ./target/release/topcat
        asset_name: topcat-linux-amd64
        asset_content_type: application/octet-stream
```