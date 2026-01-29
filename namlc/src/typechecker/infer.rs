//!
//! Type Inference for Expressions
//!
//! This module infers types for expressions. Each expression form has
//! specific typing rules:
//!
//! - Literals: Type is determined by literal form
//! - Identifiers: Look up in environment
//! - Binary/Unary: Check operand types, return result type
//! - Calls: Unify arguments with parameters, return result type
//! - Field access: Look up field type on struct
//! - Index: Check indexable type, return element type
//!
//! Type variables are created for unknown types and unified during
//! inference to discover concrete types.
//!

use lasso::Rodeo;

use crate::ast::{self, Expression, Literal, Pattern};
use crate::source::Spanned;

use super::env::TypeEnv;
use super::error::TypeError;
use super::symbols::{SymbolTable, TypeDef};
use super::typed_ast::{ExprTypeInfo, TypeAnnotations};
use super::types::{FunctionType, Type};
use super::unify::{fresh_type_var, unify};

pub struct TypeInferrer<'a> {
    pub env: &'a mut TypeEnv,
    pub symbols: &'a SymbolTable,
    pub interner: &'a Rodeo,
    pub next_var_id: &'a mut u32,
    pub errors: &'a mut Vec<TypeError>,
    pub annotations: &'a mut TypeAnnotations,
    pub switch_scrutinee: Option<Type>,
}

impl<'a> TypeInferrer<'a> {
    pub fn infer_expr(&mut self, expr: &Expression) -> Type {
        let ty = match expr {
            Expression::Literal(lit) => self.infer_literal(lit),
            Expression::Identifier(ident) => self.infer_identifier(ident),
            Expression::Path(path) => self.infer_path(path),
            Expression::Binary(bin) => self.infer_binary(bin),
            Expression::Unary(un) => self.infer_unary(un),
            Expression::Call(call) => self.infer_call(call),
            Expression::MethodCall(call) => self.infer_method_call(call),
            Expression::Index(idx) => self.infer_index(idx),
            Expression::Field(field) => self.infer_field(field),
            Expression::Array(arr) => self.infer_array(arr),
            Expression::Map(map) => self.infer_map(map),
            Expression::StructLiteral(lit) => self.infer_struct_literal(lit),
            Expression::If(if_expr) => self.infer_if(if_expr),
            Expression::Block(block) => self.infer_block(block),
            Expression::Lambda(lambda) => self.infer_lambda(lambda),
            Expression::Spawn(spawn) => self.infer_spawn(spawn),
            Expression::Try(try_expr) => self.infer_try(try_expr),
            Expression::Catch(catch) => self.infer_catch(catch),
            Expression::Cast(cast) => self.infer_cast(cast),
            Expression::FallibleCast(cast) => self.infer_fallible_cast(cast),
            Expression::Range(range) => self.infer_range(range),
            Expression::Grouped(grouped) => self.infer_expr(grouped.inner),
            Expression::Some(some) => self.infer_some(some),
            Expression::Ternary(ternary) => self.infer_ternary(ternary),
            Expression::Elvis(elvis) => self.infer_elvis(elvis),
        };

        let resolved_ty = ty.resolve();
        let is_lvalue = self.is_lvalue(expr);
        let needs_clone = self.compute_needs_clone(expr, &resolved_ty);
        self.annotations.annotate(
            expr.span(),
            ExprTypeInfo::new(resolved_ty.clone())
                .with_lvalue(is_lvalue)
                .with_clone(needs_clone),
        );

        ty
    }

    fn is_lvalue(&self, expr: &Expression) -> bool {
        matches!(
            expr,
            Expression::Identifier(_) | Expression::Field(_) | Expression::Index(_)
        )
    }

    fn compute_needs_clone(&self, expr: &Expression, ty: &Type) -> bool {
        if self.is_copy_type(ty) {
            return false;
        }
        if self.env.current_function().is_some() && self.involves_self(expr) {
            return true;
        }
        false
    }

    fn is_copy_type(&self, ty: &Type) -> bool {
        matches!(
            ty,
            Type::Int | Type::Uint | Type::Float | Type::Bool | Type::Unit
        )
    }

