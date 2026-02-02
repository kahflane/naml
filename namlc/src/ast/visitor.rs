//!
//! AST Visitor Pattern
//!
//! This module provides a visitor trait for traversing the AST. The visitor
//! pattern allows implementing different operations over the AST without
//! modifying the AST node types.
//!
//! Usage:
//! - Implement the Visitor trait
//! - Override only the methods you need
//! - Default implementations call walk_* functions for recursive traversal
//!
//! Common use cases:
//! - Type checking: visit expressions and statements to check types
//! - Code generation: visit items to emit code
//! - Linting: visit nodes to check for patterns
//! - Pretty printing: visit nodes to format code
//!

use super::expressions::*;
use super::items::*;
use super::patterns::*;
use super::statements::*;
use super::types::*;

pub trait Visitor<'ast>: Sized {
    fn visit_item(&mut self, item: &Item<'ast>) {
        walk_item(self, item)
    }

    fn visit_stmt(&mut self, stmt: &Statement<'ast>) {
        walk_stmt(self, stmt)
    }

    fn visit_expr(&mut self, expr: &Expression<'ast>) {
        walk_expr(self, expr)
    }

    fn visit_type(&mut self, ty: &NamlType) {
        walk_type(self, ty)
    }

    fn visit_pattern(&mut self, pattern: &Pattern<'ast>) {
        walk_pattern(self, pattern)
    }

    fn visit_ident(&mut self, _ident: &Ident) {}
}

pub fn walk_item<'ast, V: Visitor<'ast>>(v: &mut V, item: &Item<'ast>) {
    match item {
        Item::Function(f) => {
            v.visit_ident(&f.name);
            if let Some(ref recv) = f.receiver {
                v.visit_ident(&recv.name);
                v.visit_type(&recv.ty);
            }
            for generic in &f.generics {
                v.visit_ident(&generic.name);
                for bound in &generic.bounds {
                    v.visit_type(bound);
                }
            }
            for param in &f.params {
                v.visit_ident(&param.name);
                v.visit_type(&param.ty);
            }
            if let Some(ref ret) = f.return_ty {
                v.visit_type(ret);
            }
            for throws_ty in &f.throws {
                v.visit_type(throws_ty);
            }
            if let Some(ref body) = f.body {
                for stmt in &body.statements {
                    v.visit_stmt(stmt);
                }
            }
        }
        Item::Struct(s) => {
            v.visit_ident(&s.name);
            for generic in &s.generics {
                v.visit_ident(&generic.name);
            }
            for impl_ty in &s.implements {
                v.visit_type(impl_ty);
            }
            for field in &s.fields {
                v.visit_ident(&field.name);
                v.visit_type(&field.ty);
            }
        }
        Item::Interface(i) => {
            v.visit_ident(&i.name);
            for generic in &i.generics {
                v.visit_ident(&generic.name);
            }
            for ext in &i.extends {
                v.visit_type(ext);
            }
            for method in &i.methods {
                v.visit_ident(&method.name);
                for param in &method.params {
                    v.visit_ident(&param.name);
                    v.visit_type(&param.ty);
                }
                if let Some(ref ret) = method.return_ty {
                    v.visit_type(ret);
                }
            }
        }
        Item::Enum(e) => {
            v.visit_ident(&e.name);
            for generic in &e.generics {
                v.visit_ident(&generic.name);
            }
            for variant in &e.variants {
                v.visit_ident(&variant.name);
                if let Some(ref fields) = variant.fields {
                    for ty in fields {
                        v.visit_type(ty);
                    }
                }
            }
        }
        Item::Exception(e) => {
            v.visit_ident(&e.name);
            for field in &e.fields {
                v.visit_ident(&field.name);
                v.visit_type(&field.ty);
            }
        }
        Item::Use(u) => {
            for seg in &u.path {
                v.visit_ident(seg);
            }
            if let UseItems::Specific(entries) = &u.items {
                for entry in entries {
                    v.visit_ident(&entry.name);
                    if let Some(ref alias) = entry.alias {
                        v.visit_ident(alias);
                    }
                }
            }
        }
        Item::Extern(e) => {
            v.visit_ident(&e.name);
            for param in &e.params {
                v.visit_ident(&param.name);
                v.visit_type(&param.ty);
            }
            if let Some(ref ret) = e.return_ty {
                v.visit_type(ret);
            }
        }
        Item::TypeAlias(a) => {
            v.visit_ident(&a.name);
            for generic in &a.generics {
                v.visit_ident(&generic.name);
                for bound in &generic.bounds {
                    v.visit_type(bound);
                }
            }
            v.visit_type(&a.aliased_type);
        }
        Item::TopLevelStmt(s) => {
            v.visit_stmt(&s.stmt);
        }
        Item::Mod(m) => {
            v.visit_ident(&m.name);
            if let Some(ref body) = m.body {
                for item in body {
                    v.visit_item(item);
                }
            }
        }
    }
}

