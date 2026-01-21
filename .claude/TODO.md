Plan: Fix Remaining Type Checker Errors (109 → 0)                                                                                                                C

Goal

Run cargo run -- run examples/simple.naml with zero type errors.
                                                                                                                                                                                                        
---                                                                                                                                                                                                    
Error Categories (109 total)                                                                                                                                                                           
┌────────────────────────────────────┬───────┬────────────────────────────────────────────────┐                                                                                                        
│              Category              │ Count │                   Root Cause                   │                                                                                                        
├────────────────────────────────────┼───────┼────────────────────────────────────────────────┤                                                                                                        
│ Array methods/fields missing       │ ~15   │ No built-in .push(), .length for [T]           │                                                                                                        
├────────────────────────────────────┼───────┼────────────────────────────────────────────────┤                                                                                                        
│ Generic struct literal typing      │ ~10   │ Type params not substituted in struct literals │                                                                                                        
├────────────────────────────────────┼───────┼────────────────────────────────────────────────┤                                                                                                        
│ Option field access without unwrap │ ~25   │ Code bugs in simple.naml                       │                                                                                                        
├────────────────────────────────────┼───────┼────────────────────────────────────────────────┤                                                                                                        
│ Exception struct literals          │ ~2    │ Exceptions not handled in infer_struct_literal │                                                                                                        
├────────────────────────────────────┼───────┼────────────────────────────────────────────────┤                                                                                                        
│ Async/promise return types         │ ~15   │ Async functions not wrapped in promise<T>      │                                                                                                        
├────────────────────────────────────┼───────┼────────────────────────────────────────────────┤                                                                                                        
│ Spawn block return type            │ ~5    │ Always returns promise<unit>                   │                                                                                                        
├────────────────────────────────────┼───────┼────────────────────────────────────────────────┤                                                                                                        
│ Generic type parameter issues      │ ~35   │ Various T substitution failures                │                                                                                                        
└────────────────────────────────────┴───────┴────────────────────────────────────────────────┘
 ---                                                                                                                                                                                                    
Implementation Phases

Phase 1: Array Built-in Methods and Fields

File: namlc/src/typechecker/infer.rs

Changes:

1. In infer_method_call(), add handling for Type::Array(elem) before the type_name extraction:
- push(item: T) → Type::Unit (validates item unifies with elem)
- pop() → Type::Option(elem)
- clear() → Type::Unit
- len() → Type::Int
2. In infer_field(), add handling for Type::Array:
- length → Type::Int

 ---                                                                                                                                                                                                    
Phase 2: Exception Type Struct Literals

File: namlc/src/typechecker/infer.rs

Change: In infer_struct_literal(), handle TypeDef::Exception same as TypeDef::Struct:
- Extract exception fields
- Type-check each field value
- Return appropriate type

File: namlc/src/typechecker/types.rs

Optional: Add Type::Exception(ExceptionType) variant for semantic clarity.
                                                                                                                                                                                                        
---                                                                                                                                                                                                    
Phase 3: Async Function Promise Wrapping

File: namlc/src/typechecker/mod.rs

Change: In collect_function(), if func.is_async, wrap return type in Type::Promise:                                                                                                                    
let return_ty = if func.is_async {                                                                                                                                                                     
Type::Promise(Box::new(return_ty))                                                                                                                                                                 
} else {                                                                                                                                                                                               
return_ty                                                                                                                                                                                          
};
                                                                                                                                                                                                        
---                                                                                                                                                                                                    
Phase 4: Spawn Block Return Type Inference

File: namlc/src/typechecker/infer.rs

Change: In infer_spawn(), infer the block's actual return type:                                                                                                                                        
fn infer_spawn(&mut self, spawn: &ast::SpawnExpr) -> Type {                                                                                                                                            
let body_ty = self.infer_block(&spawn.body);                                                                                                                                                       
Type::Promise(Box::new(body_ty))                                                                                                                                                                   
}
                                                                                                                                                                                                        
---                                                                                                                                                                                                    
Phase 5: Generic Type Parameter Context

