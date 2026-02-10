# JIT Implementation Phases - Detailed Design

## Phase 1: Foundation Fixes

### 1.1 Remove Debug Output
**Location**: `compiler.rs` lines 139, 144, 199, 317, 589, 591

**Change**: Replace `eprintln!` with conditional logging:
```rust
if cfg!(debug_assertions) && std::env::var("NAML_DEBUG").is_ok() {
    eprintln!("[DEBUG] ...");
}
```

Or simply remove them.

### 1.2 Fix Type Inference for Inferred Variables
**Problem**: `var result = x + y` uses `NamlType::Inferred` which may not resolve correctly.

**Current code** (compiler.rs:147-150):
```rust
if ty == NamlType::Inferred {
    let inferred_ty = self.infer_expression_type(init);
    self.var_types.insert(name, inferred_ty);
}
```

**Fix**: The JIT's `infer_expression_type` needs to propagate types from binary expressions correctly:
```rust
Expression::Binary(bin) => {
    if bin.op.is_comparison() || bin.op.is_logical() {
        NamlType::Bool
    } else {
        self.infer_expression_type(bin.left)  // Propagate from left operand
    }
}
```

### 1.3 Verify `hello.naml` Execution
**Test file**:
```naml
fn main() {
    println("Hello, World!");
    var x: int = 40;
    var y: int = 2;
    var result = x + y;
    print("Result: ");
    println(result);
    if (result == 42) {
        println("The answer!");
    }
    var i: int = 0;
    while (i < 3) {
        print("i = ");
        println(i);
        i = i + 1;
    }
}
```

**Expected output**:
```
Hello, World!
Result: 42
The answer!
i = 0
i = 1
i = 2
```

---

## Phase 2: User-Defined Function Calls

### 2.1 Store Function Signatures
**Location**: `JitContext` needs access to function info during compilation

**Design**:
```rust
// In JitContext
pub struct FunctionInfo {
    func_id: FuncId,
    param_types: Vec<NamlType>,
    return_type: Option<NamlType>,
}

functions: HashMap<String, FunctionInfo>,
```

### 2.2 Modify `compile_call`
**Current** (compiler.rs:570-638): Only handles builtins.

**New logic**:
```rust
fn compile_call(&mut self, name: &str, args: &[Expression]) -> Result<Value, JitError> {
    // 1. Check built-ins first
    if let Some(value) = self.try_compile_builtin(name, args)? {
        return Ok(value);
    }

    // 2. Check user-defined functions
    if let Some(func_id) = self.lookup_user_function(name) {
        return self.compile_user_call(func_id, args);
    }

    Err(JitError::Compilation(format!("Unknown function: {}", name)))
}

fn compile_user_call(&mut self, func_id: FuncId, args: &[Expression]) -> Result<Value, JitError> {
    // Compile arguments
    let compiled_args: Vec<Value> = args.iter()
        .map(|arg| self.compile_expression(arg))
        .collect::<Result<_, _>>()?;

    // Declare function reference
    let func_ref = self.module.declare_func_in_func(func_id, self.builder.func);

    // Generate call
    let call = self.builder.ins().call(func_ref, &compiled_args);

    // Get return value (if any)
    let results = self.builder.inst_results(call);
    if results.is_empty() {
        Ok(self.builder.ins().iconst(cl_types::I64, 0))
    } else {
        Ok(results[0])
    }
}
```

### 2.3 Pass Function Registry to FunctionCompiler
```rust
pub fn compile_function(func, interner, function_registry) -> Result<(), JitError>
```

---

## Phase 3: Structs & Field Access

