///
/// Type Environment - Scope Management
///
/// This module manages the type environment during type checking. It tracks:
///
/// - Variable bindings and their types in nested scopes
/// - Whether variables are mutable
/// - The current function's return type (for return statement checking)
/// - Loop nesting (for break/continue validation)
/// - Async context (for await validation)
///
/// Scopes are managed as a stack, pushed when entering blocks and popped
/// when leaving them.
///

use std::collections::HashMap;

use lasso::Spur;

use super::types::{Type, TypeParam};

#[derive(Debug, Clone)]
pub struct TypeParamBinding {
    pub bounds: Vec<Type>,
    pub concrete: Option<Type>,
}

#[derive(Debug, Clone)]
pub struct Binding {
    pub ty: Type,
    pub mutable: bool,
    pub initialized: bool,
}

impl Binding {
    pub fn new(ty: Type, mutable: bool) -> Self {
        Self {
            ty,
            mutable,
            initialized: true,
        }
    }

    pub fn uninitialized(ty: Type, mutable: bool) -> Self {
        Self {
            ty,
            mutable,
            initialized: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Scope {
    bindings: HashMap<Spur, Binding>,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    pub fn define(&mut self, name: Spur, binding: Binding) {
        self.bindings.insert(name, binding);
    }

    pub fn get(&self, name: Spur) -> Option<&Binding> {
        self.bindings.get(&name)
    }

    pub fn get_mut(&mut self, name: Spur) -> Option<&mut Binding> {
        self.bindings.get_mut(&name)
    }
}

impl Default for Scope {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct FunctionContext {
    pub return_ty: Type,
    pub throws: Option<Type>,
    pub is_async: bool,
    pub type_params: HashMap<Spur, TypeParamBinding>,
}

#[derive(Debug)]
pub struct TypeEnv {
    scopes: Vec<Scope>,
    loop_depth: usize,
    function_stack: Vec<FunctionContext>,
}

impl TypeEnv {
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope::new()],
            loop_depth: 0,
            function_stack: Vec::new(),
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn define(&mut self, name: Spur, ty: Type, mutable: bool) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.define(name, Binding::new(ty, mutable));
        }
    }

    pub fn define_uninitialized(&mut self, name: Spur, ty: Type, mutable: bool) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.define(name, Binding::uninitialized(ty, mutable));
        }
    }

    pub fn lookup(&self, name: Spur) -> Option<&Binding> {
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.get(name) {
                return Some(binding);
            }
        }
        None
    }

    pub fn lookup_mut(&mut self, name: Spur) -> Option<&mut Binding> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(binding) = scope.get_mut(name) {
                return Some(binding);
            }
        }
        None
    }

    pub fn is_defined_in_current_scope(&self, name: Spur) -> bool {
        self.scopes
            .last()
            .map_or(false, |scope| scope.get(name).is_some())
    }

    pub fn enter_loop(&mut self) {
        self.loop_depth += 1;
    }

    pub fn exit_loop(&mut self) {
        if self.loop_depth > 0 {
            self.loop_depth -= 1;
        }
    }

    pub fn in_loop(&self) -> bool {
        self.loop_depth > 0
    }

    pub fn enter_function(
        &mut self,
        return_ty: Type,
        throws: Option<Type>,
        is_async: bool,
        type_params: &[TypeParam],
    ) {
        let type_param_map = type_params
            .iter()
            .map(|tp| {
                (
                    tp.name,
                    TypeParamBinding {
                        bounds: tp.bounds.clone(),
                        concrete: None,
                    },
                )
            })
            .collect();

        self.function_stack.push(FunctionContext {
            return_ty,
            throws,
            is_async,
            type_params: type_param_map,
        });
    }

    pub fn exit_function(&mut self) {
        self.function_stack.pop();
    }

    pub fn get_type_param_bounds(&self, name: Spur) -> Option<&Vec<Type>> {
        self.function_stack
            .last()
            .and_then(|f| f.type_params.get(&name))
            .map(|b| &b.bounds)
    }

    pub fn bind_type_param(&mut self, name: Spur, concrete: Type) {
        if let Some(func) = self.function_stack.last_mut() {
            if let Some(binding) = func.type_params.get_mut(&name) {
                binding.concrete = Some(concrete);
            }
        }
    }

    pub fn get_type_param_binding(&self, name: Spur) -> Option<&Type> {
        self.function_stack
            .last()
            .and_then(|f| f.type_params.get(&name))
            .and_then(|b| b.concrete.as_ref())
    }

    pub fn current_function(&self) -> Option<&FunctionContext> {
        self.function_stack.last()
    }

    pub fn expected_return_type(&self) -> Option<&Type> {
        self.function_stack.last().map(|f| &f.return_ty)
    }

    pub fn is_async(&self) -> bool {
        self.function_stack.last().map_or(false, |f| f.is_async)
    }
}

impl Default for TypeEnv {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lasso::Rodeo;

    #[test]
    fn test_scope_define_lookup() {
        let mut rodeo = Rodeo::default();
        let x = rodeo.get_or_intern("x");

        let mut env = TypeEnv::new();
        env.define(x, Type::Int, false);

        let binding = env.lookup(x).unwrap();
        assert_eq!(binding.ty, Type::Int);
        assert!(!binding.mutable);
    }

    #[test]
    fn test_nested_scopes() {
        let mut rodeo = Rodeo::default();
        let x = rodeo.get_or_intern("x");
        let y = rodeo.get_or_intern("y");

        let mut env = TypeEnv::new();
        env.define(x, Type::Int, false);

        env.push_scope();
        env.define(y, Type::String, true);

        assert!(env.lookup(x).is_some());
        assert!(env.lookup(y).is_some());

        env.pop_scope();
        assert!(env.lookup(x).is_some());
        assert!(env.lookup(y).is_none());
    }

    #[test]
    fn test_shadowing() {
        let mut rodeo = Rodeo::default();
        let x = rodeo.get_or_intern("x");

        let mut env = TypeEnv::new();
        env.define(x, Type::Int, false);

        env.push_scope();
        env.define(x, Type::String, true);

        let binding = env.lookup(x).unwrap();
        assert_eq!(binding.ty, Type::String);

        env.pop_scope();

        let binding = env.lookup(x).unwrap();
        assert_eq!(binding.ty, Type::Int);
    }

    #[test]
    fn test_loop_context() {
        let mut env = TypeEnv::new();
        assert!(!env.in_loop());

        env.enter_loop();
        assert!(env.in_loop());

        env.enter_loop();
        env.exit_loop();
        assert!(env.in_loop());

        env.exit_loop();
        assert!(!env.in_loop());
    }

    #[test]
    fn test_function_context() {
        let mut env = TypeEnv::new();
        assert!(!env.is_async());

        env.enter_function(Type::Int, None, true, &[]);
        assert!(env.is_async());
        assert_eq!(env.expected_return_type(), Some(&Type::Int));

        env.exit_function();
        assert!(!env.is_async());
    }
}
