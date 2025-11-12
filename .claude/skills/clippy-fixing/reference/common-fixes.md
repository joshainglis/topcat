# Common Clippy Fixes

## Performance Warnings

### unnecessary_to_owned

**Problem**: Converting borrowed data to owned unnecessarily.

```rust
// Before (clippy warning)
fn process_name(node: &FileNode) {
    let name = node.name.to_string();  // Already String
    println!("{}", name);
}

// After (fixed)
fn process_name(node: &FileNode) {
    println!("{}", node.name);  // Use reference
}
```

### redundant_clone

**Problem**: Cloning when not needed.

```rust
// Before (clippy warning)
let path = node.path.clone();
process(&path);

fn process(p: &Path) { }

// After (fixed)
process(&node.path);  // No clone needed
```

### needless_collect

**Problem**: Collecting into intermediate collection.

```rust
// Before (clippy warning)
let names: Vec<_> = nodes.iter().map(|n| &n.name).collect();
let count = names.len();

// After (fixed)
let count = nodes.iter().map(|n| &n.name).count();
```

### useless_conversion

**Problem**: Converting to same type.

```rust
// Before (clippy warning)
let s: String = string_value.into();  // Already String

// After (fixed)
let s = string_value;
```

## Idiomatic Rust

### manual_filter_map

**Problem**: Using filter + map instead of filter_map.

```rust
// Before (clippy warning)
let deps: Vec<_> = items
    .iter()
    .filter(|x| x.is_some())
    .map(|x| x.unwrap())
    .collect();

// After (fixed)
let deps: Vec<_> = items
    .iter()
    .filter_map(|x| x.as_ref())
    .collect();
```

### manual_flatten

**Problem**: Using filter_map for flattening.

```rust
// Before (clippy warning)
let all: Vec<_> = nested
    .iter()
    .filter_map(|x| x.as_ref())
    .collect();

// After (fixed - if x is Option<Vec<T>>)
let all: Vec<_> = nested
    .iter()
    .flatten()
    .collect();
```

### single_match

**Problem**: Match with only one meaningful arm.

```rust
// Before (clippy warning)
match result {
    Ok(value) => process(value),
    Err(_) => {}
}

// After (fixed)
if let Ok(value) = result {
    process(value);
}
```

### match_bool

**Problem**: Matching on boolean.

```rust
// Before (clippy warning)
let result = match is_valid {
    true => "yes",
    false => "no",
};

// After (fixed)
let result = if is_valid { "yes" } else { "no" };
```

### if_then_some_else_none

**Problem**: Manual Option construction.

```rust
// Before (clippy warning)
let result = if condition {
    Some(value)
} else {
    None
};

// After (fixed)
let result = condition.then_some(value);

// Or with closure if value is expensive:
let result = condition.then(|| expensive_computation());
```

### needless_bool

**Problem**: Returning boolean from if/else with boolean values.

```rust
// Before (clippy warning)
fn is_valid(x: i32) -> bool {
    if x > 0 {
        true
    } else {
        false
    }
}

// After (fixed)
fn is_valid(x: i32) -> bool {
    x > 0
}
```

## String Handling

### useless_format

**Problem**: Using format! for simple conversions.

```rust
// Before (clippy warning)
let s = format!("{}", value);

// After (fixed)
let s = value.to_string();
```

### string_add

**Problem**: Using + for string concatenation in loop.

```rust
// Before (clippy warning)
let mut result = String::new();
for item in items {
    result = result + &item;  // Inefficient
}

// After (fixed)
let mut result = String::new();
for item in items {
    result.push_str(&item);
}

// Or better:
let result = items.join("");
```

### str_to_string

**Problem**: Using to_string() on &str when to_owned() is clearer.

```rust
// Before (clippy warning in some contexts)
let s: String = str_ref.to_string();

// After (clippy prefers - though both work)
let s: String = str_ref.to_owned();
```

## Option/Result Handling

### map_unwrap_or

**Problem**: Calling map then unwrap_or.

```rust
// Before (clippy warning)
let result = option
    .map(|x| x * 2)
    .unwrap_or(0);

// After (fixed)
let result = option
    .map_or(0, |x| x * 2);
```

### or_fun_call

**Problem**: Calling function in unwrap_or instead of unwrap_or_else.

```rust
// Before (clippy warning)
let value = option.unwrap_or(expensive_default());  // Always calls function

// After (fixed)
let value = option.unwrap_or_else(expensive_default);  // Lazy evaluation
```

### ok_expect

**Problem**: Using .ok().expect() instead of just .expect().

```rust
// Before (clippy warning)
let value = result.ok().expect("failed");

// After (fixed)
let value = result.expect("failed");
```

### map_flatten

**Problem**: Calling map then flatten.

```rust
// Before (clippy warning)
let result = option
    .map(|x| parse(x))
    .flatten();

// After (fixed)
let result = option.and_then(|x| parse(x));
```

## Collection Operations

### unnecessary_sort_by

**Problem**: Using sort_by with simple comparison.

```rust
// Before (clippy warning)
nodes.sort_by(|a, b| a.name.cmp(&b.name));

// After (fixed)
nodes.sort_by_key(|n| &n.name);
```

### iter_next_loop

**Problem**: Using loop with iterator.next() instead of while let.

```rust
// Before (clippy warning)
loop {
    match iter.next() {
        Some(item) => process(item),
        None => break,
    }
}

// After (fixed)
while let Some(item) = iter.next() {
    process(item);
}

// Or better:
for item in iter {
    process(item);
}
```

### needless_range_loop

**Problem**: Looping over range to index.