pub fn walk_stmt<'ast, V: Visitor<'ast>>(v: &mut V, stmt: &Statement<'ast>) {
    match stmt {
        Statement::Var(s) => {
            v.visit_ident(&s.name);
            if let Some(ref ty) = s.ty {
                v.visit_type(ty);
            }
            if let Some(ref init) = s.init {
                v.visit_expr(init);
            }
        }
        Statement::Const(s) => {
            v.visit_ident(&s.name);
            if let Some(ref ty) = s.ty {
                v.visit_type(ty);
            }
            v.visit_expr(&s.init);
        }
        Statement::Assign(s) => {
            v.visit_expr(&s.target);
            v.visit_expr(&s.value);
        }
        Statement::Expression(s) => {
            v.visit_expr(&s.expr);
        }
        Statement::Return(s) => {
            if let Some(ref val) = s.value {
                v.visit_expr(val);
            }
        }
        Statement::Throw(s) => {
            v.visit_expr(&s.value);
        }
        Statement::If(s) => {
            v.visit_expr(&s.condition);
            for stmt in &s.then_branch.statements {
                v.visit_stmt(stmt);
            }
            if let Some(ref else_branch) = s.else_branch {
                match else_branch {
                    ElseBranch::ElseIf(elif) => {
                        v.visit_stmt(&Statement::If((**elif).clone()));
                    }
                    ElseBranch::Else(block) => {
                        for stmt in &block.statements {
                            v.visit_stmt(stmt);
                        }
                    }
                }
            }
        }
        Statement::While(s) => {
            v.visit_expr(&s.condition);
            for stmt in &s.body.statements {
                v.visit_stmt(stmt);
            }
        }
        Statement::For(s) => {
            if let Some(ref idx) = s.index {
                v.visit_ident(idx);
            }
            if let Some(ref ty) = s.index_ty {
                v.visit_type(ty);
            }
            v.visit_ident(&s.value);
            if let Some(ref ty) = s.value_ty {
                v.visit_type(ty);
            }
            v.visit_expr(&s.iterable);
            for stmt in &s.body.statements {
                v.visit_stmt(stmt);
            }
        }
        Statement::Loop(s) => {
            for stmt in &s.body.statements {
                v.visit_stmt(stmt);
            }
        }
        Statement::Switch(s) => {
            v.visit_expr(&s.scrutinee);
            for case in &s.cases {
                v.visit_pattern(&case.pattern);
                for stmt in &case.body.statements {
                    v.visit_stmt(stmt);
                }
            }
            if let Some(ref default) = s.default {
                for stmt in &default.statements {
                    v.visit_stmt(stmt);
                }
            }
        }
        Statement::Break(_) | Statement::Continue(_) => {}
        Statement::Block(s) => {
            for stmt in &s.statements {
                v.visit_stmt(stmt);
            }
        }
        Statement::Locked(s) => {
            v.visit_ident(&s.binding);
            if let Some(ref ty) = s.binding_ty {
                v.visit_type(ty);
            }
            v.visit_expr(&s.mutex);
            for stmt in &s.body.statements {
                v.visit_stmt(stmt);
            }
        }
    }
}