### 3.1 Struct Layout Calculation
```rust
struct StructLayout {
    size: usize,
    align: usize,
    fields: HashMap<String, FieldInfo>,
}

struct FieldInfo {
    offset: usize,
    naml_type: NamlType,
    cl_type: cl_types::Type,
}

fn calculate_struct_layout(struct_item: &StructItem) -> StructLayout {
    let mut offset = 0;
    let mut max_align = 1;
    let mut fields = HashMap::new();

    for field in &struct_item.fields {
        let (size, align) = type_size_align(&field.ty);

        // Align offset
        offset = (offset + align - 1) & !(align - 1);

        fields.insert(field.name.clone(), FieldInfo {
            offset,
            naml_type: field.ty.clone(),
            cl_type: naml_type_to_cranelift(&field.ty),
        });

        offset += size;
        max_align = max_align.max(align);
    }

    StructLayout {
        size: (offset + max_align - 1) & !(max_align - 1),
        align: max_align,
        fields,
    }
}
```

### 3.2 Struct Literal Compilation
```naml
UserId { value: "user-123" }
```

Compiles to:
1. Allocate struct-sized memory
2. Write each field at its offset
3. Return pointer

```rust
fn compile_struct_literal(&mut self, lit: &StructLiteralExpr) -> Result<Value, JitError> {
    let layout = self.get_struct_layout(&lit.name)?;

    // Allocate
    let size = self.builder.ins().iconst(cl_types::I64, layout.size as i64);
    let ptr = self.call_runtime_alloc(size, layout.align)?;

    // Initialize fields
    for (field_name, field_value) in &lit.fields {
        let field_info = layout.fields.get(field_name)?;
        let value = self.compile_expression(field_value)?;
        let offset = self.builder.ins().iconst(self.pointer_type, field_info.offset as i64);
        let field_ptr = self.builder.ins().iadd(ptr, offset);
        self.builder.ins().store(MemFlags::new(), value, field_ptr, 0);
    }

    Ok(ptr)
}
```

### 3.3 Field Access Compilation
```naml
self.value
```

Compiles to:
1. Get struct pointer
2. Calculate field offset
3. Load from offset

```rust
fn compile_field_access(&mut self, expr: &FieldExpr) -> Result<Value, JitError> {
    let struct_ptr = self.compile_expression(expr.base)?;
    let struct_type = self.infer_expression_type(expr.base);
    let layout = self.get_struct_layout(&struct_type)?;
    let field_info = layout.fields.get(&expr.field)?;

    let offset = self.builder.ins().iconst(self.pointer_type, field_info.offset as i64);
    let field_ptr = self.builder.ins().iadd(struct_ptr, offset);

    Ok(self.builder.ins().load(field_info.cl_type, MemFlags::new(), field_ptr, 0))
}
```

---

## Phase 4: Methods

### 4.1 Method Resolution
Methods are functions with a receiver. The receiver becomes the first parameter.

```naml
pub fn (self: UserId) compare(other: UserId) -> int
```

Is equivalent to:
```naml
fn UserId_compare(self: UserId, other: UserId) -> int
```

### 4.2 Method Call Compilation
```naml
user_id.compare(other_id)
```

Compiles to:
```rust
fn compile_method_call(&mut self, call: &MethodCallExpr) -> Result<Value, JitError> {
    // 1. Get receiver value
    let receiver = self.compile_expression(call.receiver)?;

    // 2. Get receiver type to find method
    let receiver_type = self.infer_expression_type(call.receiver);

    // 3. Look up mangled method name
    let method_name = format!("{}_{}", receiver_type.name(), call.method);

    // 4. Compile as function call with receiver as first arg
    let mut args = vec![receiver];
    for arg in &call.args {
        args.push(self.compile_expression(arg)?);
    }

    self.compile_user_call_by_name(&method_name, &args)
}
```

---

## Phase 5: Arrays & Indexing

### 5.1 Array Layout
```
┌──────────┬─────────────────────────────┐
│ len: i64 │ elements[0..len]            │
└──────────┴─────────────────────────────┘
```

### 5.2 Array Literal
```naml
[1, 2, 3, 4, 5]
```

