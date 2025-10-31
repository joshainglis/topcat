# Performance Testing

## Benchmarking

### Using cargo bench

```rust
// benches/benchmark.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_topological_sort(c: &mut Criterion) {
    let graph = create_large_graph(1000);

    c.bench_function("topo_sort_1000", |b| {
        b.iter(|| {
            topological_sort(black_box(&graph))
        });
    });
}

criterion_group!(benches, benchmark_topological_sort);
criterion_main!(benches);
```

Run benchmarks:
```bash
cargo bench
cargo bench -- --save-baseline before
cargo bench -- --baseline before  # Compare with saved
```

### Manual Timing

```bash
# Basic timing
time cargo run --release -- -i large_input/ -o output.sql

# Detailed timing (macOS)
/usr/bin/time -l cargo run --release -- -i input/ -o output.sql

# Detailed timing (Linux)
/usr/bin/time -v cargo run --release -- -i input/ -o output.sql
```

## Profiling

### Flamegraph

```bash
# Install
cargo install flamegraph

# Generate flamegraph
cargo flamegraph --bin topcat -- -i input/ -o output.sql

# With release optimizations
cargo flamegraph --release --bin topcat -- -i input/ -o output.sql

# Open result
open flamegraph.svg
```

### perf (Linux)

```bash
# Record performance data
perf record -g cargo run --release -- -i input/ -o output.sql

# View report
perf report

# Generate flamegraph
perf script | flamegraph.pl > flamegraph.svg
```

### Instruments (macOS)

```bash
# Build release with debug info
cargo build --release

# Run with Instruments
instruments -t "Time Profiler" target/release/topcat -- -i input/ -o output.sql
```

## Memory Profiling

### Valgrind (Linux)

```bash
# Install valgrind
sudo apt install valgrind

# Run with massif
valgrind --tool=massif cargo run -- -i input/ -o output.sql

# View results
ms_print massif.out.<pid>
```

### heaptrack (Linux)

```bash
# Install heaptrack
sudo apt install heaptrack

# Profile memory
heaptrack cargo run -- -i input/ -o output.sql

# Analyze results
heaptrack --analyze heaptrack.topcat.<pid>.gz
```

## Performance Optimization

### Common Bottlenecks

```rust
// String allocations
// Bad:
let result = format!("{}{}", s1, s2);

// Better:
let mut result = String::with_capacity(s1.len() + s2.len());
result.push_str(&s1);
result.push_str(&s2);

// HashMap capacity
// Bad:
let mut map = HashMap::new();

// Better:
let mut map = HashMap::with_capacity(expected_size);
```

### Parallel Processing

```rust
use rayon::prelude::*;

// Process files in parallel
let results: Vec<_> = files
    .par_iter()
    .map(|file| process_file(file))
    .collect();
```

## Load Testing

### Generate Test Data

```rust
// Generate large test dataset
fn generate_files(count: usize, dir: &Path) {
    for i in 0..count {
        let content = format!(
            "-- name: file_{}\n-- requires: {}\n",
            i,
            if i > 0 { format!("file_{}", i - 1) } else { String::new() }
        );

        let path = dir.join(format!("file_{}.sql", i));
        std::fs::write(path, content).unwrap();
    }
}
```

### Stress Testing

```bash
# Test with increasing file counts
for n in 100 500 1000 5000 10000; do
    echo "Testing with $n files..."
    generate_test_files $n /tmp/test_$n
    time cargo run --release -- -i /tmp/test_$n -o /tmp/out.sql
done
```

## Monitoring

### Resource Usage

```bash
# Monitor in real-time (Linux)
pidstat -r -p $(pgrep topcat) 1  # Memory
pidstat -u -p $(pgrep topcat) 1  # CPU

# Monitor in real-time (macOS)
top -pid $(pgrep topcat)
```

### Profile-Guided Optimization

```bash
# Build with PGO instrumentation
RUSTFLAGS="-Cprofile-generate=/tmp/pgo-data" \
  cargo build --release

# Run typical workloads
./target/release/topcat -i typical_input/ -o output.sql

# Build with PGO optimization
RUSTFLAGS="-Cprofile-use=/tmp/pgo-data" \
  cargo build --release
```