    fn involves_self(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier(ident) => {
                let name = self.interner.resolve(&ident.ident.symbol);
                name == "self"
            }
            Expression::Field(field) => self.involves_self(field.base),
            Expression::Index(idx) => self.involves_self(idx.base),
            _ => false,
        }
    }

    fn mangle_generic_function(&self, func_name: &str, type_args: &[Type]) -> String {
        let mut mangled = func_name.to_string();
        for ty in type_args {
            mangled.push('_');
            mangled.push_str(&self.mangle_type(ty));
        }
        mangled
    }

    fn mangle_type(&self, ty: &Type) -> String {
        match ty {
            Type::Int => "int".to_string(),
            Type::Uint => "uint".to_string(),
            Type::Float => "float".to_string(),
            Type::Bool => "bool".to_string(),
            Type::String => "string".to_string(),
            Type::Bytes => "bytes".to_string(),
            Type::Unit => "unit".to_string(),
            Type::Array(inner) => format!("Array_{}", self.mangle_type(inner)),
            Type::FixedArray(inner, size) => format!("FixedArray_{}_{}", self.mangle_type(inner), size),
            Type::Option(inner) => format!("Option_{}", self.mangle_type(inner)),
            Type::Map(k, v) => format!("Map_{}_{}", self.mangle_type(k), self.mangle_type(v)),
            Type::Channel(inner) => format!("Channel_{}", self.mangle_type(inner)),
            Type::Struct(s) => self.interner.resolve(&s.name).to_string(),
            Type::Enum(e) => self.interner.resolve(&e.name).to_string(),
            Type::Interface(i) => self.interner.resolve(&i.name).to_string(),
            Type::Exception(name) => self.interner.resolve(name).to_string(),
            Type::Function(_) => "fn".to_string(),
            Type::TypeVar(tv) => format!("T{}", tv.id),
            Type::Generic(name, args) => {
                let mut s = self.interner.resolve(name).to_string();
                for arg in args {
                    s.push('_');
                    s.push_str(&self.mangle_type(arg));
                }
                s
            }
            Type::Error => "error".to_string(),
            Type::Never => "never".to_string(),
        }
    }

    fn infer_some(&mut self, some: &ast::SomeExpr) -> Type {
        let inner_ty = self.infer_expr(some.value);
        Type::Option(Box::new(inner_ty))
    }

    fn infer_ternary(&mut self, ternary: &ast::TernaryExpr) -> Type {
        let cond_ty = self.infer_expr(ternary.condition);
        if let Err(e) = unify(&cond_ty, &Type::Bool, ternary.condition.span()) {
            self.errors.push(e);
        }

        let true_ty = self.infer_expr(ternary.true_expr);
        let false_ty = self.infer_expr(ternary.false_expr);

        if let Err(e) = unify(&true_ty, &false_ty, ternary.span) {
            self.errors.push(e);
            return Type::Error;
        }

        true_ty.resolve()
    }

    fn infer_elvis(&mut self, elvis: &ast::ElvisExpr) -> Type {
        let left_ty = self.infer_expr(elvis.left);
        let right_ty = self.infer_expr(elvis.right);
        let left_resolved = left_ty.resolve();

        match &left_resolved {
            Type::Option(inner) => {
                if let Err(e) = unify(inner, &right_ty, elvis.span) {
                    self.errors.push(e);
                }
                right_ty.resolve()
            }
            _ => {
                if let Err(e) = unify(&left_ty, &right_ty, elvis.span) {
                    self.errors.push(e);
                }
                right_ty.resolve()
            }
        }
    }

    fn infer_literal(&mut self, lit: &ast::LiteralExpr) -> Type {
        match &lit.value {
            Literal::Int(_) => Type::Int,
            Literal::UInt(_) => Type::Uint,
            Literal::Float(_) => Type::Float,
            Literal::Bool(_) => Type::Bool,
            Literal::String(_) => Type::String,
            Literal::Bytes(_) => Type::Bytes,
            Literal::None => {
                let inner = fresh_type_var(self.next_var_id);
                Type::Option(Box::new(inner))
            }
        }
    }

    fn infer_identifier(&mut self, ident: &ast::IdentExpr) -> Type {
        if let Some(binding) = self.env.lookup(ident.ident.symbol) {
            binding.ty.clone()
        } else if let Some(func) = self.symbols.get_function(ident.ident.symbol) {
            Type::Function(self.symbols.to_function_type(func))
        } else if let Some(def) = self.symbols.get_type(ident.ident.symbol) {
            // Handle enum type names being used directly (e.g., UserRole in UserRole.Admin)
            use super::symbols::TypeDef;
            match def {
                TypeDef::Enum(e) => Type::Enum(self.symbols.to_enum_type(e)),
                _ => {
                    let name = self.interner.resolve(&ident.ident.symbol).to_string();
                    self.errors
                        .push(TypeError::undefined_var(name, ident.span));
                    Type::Error
                }
            }
        } else if let Some(Type::Enum(ref e)) = self.switch_scrutinee {
            // In a switch context, try to resolve bare identifier as enum variant
            for variant in &e.variants {
                if variant.name == ident.ident.symbol {
                    return Type::Enum(e.clone());
                }
            }
            let name = self.interner.resolve(&ident.ident.symbol).to_string();
            self.errors
                .push(TypeError::undefined_var(name, ident.span));
            Type::Error
        } else {
            let name = self.interner.resolve(&ident.ident.symbol).to_string();
            self.errors
                .push(TypeError::undefined_var(name, ident.span));
            Type::Error
        }
    }

    fn infer_path(&mut self, path: &ast::PathExpr) -> Type {
        if path.segments.is_empty() {
            return Type::Error;
        }

        let first = &path.segments[0];
        if let Some(def) = self.symbols.get_type(first.symbol) {
            use super::symbols::TypeDef;
            match def {
                TypeDef::Enum(e) => {
                    let enum_ty = self.symbols.to_enum_type(e);
                    if path.segments.len() == 2 {
                        let variant_name = path.segments[1].symbol;
                        for (name, fields) in &e.variants {
                            if *name == variant_name {
                                // If variant has associated data, return function type
                                if let Some(field_types) = fields {
                                    return Type::Function(FunctionType {
                                        params: field_types.clone(),
                                        returns: Box::new(Type::Enum(enum_ty)),
                                        throws: vec![],
                                        is_variadic: false,
                                    });
                                }
                                return Type::Enum(enum_ty);
                            }
                        }
                        let variant = self.interner.resolve(&variant_name).to_string();
                        let enum_name = self.interner.resolve(&first.symbol).to_string();
                        self.errors.push(TypeError::Custom {
                            message: format!("unknown variant {} for enum {}", variant, enum_name),
                            span: path.span,
                        });
                    }
                    Type::Enum(enum_ty)
                }
                _ => {
                    Type::Generic(first.symbol, Vec::new())
                }
            }
        } else {
            Type::Generic(first.symbol, Vec::new())
        }
    }

    fn infer_binary(&mut self, bin: &ast::BinaryExpr) -> Type {
        use ast::BinaryOp::*;

        // Handle `is` operator specially - RHS is a type name, not an expression
        if bin.op == Is {
            let _left_ty = self.infer_expr(bin.left);
            if let ast::Expression::Identifier(ident) = &bin.right
                && self.symbols.get_type(ident.ident.symbol).is_some() {
                    return Type::Bool;
                }
            self.errors.push(TypeError::Custom {
                message: "'is' operator requires a type name on the right side".to_string(),
                span: bin.right.span(),
            });
            return Type::Bool;
        }

        let left_ty = self.infer_expr(bin.left);
        let right_ty = self.infer_expr(bin.right);

        match bin.op {
            Add => {
                let left_resolved = left_ty.resolve();
                let right_resolved = right_ty.resolve();

                match (&left_resolved, &right_resolved) {
                    (Type::String, Type::String) => Type::String,
                    _ if left_resolved.is_numeric() || right_resolved.is_numeric() => {
                        // Handle int/uint coercion for Add as well
                        let coerced = self.coerce_int_uint(&left_resolved, &right_resolved, bin.left, bin.right);
                        if let Some(result_ty) = coerced {
                            return result_ty;
                        }

                        if let Err(e) = unify(&left_ty, &right_ty, bin.span) {
                            self.errors.push(e);
                            return Type::Error;
                        }
                        let resolved = left_ty.resolve();
                        if !resolved.is_numeric() && resolved != Type::Error {
                            self.errors.push(TypeError::InvalidBinaryOp {
                                op: format!("{:?}", bin.op),
                                left: left_ty.to_string(),
                                right: right_ty.to_string(),
                                span: bin.span,
                            });
                            return Type::Error;
                        }
                        resolved
                    }
                    _ => {
                        self.errors.push(TypeError::InvalidBinaryOp {
                            op: format!("{:?}", bin.op),
                            left: left_ty.to_string(),
                            right: right_ty.to_string(),
                            span: bin.span,
                        });
                        Type::Error
                    }
                }
            }

            Sub | Mul | Div | Mod => {
                let left_resolved = left_ty.resolve();
                let right_resolved = right_ty.resolve();

                // Handle int/uint coercion: if one is uint and other is int, prefer uint
                let coerced = self.coerce_int_uint(&left_resolved, &right_resolved, bin.left, bin.right);
                if let Some(result_ty) = coerced {
                    return result_ty;
                }

                if let Err(e) = unify(&left_ty, &right_ty, bin.span) {
                    self.errors.push(e);
                    return Type::Error;
                }
                let resolved = left_ty.resolve();
                if !resolved.is_numeric() && resolved != Type::Error {
                    self.errors.push(TypeError::InvalidBinaryOp {
                        op: format!("{:?}", bin.op),
                        left: left_ty.to_string(),
                        right: right_ty.to_string(),
                        span: bin.span,
                    });
                    return Type::Error;
                }
                resolved
            }

            Eq | NotEq => {
                if let Err(e) = unify(&left_ty, &right_ty, bin.span) {
                    self.errors.push(e);
                }
                Type::Bool
            }

            Lt | LtEq | Gt | GtEq => {
                if let Err(e) = unify(&left_ty, &right_ty, bin.span) {
                    self.errors.push(e);
                    return Type::Bool;
                }
                let resolved = left_ty.resolve();
                if !resolved.is_comparable() && resolved != Type::Error {
                    self.errors.push(TypeError::InvalidBinaryOp {
                        op: format!("{:?}", bin.op),
                        left: left_ty.to_string(),
                        right: right_ty.to_string(),
                        span: bin.span,
                    });
                }
                Type::Bool
            }

            And | Or => {
                if let Err(e) = unify(&left_ty, &Type::Bool, bin.span) {
                    self.errors.push(e);
                }
                if let Err(e) = unify(&right_ty, &Type::Bool, bin.span) {
                    self.errors.push(e);
                }
                Type::Bool
            }

            BitAnd | BitOr | BitXor | Shl | Shr => {
                if let Err(e) = unify(&left_ty, &right_ty, bin.span) {
                    self.errors.push(e);
                    return Type::Error;
                }
                let resolved = left_ty.resolve();
                if !resolved.is_integer() && resolved != Type::Error {
                    self.errors.push(TypeError::InvalidBinaryOp {
                        op: format!("{:?}", bin.op),
                        left: left_ty.to_string(),
                        right: right_ty.to_string(),
                        span: bin.span,
                    });
                    return Type::Error;
                }
                resolved
            }

            Range | RangeIncl => {
                if let Err(e) = unify(&left_ty, &right_ty, bin.span) {
                    self.errors.push(e);
                }
                Type::Array(Box::new(left_ty.resolve()))
            }

            Is => Type::Bool,

            NullCoalesce => {
                let left_resolved = left_ty.resolve();
                match &left_resolved {
                    Type::Option(inner) => {
                        if let Err(e) = unify(inner, &right_ty, bin.span) {
                            self.errors.push(e);
                        }
                        right_ty.resolve()
                    }
                    _ => {
                        if let Err(e) = unify(&left_ty, &right_ty, bin.span) {
                            self.errors.push(e);
                        }
                        right_ty.resolve()
                    }
                }
            }
        }
    }

    fn infer_unary(&mut self, un: &ast::UnaryExpr) -> Type {
        let operand_ty = self.infer_expr(un.operand);

        use ast::UnaryOp::*;
        match un.op {
            Neg => {
                let resolved = operand_ty.resolve();
                if !resolved.is_numeric() && resolved != Type::Error {
                    self.errors.push(TypeError::InvalidOperation {
                        op: "negation".into(),
                        ty: operand_ty.to_string(),
                        span: un.span,
                    });
                    return Type::Error;
                }
                resolved
            }
            Not => {
                if let Err(e) = unify(&operand_ty, &Type::Bool, un.span) {
                    self.errors.push(e);
                }
                Type::Bool
            }
            BitNot => {
                let resolved = operand_ty.resolve();
                if !resolved.is_integer() && resolved != Type::Error {
                    self.errors.push(TypeError::InvalidOperation {
                        op: "bitwise not".into(),
                        ty: operand_ty.to_string(),
                        span: un.span,
                    });
                    return Type::Error;
                }
                resolved
            }
        }
    }

    fn infer_call(&mut self, call: &ast::CallExpr) -> Type {
        // Check if callee is an identifier referring to a generic function or exception
        if let ast::Expression::Identifier(ident) = call.callee {
            // Check for exception constructor: ExceptionType("message")
            if let Some(TypeDef::Exception(exc_def)) = self.symbols.get_type(ident.ident.symbol) {
                if call.args.len() != 1 {
                    self.errors.push(TypeError::WrongArgCount {
                        expected: 1,
                        found: call.args.len(),
                        span: call.span,
                    });
                    return Type::Error;
                }
                let arg_ty = self.infer_expr(&call.args[0]);
                if let Err(e) = unify(&arg_ty, &Type::String, call.args[0].span()) {
                    self.errors.push(e);
                }
                return Type::Exception(exc_def.name);
            }

            if let Some(func_sig) = self.symbols.get_function(ident.ident.symbol)
                && !func_sig.type_params.is_empty() {
                    return self.infer_generic_call(call, func_sig);
                }
        }

        let callee_ty = self.infer_expr(call.callee);
        let resolved = callee_ty.resolve();

        match resolved {
            Type::Function(func) => {
                if func.is_variadic {
                    for (arg, param_ty) in call.args.iter().zip(func.params.iter()) {
                        let arg_ty = self.infer_expr(arg);
                        if let Err(e) = unify(&arg_ty, param_ty, arg.span()) {
                            self.errors.push(e);
                        }
                    }
                    for arg in call.args.iter().skip(func.params.len()) {
                        self.infer_expr(arg);
                    }
                } else {
                    if call.args.len() != func.params.len() {
                        self.errors.push(TypeError::WrongArgCount {
                            expected: func.params.len(),
                            found: call.args.len(),
                            span: call.span,
                        });
                        return Type::Error;
                    }

                    for (arg, param_ty) in call.args.iter().zip(func.params.iter()) {
                        let arg_ty = self.infer_expr(arg);
                        if let Err(e) = unify(&arg_ty, param_ty, arg.span()) {
                            self.errors.push(e);
                        }
                    }
                }

                (*func.returns).clone()
            }
            Type::Error => Type::Error,
            _ => {
                self.errors.push(TypeError::NotCallable {
                    ty: callee_ty.to_string(),
                    span: call.span,
                });
                Type::Error
            }
        }
    }

    fn infer_generic_call(
        &mut self,
        call: &ast::CallExpr,
        func_sig: &super::symbols::FunctionSig,
    ) -> Type {
        use super::generics::{build_substitution, instantiate_generic_function};

        // Build substitution map
        let substitution = if !call.type_args.is_empty() {
            // Explicit type arguments provided
            if call.type_args.len() != func_sig.type_params.len() {
                self.errors.push(TypeError::WrongTypeArgCount {
                    expected: func_sig.type_params.len(),
                    found: call.type_args.len(),
                    span: call.span,
                });
                return Type::Error;
            }
            let type_args: Vec<Type> = call
                .type_args
                .iter()
                .map(|t| self.convert_ast_type(t))
                .collect();
            build_substitution(&func_sig.type_params, &type_args)
        } else {
            // Infer type arguments - create fresh type vars
            let (_, subst) = instantiate_generic_function(&func_sig.type_params, self.next_var_id);
            subst
        };

        // Check argument count
        if call.args.len() != func_sig.params.len() {
            self.errors.push(TypeError::WrongArgCount {
                expected: func_sig.params.len(),
                found: call.args.len(),
                span: call.span,
            });
            return Type::Error;
        }

        // Check argument types with substitution applied
        for (arg, (_, param_ty)) in call.args.iter().zip(func_sig.params.iter()) {
            let arg_ty = self.infer_expr(arg);
            let substituted_param_ty = param_ty.substitute(&substitution);
            if let Err(e) = unify(&arg_ty, &substituted_param_ty, arg.span()) {
                self.errors.push(e);
            }
        }

        // Record monomorphization for codegen: resolve type vars to concrete types
        let resolved_type_args: Vec<Type> = func_sig
            .type_params
            .iter()
            .map(|tp| {
                substitution
                    .get(&tp.name)
                    .cloned()
                    .unwrap_or(Type::Error)
                    .resolve()
            })
            .collect();

        // Generate mangled name: func_TypeArg1_TypeArg2
        let func_name = self.interner.resolve(&func_sig.name);
        let mangled_name = self.mangle_generic_function(func_name, &resolved_type_args);
        self.annotations
            .record_monomorphization(call.span, func_sig.name, resolved_type_args, mangled_name);

        // Return substituted return type
        func_sig.return_ty.substitute(&substitution)
    }

    fn infer_method_call(&mut self, call: &ast::MethodCallExpr) -> Type {
        let receiver_ty = self.infer_expr(call.receiver);
        let resolved = receiver_ty.resolve();

        // Handle built-in option methods
        if let Type::Option(inner) = &resolved {
            let method_name = self.interner.resolve(&call.method.symbol);
            return match method_name {
                "is_some" | "is_none" => {
                    if !call.args.is_empty() {
                        self.errors.push(TypeError::WrongArgCount {
                            expected: 0,
                            found: call.args.len(),
                            span: call.span,
                        });
                    }
                    Type::Bool
                }
                _ => {
                    self.errors.push(TypeError::UndefinedMethod {
                        ty: resolved.to_string(),
                        method: method_name.to_string(),
                        span: call.span,
                    });
                    Type::Error
                }
            };
        }

        // Handle built-in array methods
        if let Type::Array(elem) = &resolved {
            let method_name = self.interner.resolve(&call.method.symbol);
            return match method_name {
                "push" => {
                    if call.args.len() != 1 {
                        self.errors.push(TypeError::WrongArgCount {
                            expected: 1,
                            found: call.args.len(),
                            span: call.span,
                        });
                        return Type::Unit;
                    }
                    let arg_ty = self.infer_expr(&call.args[0]);
                    if let Err(e) = unify(&arg_ty, elem, call.args[0].span()) {
                        self.errors.push(e);
                    }
                    Type::Unit
                }
                "pop" => {
                    if !call.args.is_empty() {
                        self.errors.push(TypeError::WrongArgCount {
                            expected: 0,
                            found: call.args.len(),
                            span: call.span,
                        });
                    }
                    Type::Option(elem.clone())
                }
                "clear" => {
                    if !call.args.is_empty() {
                        self.errors.push(TypeError::WrongArgCount {
                            expected: 0,
                            found: call.args.len(),
                            span: call.span,
                        });
                    }
                    Type::Unit
                }
                "len" => {
                    if !call.args.is_empty() {
                        self.errors.push(TypeError::WrongArgCount {
                            expected: 0,
                            found: call.args.len(),
                            span: call.span,
                        });
                    }
                    Type::Int
                }
                _ => {
                    self.errors.push(TypeError::UndefinedMethod {
                        ty: resolved.to_string(),
                        method: method_name.to_string(),
                        span: call.span,
                    });
                    Type::Error
                }
            };
        }

        // Handle built-in channel methods
        if let Type::Channel(inner) = &resolved {
            let method_name = self.interner.resolve(&call.method.symbol);
            return match method_name {
                "send" => {
                    if call.args.len() != 1 {
                        self.errors.push(TypeError::WrongArgCount {
                            expected: 1,
                            found: call.args.len(),
                            span: call.span,
                        });
                        return Type::Int;
                    }
                    let arg_ty = self.infer_expr(&call.args[0]);
                    if let Err(e) = unify(&arg_ty, inner, call.args[0].span()) {
                        self.errors.push(e);
                    }
                    Type::Int
                }
                "receive" => {
                    if !call.args.is_empty() {
                        self.errors.push(TypeError::WrongArgCount {
                            expected: 0,
                            found: call.args.len(),
                            span: call.span,
                        });
                    }
                    (**inner).clone()
                }
                "close" => {
                    if !call.args.is_empty() {
                        self.errors.push(TypeError::WrongArgCount {
                            expected: 0,
                            found: call.args.len(),
                            span: call.span,
                        });
                    }
                    Type::Unit
                }
                _ => {
                    self.errors.push(TypeError::UndefinedMethod {
                        ty: resolved.to_string(),
                        method: method_name.to_string(),
                        span: call.span,
                    });
                    Type::Error
                }
            };
        }

        // Check if receiver is a bare type parameter (T with no type args)
        // If so, look up methods from its bounds
        if let Type::Generic(param_name, type_args) = &resolved
            && type_args.is_empty() {
                // This might be a type parameter - check if it has bounds with this method
                if let Some(method_type) =
                    super::generics::find_method_from_bounds(*param_name, call.method.symbol, self.env, self.symbols)
                {
                    // Check argument count
                    if call.args.len() != method_type.params.len() {
                        self.errors.push(TypeError::WrongArgCount {
                            expected: method_type.params.len(),
                            found: call.args.len(),
                            span: call.span,
                        });
                        return Type::Error;
                    }

                    // Check argument types
                    for (arg, param_ty) in call.args.iter().zip(method_type.params.iter()) {
                        let arg_ty = self.infer_expr(arg);
                        if let Err(e) = unify(&arg_ty, param_ty, arg.span()) {
                            self.errors.push(e);
                        }
                    }

                    return method_type.returns;
                }
            }

        // Handle built-in exception methods
        if let Type::Exception(_) = &resolved {
            let method_name = self.interner.resolve(&call.method.symbol);
            return match method_name {
                "message" => {
                    if !call.args.is_empty() {
                        self.errors.push(TypeError::WrongArgCount {
                            expected: 0,
                            found: call.args.len(),
                            span: call.span,
                        });
                    }
                    Type::String
                }
                _ => {
                    self.errors.push(TypeError::UndefinedMethod {
                        ty: resolved.to_string(),
                        method: method_name.to_string(),
                        span: call.span,
                    });
                    Type::Error
                }
            };
        }

        // Handle built-in string methods
        if let Type::String = &resolved {
            let method_name = self.interner.resolve(&call.method.symbol);
            return match method_name {
                "len" => {
                    if !call.args.is_empty() {
                        self.errors.push(TypeError::WrongArgCount {
                            expected: 0,
                            found: call.args.len(),
                            span: call.span,
                        });
                    }
                    Type::Int
                }
                "char_at" => {
                    if call.args.len() != 1 {
                        self.errors.push(TypeError::WrongArgCount {
                            expected: 1,
                            found: call.args.len(),
                            span: call.span,
                        });
                        return Type::Int;
                    }
                    let arg_ty = self.infer_expr(&call.args[0]);
                    if let Err(e) = unify(&arg_ty, &Type::Int, call.args[0].span()) {
                        self.errors.push(e);
                    }
                    Type::Int
                }
                _ => {
                    self.errors.push(TypeError::UndefinedMethod {
                        ty: resolved.to_string(),
                        method: method_name.to_string(),
                        span: call.span,
                    });
                    Type::Error
                }
            };
        }

        let type_name = match &resolved {
            Type::Struct(s) => s.name,
            Type::Enum(e) => e.name,
            Type::Generic(name, _) => *name,
            Type::Error => return Type::Error,
            _ => {
                let method = self.interner.resolve(&call.method.symbol).to_string();
                self.errors.push(TypeError::UndefinedMethod {
                    ty: resolved.to_string(),
                    method,
                    span: call.span,
                });
                return Type::Error;
            }
        };

        if let Some(method) = self.symbols.get_method(type_name, call.method.symbol) {
            if call.args.len() != method.params.len() {
                self.errors.push(TypeError::WrongArgCount {
                    expected: method.params.len(),
                    found: call.args.len(),
                    span: call.span,
                });
                return Type::Error;
            }

            // Build substitution map for generic type arguments
            use std::collections::HashMap;
            let substitutions: HashMap<lasso::Spur, Type> = if let Type::Generic(_, type_args) = &resolved {
                // Look up the struct definition to get type parameter names
                if let Some(TypeDef::Struct(struct_def)) = self.symbols.get_type(type_name) {
                    struct_def.type_params.iter()
                        .zip(type_args.iter())
                        .map(|(param, arg)| (param.name, arg.clone()))
                        .collect()
                } else {
                    HashMap::new()
                }
            } else {
                HashMap::new()
            };

            for (arg, (_, param_ty)) in call.args.iter().zip(method.params.iter()) {
                let arg_ty = self.infer_expr(arg);
                // Substitute type parameters in the parameter type
                let substituted_param_ty = param_ty.substitute(&substitutions);
                if let Err(e) = unify(&arg_ty, &substituted_param_ty, arg.span()) {
                    self.errors.push(e);
                }
            }

            // Substitute type parameters in the return type
            method.return_ty.substitute(&substitutions)
        } else {
            let method = self.interner.resolve(&call.method.symbol).to_string();
            self.errors.push(TypeError::UndefinedMethod {
                ty: resolved.to_string(),
                method,
                span: call.span,
            });
            Type::Error
        }
    }

    fn infer_index(&mut self, idx: &ast::IndexExpr) -> Type {
        let base_ty = self.infer_expr(idx.base);
        let index_ty = self.infer_expr(idx.index);
        let resolved = base_ty.resolve();

        match resolved {
            Type::Array(elem) | Type::FixedArray(elem, _) => {
                if let Err(e) = unify(&index_ty, &Type::Int, idx.index.span()) {
                    self.errors.push(e);
                }
                (*elem).clone()
            }
            Type::Map(key, val) => {
                if let Err(e) = unify(&index_ty, &key, idx.index.span()) {
                    self.errors.push(e);
                }
                // Map indexing returns option<V> since key might not exist
                Type::Option(val.clone())
            }
            Type::String => {
                if let Err(e) = unify(&index_ty, &Type::Int, idx.index.span()) {
                    self.errors.push(e);
                }
                Type::String
            }
            Type::Error => Type::Error,
            _ => {
                self.errors.push(TypeError::NotIndexable {
                    ty: base_ty.to_string(),
                    span: idx.span,
                });
                Type::Error
            }
        }
    }

    fn infer_field(&mut self, field: &ast::FieldExpr) -> Type {
        let base_ty = self.infer_expr(field.base);
        let resolved = base_ty.resolve();

        match resolved {
            Type::Array(_) | Type::FixedArray(_, _) => {
                let field_name = self.interner.resolve(&field.field.symbol);
                if field_name == "length" {
                    return Type::Int;
                }
                self.errors.push(TypeError::UndefinedField {
                    ty: resolved.to_string(),
                    field: field_name.to_string(),
                    span: field.span,
                });
                Type::Error
            }
            Type::Struct(s) => {
                for f in &s.fields {
                    if f.name == field.field.symbol {
                        return f.ty.clone();
                    }
                }
                let field_name = self.interner.resolve(&field.field.symbol).to_string();
                self.errors.push(TypeError::UndefinedField {
                    ty: format!("{:?}", s.name),
                    field: field_name,
                    span: field.span,
                });
                Type::Error
            }
            Type::Enum(ref e) => {
                // Handle EnumType.Variant syntax (e.g., UserRole.Admin)
                for variant in &e.variants {
                    if variant.name == field.field.symbol {
                        return Type::Enum(e.clone());
                    }
                }
                let field_name = self.interner.resolve(&field.field.symbol).to_string();
                let enum_name = self.interner.resolve(&e.name).to_string();
                self.errors.push(TypeError::Custom {
                    message: format!("unknown variant '{}' for enum '{}'", field_name, enum_name),
                    span: field.span,
                });
                Type::Error
            }
            Type::Generic(name, ref type_args) => {
                // Look up the struct definition by name
                use super::symbols::TypeDef;
                use std::collections::HashMap;
                if let Some(TypeDef::Struct(def)) = self.symbols.get_type(name) {
                    let struct_ty = self.symbols.to_struct_type(def);
                    // Build substitution map from type params to actual type args
                    let substitution: HashMap<lasso::Spur, Type> = struct_ty
                        .type_params
                        .iter()
                        .zip(type_args.iter())
                        .map(|(tp, arg)| (tp.name, arg.clone()))
                        .collect();
                    for f in &struct_ty.fields {
                        if f.name == field.field.symbol {
                            // Apply substitution to get concrete field type
                            return f.ty.substitute(&substitution);
                        }
                    }
                    let field_name = self.interner.resolve(&field.field.symbol).to_string();
                    self.errors.push(TypeError::UndefinedField {
                        ty: format!("{:?}", name),
                        field: field_name,
                        span: field.span,
                    });
                    Type::Error
                } else {
                    let field_name = self.interner.resolve(&field.field.symbol).to_string();
                    self.errors.push(TypeError::UndefinedField {
                        ty: resolved.to_string(),
                        field: field_name,
                        span: field.span,
                    });
                    Type::Error
                }
            }
            Type::Exception(name) => {
                if let Some(TypeDef::Exception(def)) = self.symbols.get_type(name) {
                    let field_name_str = self.interner.resolve(&field.field.symbol);
                    if field_name_str == "message" {
                        return Type::String;
                    }
                    for (f_name, f_ty) in &def.fields {
                        if *f_name == field.field.symbol {
                            return f_ty.clone();
                        }
                    }
                    self.errors.push(TypeError::UndefinedField {
                        ty: resolved.to_string(),
                        field: field_name_str.to_string(),
                        span: field.span,
                    });
                    Type::Error
                } else {
                    Type::Error
                }
            }
            Type::Error => Type::Error,
            _ => {
                let field_name = self.interner.resolve(&field.field.symbol).to_string();
                self.errors.push(TypeError::UndefinedField {
                    ty: resolved.to_string(),
                    field: field_name,
                    span: field.span,
                });
                Type::Error
            }
        }
    }

    fn infer_array(&mut self, arr: &ast::ArrayExpr) -> Type {
        if arr.elements.is_empty() {
            return Type::Array(Box::new(fresh_type_var(self.next_var_id)));
        }

        let first_ty = self.infer_expr(&arr.elements[0]);
        for elem in arr.elements.iter().skip(1) {
            let elem_ty = self.infer_expr(elem);
            if let Err(e) = unify(&elem_ty, &first_ty, elem.span()) {
                self.errors.push(e);
            }
        }

        Type::Array(Box::new(first_ty.resolve()))
    }

    fn infer_map(&mut self, map: &ast::MapExpr) -> Type {
        if map.entries.is_empty() {
            return Type::Map(
                Box::new(fresh_type_var(self.next_var_id)),
                Box::new(fresh_type_var(self.next_var_id)),
            );
        }

        let first_key_ty = self.infer_expr(&map.entries[0].key);
        let first_val_ty = self.infer_expr(&map.entries[0].value);

        for entry in map.entries.iter().skip(1) {
            let key_ty = self.infer_expr(&entry.key);
            let val_ty = self.infer_expr(&entry.value);
            if let Err(e) = unify(&key_ty, &first_key_ty, entry.key.span()) {
                self.errors.push(e);
            }
            if let Err(e) = unify(&val_ty, &first_val_ty, entry.value.span()) {
                self.errors.push(e);
            }
        }

        Type::Map(
            Box::new(first_key_ty.resolve()),
            Box::new(first_val_ty.resolve()),
        )
    }

    fn infer_struct_literal(&mut self, lit: &ast::StructLiteralExpr) -> Type {
        if let Some(def) = self.symbols.get_type(lit.name.symbol) {
            use super::symbols::TypeDef;
            use std::collections::HashMap;
            match def {
                TypeDef::Struct(s) => {
                    let struct_ty = self.symbols.to_struct_type(s);

                    // Build substitution map from type params to fresh type vars
                    let substitution: HashMap<lasso::Spur, Type> = struct_ty
                        .type_params
                        .iter()
                        .map(|tp| (tp.name, fresh_type_var(self.next_var_id)))
                        .collect();

                    for field_lit in &lit.fields {
                        let field_def = struct_ty.fields.iter().find(|f| f.name == field_lit.name.symbol);
                        if let Some(field_def) = field_def {
                            let value_ty = self.infer_expr(&field_lit.value);
                            // Apply substitution to field type to replace type params with type vars
                            let substituted_field_ty = field_def.ty.substitute(&substitution);
                            if let Err(e) = unify(&value_ty, &substituted_field_ty, field_lit.span) {
                                self.errors.push(e);
                            }
                        } else {
                            let field_name = self.interner.resolve(&field_lit.name.symbol).to_string();
                            self.errors.push(TypeError::UndefinedField {
                                ty: format!("{:?}", lit.name.symbol),
                                field: field_name,
                                span: field_lit.span,
                            });
                        }
                    }

                    // If struct has type parameters, return Generic type instead of Struct
                    if !struct_ty.type_params.is_empty() {
                        // Collect the type vars in param order for the return type
                        let type_args: Vec<Type> = struct_ty
                            .type_params
                            .iter()
                            .map(|tp| substitution.get(&tp.name).cloned().unwrap_or_else(|| fresh_type_var(self.next_var_id)))
                            .collect();
                        Type::Generic(lit.name.symbol, type_args)
                    } else {
                        Type::Struct(struct_ty)
                    }
                }
                TypeDef::Exception(exc) => {
                    // Handle exception types like struct literals
                    for field_lit in &lit.fields {
                        let field_def = exc.fields.iter().find(|(name, _)| *name == field_lit.name.symbol);
                        if let Some((_, field_ty)) = field_def {
                            let value_ty = self.infer_expr(&field_lit.value);
                            if let Err(e) = unify(&value_ty, field_ty, field_lit.span) {
                                self.errors.push(e);
                            }
                        } else {
                            let field_name = self.interner.resolve(&field_lit.name.symbol).to_string();
                            self.errors.push(TypeError::UndefinedField {
                                ty: format!("{:?}", lit.name.symbol),
                                field: field_name,
                                span: field_lit.span,
                            });
                        }
                    }
                    // Return struct-like type for exception
                    Type::Generic(lit.name.symbol, vec![])
                }
                _ => {
                    let name = self.interner.resolve(&lit.name.symbol).to_string();
                    self.errors.push(TypeError::Custom {
                        message: format!("{} is not a struct type", name),
                        span: lit.span,
                    });
                    Type::Error
                }
            }
        } else {
            let name = self.interner.resolve(&lit.name.symbol).to_string();
            self.errors.push(TypeError::UndefinedType {
                name,
                span: lit.span,
            });
            Type::Error
        }
    }

    fn infer_if(&mut self, if_expr: &ast::IfExpr) -> Type {
        let cond_ty = self.infer_expr(if_expr.condition);
        if let Err(e) = unify(&cond_ty, &Type::Bool, if_expr.condition.span()) {
            self.errors.push(e);
        }

        let then_ty = self.infer_block(if_expr.then_branch);

        if let Some(else_branch) = &if_expr.else_branch {
            let else_ty = match else_branch {
                ast::ElseExpr::ElseIf(elif) => self.infer_if(elif),
                ast::ElseExpr::Else(block) => self.infer_block(block),
            };
            if let Err(e) = unify(&then_ty, &else_ty, if_expr.span) {
                self.errors.push(e);
            }
            then_ty.resolve()
        } else {
            Type::Unit
        }
    }

    fn infer_block(&mut self, block: &ast::BlockExpr) -> Type {
        self.env.push_scope();

        for stmt in &block.statements {
            self.check_stmt(stmt);
        }

        let result = if let Some(tail) = &block.tail {
            self.infer_expr(tail)
        } else {
            Type::Unit
        };

        self.env.pop_scope();
        result
    }

    fn infer_lambda(&mut self, lambda: &ast::LambdaExpr) -> Type {
        self.env.push_scope();

        let mut param_types = Vec::new();
        for param in &lambda.params {
            let ty = if let Some(ty_annot) = &param.ty {
                self.convert_ast_type(ty_annot)
            } else {
                fresh_type_var(self.next_var_id)
            };
            self.env.define(param.name.symbol, ty.clone(), false);
            param_types.push(ty);
        }

        let expected_return_ty = if let Some(ret) = &lambda.return_ty {
            self.convert_ast_type(ret)
        } else {
            fresh_type_var(self.next_var_id)
        };

        self.env
            .enter_function(expected_return_ty.clone(), vec![], &[]);

        let body_ty = self.infer_expr(lambda.body);

        self.env.exit_function();

        let return_ty = if lambda.return_ty.is_some() {
            if let Err(e) = unify(&body_ty, &expected_return_ty, lambda.span)
                && !matches!(body_ty, Type::Unit) {
                    self.errors.push(e);
                }
            expected_return_ty
        } else if unify(&body_ty, &expected_return_ty, lambda.span).is_err() {
            body_ty
        } else {
            expected_return_ty.resolve()
        };

        self.env.pop_scope();

        Type::Function(FunctionType {
            params: param_types,
            returns: Box::new(return_ty),
            throws: vec![],
            is_variadic: false,
        })
    }

    fn infer_spawn(&mut self, spawn: &ast::SpawnExpr) -> Type {
        // Infer the block body for type checking purposes
        let _body_ty = self.infer_block(spawn.body);
        // Spawn runs concurrently and doesn't return a value
        Type::Unit
    }

    fn infer_try(&mut self, try_expr: &ast::TryExpr) -> Type {
        
        self.infer_expr(try_expr.expr)
    }

    fn infer_catch(&mut self, catch: &ast::CatchExpr) -> Type {
        let expr_ty = self.infer_expr(catch.expr);

        // Determine the exception type from the expression being caught
        let exception_ty = self.get_throws_type(catch.expr);

        self.env.push_scope();

        let error_spur = catch.error_binding.symbol;
        self.env.define(error_spur, exception_ty, true);

        for stmt in &catch.handler.statements {
            self.check_stmt(stmt);
        }
        if let Some(tail) = catch.handler.tail {
            self.infer_expr(tail);
        }

        self.env.pop_scope();

        expr_ty
    }

    /// Get the exception type that an expression can throw
    fn get_throws_type(&self, expr: &Expression) -> Type {
        match expr {
            Expression::Call(call) => {
                // Check if callee is a function with throws
                if let Expression::Identifier(ident) = call.callee
                    && let Some(func_sig) = self.symbols.get_function(ident.ident.symbol)
                        && let Some(first_throw) = func_sig.throws.first() {
                            return first_throw.clone();
                        }
                Type::Error
            }
            Expression::MethodCall(_method_call) => {
                // For method calls, we'd need to look up the method's throws
                // For now, return Error as fallback
                Type::Error
            }
            _ => Type::Error,
        }
    }

    fn infer_cast(&mut self, cast: &ast::CastExpr) -> Type {
        self.infer_expr(cast.expr);
        self.convert_ast_type(&cast.target_ty)
    }

    fn infer_fallible_cast(&mut self, cast: &ast::FallibleCastExpr) -> Type {
        self.infer_expr(cast.expr);
        let target_ty = self.convert_ast_type(&cast.target_ty);
        Type::Option(Box::new(target_ty))
    }

    fn infer_range(&mut self, range: &ast::RangeExpr) -> Type {
        let elem_ty = if let Some(start) = &range.start {
            self.infer_expr(start)
        } else if let Some(end) = &range.end {
            self.infer_expr(end)
        } else {
            Type::Int
        };

        if let Some(start) = &range.start {
            let ty = self.infer_expr(start);
            if let Err(e) = unify(&ty, &elem_ty, start.span()) {
                self.errors.push(e);
            }
        }
        if let Some(end) = &range.end {
            let ty = self.infer_expr(end);
            if let Err(e) = unify(&ty, &elem_ty, end.span()) {
                self.errors.push(e);
            }
        }

        Type::Array(Box::new(elem_ty.resolve()))
    }

    pub fn infer_pattern(&mut self, pattern: &Pattern, scrutinee_ty: &Type) -> Type {
        match pattern {
            Pattern::Literal(lit) => {
                // Literal patterns have a fixed type
                match &lit.value {
                    Literal::Int(_) => Type::Int,
                    Literal::UInt(_) => Type::Uint,
                    Literal::Float(_) => Type::Float,
                    Literal::Bool(_) => Type::Bool,
                    Literal::String(_) => Type::String,
                    Literal::Bytes(_) => Type::Bytes,
                    Literal::None => {
                        let inner = fresh_type_var(self.next_var_id);
                        Type::Option(Box::new(inner))
                    }
                }
            }

            Pattern::Identifier(ident) => {
                // Identifier pattern: first check if it's an enum variant name
                if let Type::Enum(enum_ty) = scrutinee_ty {
                    for variant in &enum_ty.variants {
                        if variant.name == ident.ident.symbol {
                            // It's a variant name, return the enum type
                            return scrutinee_ty.clone();
                        }
                    }
                }
                // Otherwise it's a binding that captures the scrutinee value
                self.env.define(ident.ident.symbol, scrutinee_ty.clone(), false);
                scrutinee_ty.clone()
            }

            Pattern::Variant(variant) => {
                // Get the enum type from the path
                if variant.path.is_empty() {
                    return Type::Error;
                }

                // First segment is the enum type or variant name
                let first = &variant.path[0];

                // Check if it's a qualified path (EnumType::Variant) or bare variant
                if variant.path.len() == 1 {
                    // Bare variant name - use scrutinee type to resolve
                    if let Type::Enum(enum_ty) = scrutinee_ty {
                        for var in &enum_ty.variants {
                            if var.name == first.symbol {
                                // Bind variant fields if present
                                if let Some(ref fields) = var.fields {
                                    for (i, binding) in variant.bindings.iter().enumerate() {
                                        if i < fields.len() {
                                            self.env.define(binding.symbol, fields[i].clone(), false);
                                        }
                                    }
                                }
                                return scrutinee_ty.clone();
                            }
                        }
                        // Unknown variant
                        let variant_name = self.interner.resolve(&first.symbol).to_string();
                        self.errors.push(TypeError::Custom {
                            message: format!("unknown variant '{}'", variant_name),
                            span: variant.span,
                        });
                    }
                    return Type::Error;
                }

                // Qualified path - look up the enum type
                if let Some(def) = self.symbols.get_type(first.symbol) {
                    use super::symbols::TypeDef;
                    if let TypeDef::Enum(e) = def {
                        let enum_ty = self.symbols.to_enum_type(e);
                        let variant_name = variant.path.last().unwrap().symbol;

                        for var in &enum_ty.variants {
                            if var.name == variant_name {
                                // Bind variant fields if present
                                if let Some(ref fields) = var.fields {
                                    for (i, binding) in variant.bindings.iter().enumerate() {
                                        if i < fields.len() {
                                            self.env.define(binding.symbol, fields[i].clone(), false);
                                        }
                                    }
                                }
                                return Type::Enum(enum_ty);
                            }
                        }
                    }
                }

                Type::Error
            }

            Pattern::Wildcard(_) => {
                // Wildcard matches anything
                scrutinee_ty.clone()
            }

            Pattern::_Phantom(_) => Type::Error,
        }
    }

    pub fn check_stmt(&mut self, stmt: &ast::Statement) {
        use ast::Statement::*;
        match stmt {
            Var(var) => {
                // Handle var x = opt else { ... } pattern
                if var.else_block.is_some() {
                    let init_ty = if let Some(init) = &var.init {
                        self.infer_expr(init)
                    } else {
                        self.errors.push(TypeError::Custom {
                            message: "else clause requires an initializer".to_string(),
                            span: var.span,
                        });
                        Type::Error
                    };

                    let resolved = init_ty.resolve();
                    let inner_ty = match &resolved {
                        Type::Option(inner) => (**inner).clone(),
                        Type::Error => Type::Error,
                        _ => {
                            self.errors.push(TypeError::Custom {
                                message: format!(
                                    "else clause can only be used with option types, found {}",
                                    resolved
                                ),
                                span: var.span,
                            });
                            Type::Error
                        }
                    };

                    // Type check the else block
                    if let Some(ref else_block) = var.else_block {
                        self.env.push_scope();
                        for stmt in &else_block.statements {
                            self.check_stmt(stmt);
                        }
                        self.env.pop_scope();
                    }

                    // Unify with annotation if present
                    let ty = if let Some(annot) = &var.ty {
                        let expected = self.convert_ast_type(annot);
                        if let Err(e) = unify(&inner_ty, &expected, var.span) {
                            self.errors.push(e);
                        }
                        expected
                    } else {
                        inner_ty
                    };

                    self.env.define(var.name.symbol, ty, var.mutable);
                } else {
                    // Original logic for normal var statements
                    let ty = if let Some(annot) = &var.ty {
                        self.convert_ast_type(annot)
                    } else if let Some(init) = &var.init {
                        self.infer_expr(init)
                    } else {
                        fresh_type_var(self.next_var_id)
                    };

                    if let Some(init) = &var.init {
                        let init_ty = self.infer_expr(init);
                        // Allow int literals to be assigned to uint (non-negative int → uint coercion)
                        let should_unify = !self.is_int_to_uint_coercion(&init_ty, &ty, init);
                        if should_unify
                            && let Err(e) = unify(&init_ty, &ty, init.span()) {
                                self.errors.push(e);
                            }
                    }

                    self.env.define(var.name.symbol, ty, var.mutable);
                }
            }
            Const(c) => {
                let ty = if let Some(annot) = &c.ty {
                    self.convert_ast_type(annot)
                } else {
                    self.infer_expr(&c.init)
                };

                let init_ty = self.infer_expr(&c.init);
                // Allow int literals to be assigned to uint (non-negative int → uint coercion)
                let should_unify = !self.is_int_to_uint_coercion(&init_ty, &ty, &c.init);
                if should_unify
                    && let Err(e) = unify(&init_ty, &ty, c.init.span()) {
                        self.errors.push(e);
                    }

                self.env.define(c.name.symbol, ty, false);
            }
            Assign(assign) => {
                // Special case for map index assignment: map[key] = value
                // The value should be of type V, not option<V>
                if let ast::Expression::Index(idx) = &assign.target {
                    let base_ty = self.infer_expr(idx.base);
                    let index_ty = self.infer_expr(idx.index);
                    let resolved = base_ty.resolve();

                    if let Type::Map(key, val) = resolved {
                        if let Err(e) = unify(&index_ty, &key, idx.index.span()) {
                            self.errors.push(e);
                        }
                        let value_ty = self.infer_expr(&assign.value);
                        if let Err(e) = unify(&value_ty, &val, assign.value.span()) {
                            self.errors.push(e);
                        }
                        return;
                    }
                }

                let target_ty = self.infer_expr(&assign.target);
                let value_ty = self.infer_expr(&assign.value);
                if let Err(e) = unify(&value_ty, &target_ty, assign.span) {
                    self.errors.push(e);
                }
            }
            Expression(expr) => {
                self.infer_expr(&expr.expr);
            }
            Return(ret) => {
                if let Some(value) = &ret.value {
                    let ret_ty = self.infer_expr(value);
                    if let Some(expected) = self.env.expected_return_type()
                        && let Err(e) = unify(&ret_ty, expected, value.span()) {
                            self.errors.push(e);
                        }
                }
            }
            Throw(throw) => {
                self.infer_expr(&throw.value);
            }
            If(if_stmt) => {
                let cond_ty = self.infer_expr(&if_stmt.condition);
                if let Err(e) = unify(&cond_ty, &Type::Bool, if_stmt.condition.span()) {
                    self.errors.push(e);
                }

                self.env.push_scope();
                for s in &if_stmt.then_branch.statements {
                    self.check_stmt(s);
                }
                self.env.pop_scope();

                if let Some(else_branch) = &if_stmt.else_branch {
                    match else_branch {
                        ast::ElseBranch::ElseIf(elif) => {
                            self.check_stmt(&ast::Statement::If(*elif.clone()));
                        }
                        ast::ElseBranch::Else(block) => {
                            self.env.push_scope();
                            for s in &block.statements {
                                self.check_stmt(s);
                            }
                            self.env.pop_scope();
                        }
                    }
                }
            }
            While(while_stmt) => {
                let cond_ty = self.infer_expr(&while_stmt.condition);
                if let Err(e) = unify(&cond_ty, &Type::Bool, while_stmt.condition.span()) {
                    self.errors.push(e);
                }

                self.env.push_scope();
                self.env.enter_loop();
                for s in &while_stmt.body.statements {
                    self.check_stmt(s);
                }
                self.env.exit_loop();
                self.env.pop_scope();
            }
            For(for_stmt) => {
                let iterable_ty = self.infer_expr(&for_stmt.iterable);
                let resolved = iterable_ty.resolve();

                let elem_ty = match &resolved {
                    Type::Array(elem) | Type::FixedArray(elem, _) => (**elem).clone(),
                    Type::Map(_, val) => (**val).clone(),
                    Type::String => Type::String,
                    Type::Error => Type::Error,
                    _ => {
                        self.errors.push(TypeError::NotIterable {
                            ty: iterable_ty.to_string(),
                            span: for_stmt.iterable.span(),
                        });
                        Type::Error
                    }
                };

                self.env.push_scope();
                if let Some(idx) = &for_stmt.index {
                    self.env.define(idx.symbol, Type::Int, false);
                }
                self.env.define(for_stmt.value.symbol, elem_ty, false);

                self.env.enter_loop();
                for s in &for_stmt.body.statements {
                    self.check_stmt(s);
                }
                self.env.exit_loop();
                self.env.pop_scope();
            }
            Loop(loop_stmt) => {
                self.env.push_scope();
                self.env.enter_loop();
                for s in &loop_stmt.body.statements {
                    self.check_stmt(s);
                }
                self.env.exit_loop();
                self.env.pop_scope();
            }
            Switch(switch) => {
                let scrutinee_ty = self.infer_expr(&switch.scrutinee);

                // Set switch context for resolving bare enum variants in case patterns
                let old_scrutinee = self.switch_scrutinee.take();
                self.switch_scrutinee = Some(scrutinee_ty.resolve());

                for case in &switch.cases {
                    self.env.push_scope();
                    let resolved_scrutinee = scrutinee_ty.resolve();
                    let pattern_ty = self.infer_pattern(&case.pattern, &resolved_scrutinee);
                    if let Err(e) = unify(&pattern_ty, &scrutinee_ty, case.pattern.span()) {
                        self.errors.push(e);
                    }
                    for s in &case.body.statements {
                        self.check_stmt(s);
                    }
                    self.env.pop_scope();
                }

                // Restore previous context
                self.switch_scrutinee = old_scrutinee;

                if let Some(default) = &switch.default {
                    self.env.push_scope();
                    for s in &default.statements {
                        self.check_stmt(s);
                    }
                    self.env.pop_scope();
                }
            }
            Break(brk) => {
                if !self.env.in_loop() {
                    self.errors.push(TypeError::BreakOutsideLoop { span: brk.span });
                }
            }
            Continue(cont) => {
                if !self.env.in_loop() {
                    self.errors
                        .push(TypeError::ContinueOutsideLoop { span: cont.span });
                }
            }
            Block(block) => {
                self.env.push_scope();
                for s in &block.statements {
                    self.check_stmt(s);
                }
                self.env.pop_scope();
            }
        }
    }

    pub fn convert_ast_type(&self, ast_ty: &ast::NamlType) -> Type {
        match ast_ty {
            ast::NamlType::Int => Type::Int,
            ast::NamlType::Uint => Type::Uint,
            ast::NamlType::Float => Type::Float,
            ast::NamlType::Bool => Type::Bool,
            ast::NamlType::String => Type::String,
            ast::NamlType::Bytes => Type::Bytes,
            ast::NamlType::Unit => Type::Unit,
            ast::NamlType::Decimal { .. } => Type::Float,
            ast::NamlType::Array(inner) => Type::Array(Box::new(self.convert_ast_type(inner))),
            ast::NamlType::FixedArray(inner, n) => {
                Type::FixedArray(Box::new(self.convert_ast_type(inner)), *n)
            }
            ast::NamlType::Option(inner) => Type::Option(Box::new(self.convert_ast_type(inner))),
            ast::NamlType::Map(k, v) => Type::Map(
                Box::new(self.convert_ast_type(k)),
                Box::new(self.convert_ast_type(v)),
            ),
            ast::NamlType::Channel(inner) => {
                Type::Channel(Box::new(self.convert_ast_type(inner)))
            }
            ast::NamlType::Named(ident) => {
                // Look up the name to see if it's a known type (struct, enum, etc.)
                if let Some(def) = self.symbols.get_type(ident.symbol) {
                    use super::symbols::TypeDef;
                    match def {
                        TypeDef::Struct(s) => Type::Struct(self.symbols.to_struct_type(s)),
                        TypeDef::Enum(e) => Type::Enum(self.symbols.to_enum_type(e)),
                        TypeDef::Interface(i) => Type::Interface(self.symbols.to_interface_type(i)),
                        TypeDef::Exception(e) => Type::Exception(e.name),
                    }
                } else {
                    // Fall back to generic type (for type parameters)
                    Type::Generic(ident.symbol, Vec::new())
                }
            }
            ast::NamlType::Generic(ident, args) => {
                let converted_args = args.iter().map(|a| self.convert_ast_type(a)).collect();
                Type::Generic(ident.symbol, converted_args)
            }
            ast::NamlType::Function { params, returns } => {
                let param_types = params.iter().map(|p| self.convert_ast_type(p)).collect();
                Type::Function(FunctionType {
                    params: param_types,
                    returns: Box::new(self.convert_ast_type(returns)),
                    throws: vec![],
                    is_variadic: false,
                })
            }
            ast::NamlType::Inferred => fresh_type_var(&mut 0),
        }
    }

    fn is_int_to_uint_coercion(&self, init_ty: &Type, target_ty: &Type, init: &Expression) -> bool {
        let init_resolved = init_ty.resolve();
        let target_resolved = target_ty.resolve();

        if target_resolved != Type::Uint || init_resolved != Type::Int {
            return false;
        }
        matches!(init, Expression::Literal(lit) if matches!(lit.value, Literal::Int(_)))
    }

    fn coerce_int_uint(
        &self,
        left_ty: &Type,
        right_ty: &Type,
        left_expr: &Expression,
        right_expr: &Expression,
    ) -> Option<Type> {
        let is_left_int_literal =
            matches!(left_expr, Expression::Literal(lit) if matches!(lit.value, Literal::Int(_)));
        let is_right_int_literal =
            matches!(right_expr, Expression::Literal(lit) if matches!(lit.value, Literal::Int(_)));

        match (left_ty, right_ty) {
            // uint op int_literal -> uint
            (Type::Uint, Type::Int) if is_right_int_literal => Some(Type::Uint),
            // int_literal op uint -> uint
            (Type::Int, Type::Uint) if is_left_int_literal => Some(Type::Uint),
            _ => None,
        }
    }
}