```rust
fn compile_array_literal(&mut self, elements: &[Expression]) -> Result<Value, JitError> {
    let elem_type = self.infer_element_type(elements)?;
    let elem_size = type_size(&elem_type);
    let len = elements.len();
    let total_size = 8 + (len * elem_size);

    // Allocate
    let ptr = self.call_runtime_alloc(total_size, 8)?;

    // Store length
    let len_val = self.builder.ins().iconst(cl_types::I64, len as i64);
    self.builder.ins().store(MemFlags::new(), len_val, ptr, 0);

    // Store elements
    for (i, elem) in elements.iter().enumerate() {
        let value = self.compile_expression(elem)?;
        let offset = 8 + (i * elem_size);
        let elem_ptr = self.builder.ins().iadd(ptr,
            self.builder.ins().iconst(self.pointer_type, offset as i64));
        self.builder.ins().store(MemFlags::new(), value, elem_ptr, 0);
    }

    Ok(ptr)
}
```

### 5.3 Index Access
```naml
arr[i]
```

```rust
fn compile_index_access(&mut self, expr: &IndexExpr) -> Result<Value, JitError> {
    let array_ptr = self.compile_expression(expr.base)?;
    let index = self.compile_expression(expr.index)?;

    // Optional: bounds checking
    // let len = self.builder.ins().load(cl_types::I64, MemFlags::new(), array_ptr, 0);
    // self.generate_bounds_check(index, len)?;

    let elem_size = self.get_element_size(expr.base)?;
    let offset = self.builder.ins().imul_imm(index, elem_size as i64);
    let offset = self.builder.ins().iadd_imm(offset, 8); // Skip length
    let elem_ptr = self.builder.ins().iadd(array_ptr, offset);

    Ok(self.builder.ins().load(elem_type, MemFlags::new(), elem_ptr, 0))
}
```

---

## Phase 6: Control Flow Extensions

### 6.1 For-Range Loop
```naml
for (i in 0..10) { body }
```

Compiles to:
```
init_block:
    i = 0
    limit = 10
    jump header

header:
    cond = i < limit
    brif(cond, body, exit)

body:
    ... body statements ...
    i = i + 1
    jump header

exit:
    continue...
```

### 6.2 For-Iterator Loop
```naml
for (item in array) { body }
```

Compiles to:
```
init_block:
    arr_ptr = array
    arr_len = load arr_ptr[0]
    i = 0
    jump header

header:
    cond = i < arr_len
    brif(cond, body, exit)

body:
    item = load arr_ptr[8 + i * elem_size]
    ... body statements ...
    i = i + 1
    jump header

exit:
    continue...
```

### 6.3 Break/Continue
Track loop blocks in compiler state:
```rust
struct LoopContext {
    header_block: Block,
    exit_block: Block,
}

loop_stack: Vec<LoopContext>
```

Break: `jump(current_loop.exit_block)`
Continue: `jump(current_loop.header_block)`

### 6.4 Switch Statement
```naml
switch (status) {
    case Active: { return true; }
    case Inactive: { return false; }
    default: { return false; }
}
```

For simple cases (no patterns), use `br_table` or chain of `brif`:
```rust
fn compile_switch(&mut self, switch_stmt: &SwitchStmt) -> Result<(), JitError> {
    let value = self.compile_expression(&switch_stmt.value)?;
    let exit_block = self.builder.create_block();

    // Chain of comparisons
    for case in &switch_stmt.cases {
        let case_block = self.builder.create_block();
        let next_block = self.builder.create_block();

        let case_val = self.compile_pattern_value(&case.pattern)?;
        let cond = self.builder.ins().icmp(IntCC::Equal, value, case_val);
        self.builder.ins().brif(cond, case_block, &[], next_block, &[]);

        self.builder.switch_to_block(case_block);
        self.compile_block(&case.body)?;
        if !self.block_terminated {
            self.builder.ins().jump(exit_block, &[]);
        }

        self.builder.switch_to_block(next_block);
    }

    // Default case
    if let Some(default) = &switch_stmt.default {
        self.compile_block(default)?;
    }
    if !self.block_terminated {
        self.builder.ins().jump(exit_block, &[]);
    }

    self.builder.switch_to_block(exit_block);
    Ok(())
}
```