pub fn walk_expr<'ast, V: Visitor<'ast>>(v: &mut V, expr: &Expression<'ast>) {
    match expr {
        Expression::Literal(_) => {}
        Expression::Identifier(e) => {
            v.visit_ident(&e.ident);
        }
        Expression::Path(e) => {
            for seg in &e.segments {
                v.visit_ident(seg);
            }
        }
        Expression::Binary(e) => {
            v.visit_expr(e.left);
            v.visit_expr(e.right);
        }
        Expression::Unary(e) => {
            v.visit_expr(e.operand);
        }
        Expression::Call(e) => {
            v.visit_expr(e.callee);
            for ty in &e.type_args {
                v.visit_type(ty);
            }
            for arg in &e.args {
                v.visit_expr(arg);
            }
        }
        Expression::MethodCall(e) => {
            v.visit_expr(e.receiver);
            v.visit_ident(&e.method);
            for ty in &e.type_args {
                v.visit_type(ty);
            }
            for arg in &e.args {
                v.visit_expr(arg);
            }
        }
        Expression::Index(e) => {
            v.visit_expr(e.base);
            v.visit_expr(e.index);
        }
        Expression::Field(e) => {
            v.visit_expr(e.base);
            v.visit_ident(&e.field);
        }
        Expression::Array(e) => {
            for elem in &e.elements {
                v.visit_expr(elem);
            }
        }
        Expression::Map(e) => {
            for entry in &e.entries {
                v.visit_expr(&entry.key);
                v.visit_expr(&entry.value);
            }
        }
        Expression::StructLiteral(e) => {
            v.visit_ident(&e.name);
            for field in &e.fields {
                v.visit_ident(&field.name);
                v.visit_expr(&field.value);
            }
        }
        Expression::If(e) => {
            v.visit_expr(e.condition);
            for stmt in &e.then_branch.statements {
                v.visit_stmt(stmt);
            }
            if let Some(tail) = e.then_branch.tail {
                v.visit_expr(tail);
            }
            if let Some(ref else_branch) = e.else_branch {
                match else_branch {
                    ElseExpr::ElseIf(elif) => {
                        v.visit_expr(&Expression::If((**elif).clone()));
                    }
                    ElseExpr::Else(block) => {
                        for stmt in &block.statements {
                            v.visit_stmt(stmt);
                        }
                        if let Some(tail) = block.tail {
                            v.visit_expr(tail);
                        }
                    }
                }
            }
        }
        Expression::Block(e) => {
            for stmt in &e.statements {
                v.visit_stmt(stmt);
            }
            if let Some(tail) = e.tail {
                v.visit_expr(tail);
            }
        }
        Expression::Lambda(e) => {
            for param in &e.params {
                v.visit_ident(&param.name);
                if let Some(ref ty) = param.ty {
                    v.visit_type(ty);
                }
            }
            if let Some(ref ret) = e.return_ty {
                v.visit_type(ret);
            }
            v.visit_expr(e.body);
        }
        Expression::Spawn(e) => {
            for stmt in &e.body.statements {
                v.visit_stmt(stmt);
            }
            if let Some(tail) = e.body.tail {
                v.visit_expr(tail);
            }
        }
        Expression::Try(e) => {
            v.visit_expr(e.expr);
        }
        Expression::Catch(e) => {
            v.visit_expr(e.expr);
            v.visit_ident(&e.error_binding);
            for stmt in &e.handler.statements {
                v.visit_stmt(stmt);
            }
            if let Some(tail) = e.handler.tail {
                v.visit_expr(tail);
            }
        }
        Expression::Cast(e) => {
            v.visit_expr(e.expr);
            v.visit_type(&e.target_ty);
        }
        Expression::Range(e) => {
            if let Some(start) = e.start {
                v.visit_expr(start);
            }
            if let Some(end) = e.end {
                v.visit_expr(end);
            }
        }
        Expression::Grouped(e) => {
            v.visit_expr(e.inner);
        }
        Expression::Some(e) => {
            v.visit_expr(e.value);
        }
        Expression::Ternary(e) => {
            v.visit_expr(e.condition);
            v.visit_expr(e.true_expr);
            v.visit_expr(e.false_expr);
        }
        Expression::Elvis(e) => {
            v.visit_expr(e.left);
            v.visit_expr(e.right);
        }
        Expression::FallibleCast(e) => {
            v.visit_expr(e.expr);
            v.visit_type(&e.target_ty);
        }
        Expression::ForceUnwrap(e) => {
            v.visit_expr(e.expr);
        }
        Expression::TemplateString(_) => {
            // Template string expressions are stored as raw strings
            // and parsed during codegen, so nothing to visit here
        }
    }
}

pub fn walk_type<'ast, V: Visitor<'ast>>(v: &mut V, ty: &NamlType) {
    match ty {
        NamlType::Array(inner) => v.visit_type(inner),
        NamlType::FixedArray(inner, _) => v.visit_type(inner),
        NamlType::Option(inner) => v.visit_type(inner),
        NamlType::Map(key, val) => {
            v.visit_type(key);
            v.visit_type(val);
        }
        NamlType::Channel(inner) => v.visit_type(inner),
        NamlType::Named(ident) => v.visit_ident(ident),
        NamlType::Generic(ident, args) => {
            v.visit_ident(ident);
            for arg in args {
                v.visit_type(arg);
            }
        }
        NamlType::Function { params, returns } => {
            for param in params {
                v.visit_type(param);
            }
            v.visit_type(returns);
        }
        _ => {}
    }
}

pub fn walk_pattern<'ast, V: Visitor<'ast>>(v: &mut V, pattern: &Pattern<'ast>) {
    match pattern {
        Pattern::Literal(_) => {
            // Literals don't contain nested visitable elements
        }
        Pattern::Identifier(p) => {
            v.visit_ident(&p.ident);
        }
        Pattern::Variant(p) => {
            for seg in &p.path {
                v.visit_ident(seg);
            }
            for binding in &p.bindings {
                v.visit_ident(binding);
            }
        }
        Pattern::Wildcard(_) => {
            // Wildcard has no nested elements
        }
        Pattern::_Phantom(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::Span;
    use lasso::Rodeo;

    struct IdentCounter {
        count: usize,
    }

    impl<'ast> Visitor<'ast> for IdentCounter {
        fn visit_ident(&mut self, _ident: &Ident) {
            self.count += 1;
        }
    }

    #[test]
    fn test_count_identifiers() {
        let mut rodeo = Rodeo::default();
        let ident1 = Ident::new(rodeo.get_or_intern("x"), Span::dummy());
        let ident2 = Ident::new(rodeo.get_or_intern("y"), Span::dummy());

        let ty = NamlType::Map(
            Box::new(NamlType::Named(ident1)),
            Box::new(NamlType::Named(ident2)),
        );

        let mut counter = IdentCounter { count: 0 };
        counter.visit_type(&ty);
        assert_eq!(counter.count, 2);
    }
}