Files:
- namlc/src/typechecker/infer.rs
- namlc/src/typechecker/mod.rs
- namlc/src/typechecker/env.rs

Changes:

1. Add type parameter tracking to TypeEnv:                                                                                                                                                             
   type_params: HashMap<Spur, Type>  // Maps T -> concrete type or type var
2. In check_function(), register function type parameters in env before checking body
3. In infer_struct_literal(), when creating struct types:
- Look up type args from annotation or infer from field values
- Apply substitution to field types using Type::substitute()

 ---                                                                                                                                                                                                    
Phase 6: Fix Code Issues in simple.naml

File: examples/simple.naml

Some errors are actual code bugs, not type checker issues:
- Option field access without .unwrap() (lines 280, 289, 292, etc.)
- Assigning option<T> to T without unwrapping

Fix: Add .unwrap() calls where needed, or use proper pattern matching.
                                                                                                                                                                                                        
---                                                                                                                                                                                                    
Implementation Order                                                                                                                                                                                   
┌──────────┬───────────────────────────┬────────────┬────────────┐                                                                                                                                     
│ Priority │           Phase           │ Complexity │   Impact   │                                                                                                                                     
├──────────┼───────────────────────────┼────────────┼────────────┤                                                                                                                                     
│ 1        │ Phase 1 (Array methods)   │ Low        │ ~15 errors │                                                                                                                                     
├──────────┼───────────────────────────┼────────────┼────────────┤                                                                                                                                     
│ 2        │ Phase 2 (Exceptions)      │ Low        │ ~2 errors  │                                                                                                                                     
├──────────┼───────────────────────────┼────────────┼────────────┤                                                                                                                                     
│ 3        │ Phase 3 (Async wrap)      │ Low        │ ~10 errors │                                                                                                                                     
├──────────┼───────────────────────────┼────────────┼────────────┤                                                                                                                                     
│ 4        │ Phase 4 (Spawn return)    │ Low        │ ~5 errors  │                                                                                                                                     
├──────────┼───────────────────────────┼────────────┼────────────┤                                                                                                                                     
│ 5        │ Phase 6 (Fix simple.naml) │ Medium     │ ~25 errors │                                                                                                                                     
├──────────┼───────────────────────────┼────────────┼────────────┤                                                                                                                                     
│ 6        │ Phase 5 (Generic context) │ High       │ ~50 errors │                                                                                                                                     
└──────────┴───────────────────────────┴────────────┴────────────┘
 ---                                                                                                                                                                                                    
Files to Modify                                                                                                                                                                                        
┌────────────────────────────────┬────────────┐                                                                                                                                                        
│              File              │   Phases   │                                                                                                                                                        
├────────────────────────────────┼────────────┤                                                                                                                                                        
│ namlc/src/typechecker/infer.rs │ 1, 2, 4, 5 │                                                                                                                                                        
├────────────────────────────────┼────────────┤                                                                                                                                                        
│ namlc/src/typechecker/mod.rs   │ 3, 5       │                                                                                                                                                        
├────────────────────────────────┼────────────┤                                                                                                                                                        
│ namlc/src/typechecker/env.rs   │ 5          │                                                                                                                                                        
├────────────────────────────────┼────────────┤                                                                                                                                                        
│ examples/simple.naml           │ 6          │                                                                                                                                                        
└────────────────────────────────┴────────────┘
 ---                                                                                                                                                                                                    
Verification

After each phase:                                                                                                                                                                                      
cargo run -- run examples/simple.naml 2>&1 | grep -c "^  ×"

Final success:                                                                                                                                                                                         
cargo run -- run examples/simple.naml
# Should complete with no type errors
                                                                                                                                                                                                        
---                                                                                                                                                                                                    
Risk Assessment

- Phase 1-4: Low risk, isolated changes
- Phase 5: High risk, may require significant refactoring of type inference
- Phase 6: May need to significantly simplify generic code in simple.naml

Alternative Approach

If Phase 5 proves too complex, we can:
1. Simplify examples/simple.naml to avoid complex generics
2. Mark generic type inference as a known limitation
3. Focus on getting basic programs to work first