---

## Phase 7: Enums

### 7.1 Enum Layout
```rust
struct EnumLayout {
    tag_size: usize,      // Usually 1 byte
    payload_size: usize,  // Max variant size
    total_size: usize,
    variants: HashMap<String, VariantInfo>,
}

struct VariantInfo {
    tag: u8,
    payload_layout: Option<StructLayout>,
}
```

### 7.2 Enum Variant Construction
```naml
UserStatus.Active           // No payload
UserStatus.Suspended(msg)   // With payload
```

```rust
fn compile_enum_variant(&mut self, variant: &PathExpr) -> Result<Value, JitError> {
    let layout = self.get_enum_layout(&variant.enum_name)?;
    let variant_info = layout.variants.get(&variant.variant_name)?;

    // Allocate
    let ptr = self.call_runtime_alloc(layout.total_size, 8)?;

    // Store tag
    let tag_val = self.builder.ins().iconst(cl_types::I8, variant_info.tag as i64);
    self.builder.ins().store(MemFlags::new(), tag_val, ptr, 0);

    // Store payload (if any)
    if let Some(payload) = &variant.payload {
        let value = self.compile_expression(payload)?;
        let payload_ptr = self.builder.ins().iadd_imm(ptr, layout.tag_size as i64);
        self.builder.ins().store(MemFlags::new(), value, payload_ptr, 0);
    }

    Ok(ptr)
}
```

### 7.3 Enum Pattern Matching
Use switch on tag, then access payload.

---

## Phase 8: Lambdas

### 8.1 Lambda Without Captures
```naml
|n: int| n * 2
```

Compile as anonymous function, return function pointer.

### 8.2 Lambda With Captures (Closures)
```naml
var multiplier: int = 2;
var double = |n: int| n * multiplier;
```

Closure representation:
```
┌──────────────┬────────────────────┐
│ fn_ptr: *fn  │ captures: [values] │
└──────────────┴────────────────────┘
```

Closure call reads `fn_ptr` and passes `captures` as hidden first argument.

---

## Phase 9: Maps

### 9.1 Runtime HashMap
Implement simple hash table in runtime:
```rust
struct NamlMap {
    buckets: Vec<Vec<(i64, i64)>>,  // Key hash, key ptr, value ptr
    len: usize,
}
```

### 9.2 Map Operations
- `naml_map_new() -> *Map`
- `naml_map_insert(map: *Map, key: *u8, value: *u8)`
- `naml_map_get(map: *Map, key: *u8) -> *u8`
- `naml_map_len(map: *Map) -> i64`

---

### 10.3 Spawn
Create new task/coroutine, add to executor queue.

---

## Phase 11: Exceptions

### 11.1 Throw Compilation
```naml
throw NetworkError { message: "...", code: 500, retry_after: none };
```

For now, implement as panic or return error.

### 11.2 Future: Try-Catch
Would require setjmp/longjmp or state-machine transformation.

---

## Phase 12: Generics

### 12.1 Monomorphization
At compile time, instantiate separate functions for each type combination used.

```naml
fn identity<T>(x: T) -> T { return x; }

identity(42);       // -> identity$int
identity("hello");  // -> identity$string
```

### 12.2 Type Resolution
During JIT compilation, resolve generic parameters based on call site types.

### 12.3 Generic Struct Layouts
Each instantiation gets its own layout calculation.

---

## Testing Checkpoints

After each phase, verify:

| Phase | Test |
|-------|------|
| 1 | hello.naml executes cleanly |
| 2 | Multi-function programs work |
| 3 | Struct creation and field access |
| 4 | Method calls work |
| 5 | Array operations work |
| 6 | All control flow patterns |
| 7 | Enum construction and matching |
| 8 | Lambda expressions |
| 9 | Map operations |
| 10 | Async/await basics |
| 11 | Throw (as panic) |
| 12 | Generic functions and structs |

**Final test**: Full test_parse.rs naml code executes successfully.