```rust
// Before (clippy warning)
for i in 0..nodes.len() {
    process(&nodes[i]);
}

// After (fixed)
for node in &nodes {
    process(node);
}
```

## Comparison and Equality

### comparison_to_empty

**Problem**: Comparing length to zero.

```rust
// Before (clippy warning)
if deps.len() == 0 {
    // ...
}

// After (fixed)
if deps.is_empty() {
    // ...
}
```

### len_without_is_empty

**Problem**: Implementing len() without is_empty().

```rust
// Before (clippy warning)
impl MyCollection {
    pub fn len(&self) -> usize {
        self.items.len()
    }
}

// After (fixed)
impl MyCollection {
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}
```

### neg_cmp_op_on_partial_ord

**Problem**: Negating comparison instead of using opposite.

```rust
// Before (clippy warning)
if !(a < b) {
    // ...
}

// After (fixed)
if a >= b {
    // ...
}
```

## Error Handling

### result_unit_err

**Problem**: Using Result<T, ()> instead of Option<T>.

```rust
// Before (clippy warning)
fn try_parse(s: &str) -> Result<i32, ()> {
    if s.is_empty() {
        Err(())
    } else {
        Ok(s.len() as i32)
    }
}

// After (fixed)
fn try_parse(s: &str) -> Option<i32> {
    if s.is_empty() {
        None
    } else {
        Some(s.len() as i32)
    }
}
```

### unnecessary_unwrap

**Problem**: Unwrapping after checking.

```rust
// Before (clippy warning)
if result.is_ok() {
    let value = result.unwrap();
    process(value);
}

// After (fixed)
if let Ok(value) = result {
    process(value);
}
```

## Type Conversions

### from_over_into

**Problem**: Implementing Into instead of From.

```rust
// Before (clippy warning)
impl Into<String> for MyType {
    fn into(self) -> String {
        self.value
    }
}

// After (fixed)
impl From<MyType> for String {
    fn from(val: MyType) -> String {
        val.value
    }
}
// Note: From automatically provides Into
```

### unnecessary_cast

**Problem**: Casting to same type.

```rust
// Before (clippy warning)
let x: u32 = value as u32;  // value already u32

// After (fixed)
let x: u32 = value;
```

## Pattern Matching

### single_char_pattern

**Problem**: Using string for single character.

```rust
// Before (clippy warning)
let parts: Vec<_> = s.split(".").collect();

// After (fixed)
let parts: Vec<_> = s.split('.').collect();
```

### redundant_pattern_matching

**Problem**: Using match for simple checks.

```rust
// Before (clippy warning)
let is_some = match option {
    Some(_) => true,
    None => false,
};

// After (fixed)
let is_some = option.is_some();
```

### wildcard_enum_match_arm

**Problem**: Using wildcard for enum match.

```rust
// Before (clippy warning)
match error {
    Error::Io(_) => handle_io(),
    _ => handle_other(),  // Might miss new variants
}

// After (fixed - if appropriate)
match error {
    Error::Io(_) => handle_io(),
    Error::Parse(_) => handle_parse(),
    Error::Config(_) => handle_config(),
}
```

## Documentation

### missing_safety_doc

**Problem**: Unsafe function without safety documentation.

```rust
// Before (clippy warning)
pub unsafe fn dangerous_operation() {
    // ...
}

// After (fixed)
/// Performs dangerous operation.
///
/// # Safety
///
/// Caller must ensure that the pointer is valid and properly aligned.
pub unsafe fn dangerous_operation() {
    // ...
}
```

### doc_markdown

**Problem**: Code/types in docs not formatted properly.

```rust
// Before (clippy warning)
/// Process the FileNode
pub fn process(node: &FileNode) { }

// After (fixed)
/// Process the `FileNode`
pub fn process(node: &FileNode) { }
```

## Complexity

### too_many_arguments

**Problem**: Function has too many parameters.

```rust
// Before (clippy warning)
fn create_node(
    name: String,
    path: PathBuf,
    deps: HashSet<String>,
    layer: String,
    comment: String,
    source: NameSource,
) -> FileNode { }

// After (fixed - use builder or struct)
struct NodeBuilder {
    name: String,
    path: PathBuf,
    deps: HashSet<String>,
    layer: Option<String>,
    comment: Option<String>,
    source: NameSource,
}

impl NodeBuilder {
    fn build(self) -> FileNode { }
}
```

### type_complexity

**Problem**: Type too complex.

```rust
// Before (clippy warning)
fn process() -> HashMap<String, Vec<(String, Option<PathBuf>)>> { }

// After (fixed - use type alias)
type DependencyMap = HashMap<String, Vec<(String, Option<PathBuf>)>>;

fn process() -> DependencyMap { }
```

## Common Patterns in Topcat

### Path Handling

```rust
// Good
fn get_extension(path: &Path) -> Option<&str> {
    path.extension().and_then(|e| e.to_str())
}

// Avoid
fn get_extension(path: &Path) -> Option<String> {
    path.extension()
        .map(|e| e.to_string_lossy().to_string())  // Unnecessary conversion
}
```

### Iterator Sorting

```rust
// Good
let mut deps: Vec<_> = node.deps.iter().collect();
deps.sort();

// Avoid
let deps: Vec<_> = node.deps.iter().cloned().collect();
deps.sort();  // No sort() - forgot 'mut'
```

### Error Propagation

```rust
// Good
fn process() -> Result<(), TopCatError> {
    let content = fs::read_to_string(path)?;
    Ok(())
}

// Avoid
fn process() -> Result<(), TopCatError> {
    let content = fs::read_to_string(path).unwrap();  // Don't unwrap
    Ok(())
}
```
