use std::collections::{HashMap, HashSet};

use crate::ast::{Expression, Statement};
use crate::codegen::CodegenError;
use crate::codegen::cranelift::{JitCompiler, SpawnBlockInfo, LambdaInfo};
use crate::codegen::cranelift::heap::{HeapType, heap_type_from_type};

impl<'a> JitCompiler<'a> {
    pub fn scan_for_spawn_blocks(
        &mut self,
        block: &crate::ast::BlockStmt<'_>,
    ) -> Result<(), CodegenError> {
        for stmt in &block.statements {
            self.scan_statement_for_spawns(stmt)?;
        }
        Ok(())
    }

    fn scan_statement_for_spawns(&mut self, stmt: &Statement<'_>) -> Result<(), CodegenError> {
        match stmt {
            Statement::Expression(expr_stmt) => {
                self.scan_expression_for_spawns(&expr_stmt.expr)?;
            }
            Statement::If(if_stmt) => {
                self.scan_expression_for_spawns(&if_stmt.condition)?;
                self.scan_for_spawn_blocks(&if_stmt.then_branch)?;
                if let Some(ref else_branch) = if_stmt.else_branch {
                    match else_branch {
                        crate::ast::ElseBranch::ElseIf(elif) => {
                            self.scan_statement_for_spawns(&Statement::If(*elif.clone()))?;
                        }
                        crate::ast::ElseBranch::Else(block) => {
                            self.scan_for_spawn_blocks(block)?;
                        }
                    }
                }
            }
            Statement::While(while_stmt) => {
                self.scan_expression_for_spawns(&while_stmt.condition)?;
                self.scan_for_spawn_blocks(&while_stmt.body)?;
            }
            Statement::For(for_stmt) => {
                self.scan_expression_for_spawns(&for_stmt.iterable)?;
                self.scan_for_spawn_blocks(&for_stmt.body)?;
            }
            Statement::Loop(loop_stmt) => {
                self.scan_for_spawn_blocks(&loop_stmt.body)?;
            }
            Statement::Switch(switch_stmt) => {
                self.scan_expression_for_spawns(&switch_stmt.scrutinee)?;
                for case in &switch_stmt.cases {
                    self.scan_for_spawn_blocks(&case.body)?;
                }
                if let Some(ref default) = switch_stmt.default {
                    self.scan_for_spawn_blocks(default)?;
                }
            }
            Statement::Block(block) => {
                self.scan_for_spawn_blocks(block)?;
            }
            Statement::Var(var_stmt) => {
                if let Some(ref init) = var_stmt.init {
                    self.scan_expression_for_spawns(init)?;
                }
            }
            Statement::Assign(assign_stmt) => {
                self.scan_expression_for_spawns(&assign_stmt.value)?;
            }
            Statement::Return(ret_stmt) => {
                if let Some(ref value) = ret_stmt.value {
                    self.scan_expression_for_spawns(value)?;
                }
            }
            Statement::Locked(locked_stmt) => {
                self.scan_expression_for_spawns(&locked_stmt.mutex)?;
                self.scan_for_spawn_blocks(&locked_stmt.body)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn scan_expression_for_spawns(&mut self, expr: &Expression<'_>) -> Result<(), CodegenError> {
        match expr {
            Expression::Spawn(spawn_expr) => {
                // Found a spawn block - collect captured variables
                let captured = self.collect_captured_vars_expr(spawn_expr.body);
                let id = self.spawn_counter;
                self.spawn_counter += 1;
                let func_name = format!("__spawn_{}", id);

                // Store raw pointer to body for deferred trampoline compilation
                // Safety: Only used within the same compile() call
                // Note: spawn_expr.body is already a &BlockExpr, so we cast it directly
                #[allow(clippy::unnecessary_cast)]
                let body_ptr = spawn_expr.body as *const crate::ast::BlockExpr<'_>
                    as *const crate::ast::BlockExpr<'static>;

                let captured_heap_types =
                    self.find_captured_var_heap_types(spawn_expr.body, &captured);

                self.spawn_blocks.insert(
                    id,
                    SpawnBlockInfo {
                        id,
                        func_name,
                        captured_vars: captured,
                        captured_heap_types,
                        body_ptr,
                    },
                );
                self.spawn_body_to_id.insert(body_ptr as usize, id);

                // Also scan inside spawn block for nested spawns
                self.scan_for_spawn_blocks_expr(spawn_expr.body)?;
            }
            Expression::Lambda(lambda_expr) => {
                // Found a lambda - collect captured variables
                let captured = self.collect_captured_vars_for_lambda(lambda_expr);
                let id = self.lambda_counter;
                self.lambda_counter += 1;
                let func_name = format!("__lambda_{}", id);

                // Collect parameter names
                let param_names: Vec<String> = lambda_expr
                    .params
                    .iter()
                    .map(|p| self.interner.resolve(&p.name.symbol).to_string())
                    .collect();

                // Store raw pointer to body for deferred lambda compilation
                #[allow(clippy::unnecessary_cast)]
                let body_ptr = lambda_expr.body as *const crate::ast::Expression<'_>
                    as *const crate::ast::Expression<'static>;

                self.lambda_blocks.insert(
                    id,
                    LambdaInfo {
                        id,
                        func_name,
                        captured_vars: captured,
                        param_names,
                        body_ptr,
                    },
                );
                self.lambda_body_to_id.insert(body_ptr as usize, id);

                // Scan lambda body for nested spawns/lambdas
                self.scan_expression_for_spawns(lambda_expr.body)?;
            }
            Expression::Binary(bin) => {
                self.scan_expression_for_spawns(bin.left)?;
                self.scan_expression_for_spawns(bin.right)?;
            }
            Expression::Unary(un) => {
                self.scan_expression_for_spawns(un.operand)?;
            }
            Expression::Call(call) => {
                self.scan_expression_for_spawns(call.callee)?;
                for arg in &call.args {
                    self.scan_expression_for_spawns(arg)?;
                }
            }
            Expression::MethodCall(method) => {
                self.scan_expression_for_spawns(method.receiver)?;
                for arg in &method.args {
                    self.scan_expression_for_spawns(arg)?;
                }
            }
            Expression::Index(idx) => {
                self.scan_expression_for_spawns(idx.base)?;
                self.scan_expression_for_spawns(idx.index)?;
            }
            Expression::Array(arr) => {
                for elem in &arr.elements {
                    self.scan_expression_for_spawns(elem)?;
                }
            }
            Expression::If(if_expr) => {
                self.scan_expression_for_spawns(if_expr.condition)?;
                self.scan_for_spawn_blocks_expr(if_expr.then_branch)?;
                self.scan_else_branch_for_spawns(&if_expr.else_branch)?;
            }
            Expression::Block(block) => {
                self.scan_for_spawn_blocks_expr(block)?;
            }
            Expression::Grouped(grouped) => {
                self.scan_expression_for_spawns(grouped.inner)?;
            }
            Expression::Ternary(ternary) => {
                self.scan_expression_for_spawns(ternary.condition)?;
                self.scan_expression_for_spawns(ternary.true_expr)?;
                self.scan_expression_for_spawns(ternary.false_expr)?;
            }
            Expression::Elvis(elvis) => {
                self.scan_expression_for_spawns(elvis.left)?;
                self.scan_expression_for_spawns(elvis.right)?;
            }
            Expression::FallibleCast(cast) => {
                self.scan_expression_for_spawns(cast.expr)?;
            }
            Expression::ForceUnwrap(unwrap) => {
                self.scan_expression_for_spawns(unwrap.expr)?;
            }
            Expression::Catch(catch_expr) => {
                self.scan_expression_for_spawns(catch_expr.expr)?;
                self.scan_for_spawn_blocks_expr(catch_expr.handler)?;
            }
            Expression::Try(try_expr) => {
                self.scan_expression_for_spawns(try_expr.expr)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn scan_for_spawn_blocks_expr(
        &mut self,
        block: &crate::ast::BlockExpr<'_>,
    ) -> Result<(), CodegenError> {
        for stmt in &block.statements {
            self.scan_statement_for_spawns(stmt)?;
        }
        if let Some(tail) = block.tail {
            self.scan_expression_for_spawns(tail)?;
        }
        Ok(())
    }

    fn scan_else_branch_for_spawns(
        &mut self,
        else_branch: &Option<crate::ast::ElseExpr<'_>>,
    ) -> Result<(), CodegenError> {
        if let Some(branch) = else_branch {
            match branch {
                crate::ast::ElseExpr::ElseIf(elif) => {
                    self.scan_expression_for_spawns(elif.condition)?;
                    self.scan_for_spawn_blocks_expr(elif.then_branch)?;
                    self.scan_else_branch_for_spawns(&elif.else_branch)?;
                }
                crate::ast::ElseExpr::Else(block) => {
                    self.scan_for_spawn_blocks_expr(block)?;
                }
            }
        }
        Ok(())
    }

    fn collect_captured_vars_expr(&self, block: &crate::ast::BlockExpr<'_>) -> Vec<String> {
        let mut captured = Vec::new();
        let mut defined = std::collections::HashSet::new();
        self.collect_vars_in_block_expr(block, &mut captured, &mut defined);
        captured
    }

    fn collect_captured_vars_for_lambda(&self, lambda: &crate::ast::LambdaExpr<'_>) -> Vec<String> {
        let mut captured = Vec::new();
        let mut defined = std::collections::HashSet::new();

        // Lambda parameters are defined within the lambda scope
        for param in &lambda.params {
            let name = self.interner.resolve(&param.name.symbol).to_string();
            defined.insert(name);
        }

        // Collect from body (which is an Expression - typically a Block)
        self.collect_vars_in_expression(lambda.body, &mut captured, &defined);

        captured
    }

    fn collect_vars_in_block(
        &self,
        block: &crate::ast::BlockStmt<'_>,
        captured: &mut Vec<String>,
        defined: &mut std::collections::HashSet<String>,
    ) {
        for stmt in &block.statements {
            self.collect_vars_in_statement(stmt, captured, defined);
        }
    }

    fn collect_vars_in_block_expr(
        &self,
        block: &crate::ast::BlockExpr<'_>,
        captured: &mut Vec<String>,
        defined: &mut std::collections::HashSet<String>,
    ) {
        for stmt in &block.statements {
            self.collect_vars_in_statement(stmt, captured, defined);
        }
        if let Some(tail) = block.tail {
            self.collect_vars_in_expression(tail, captured, defined);
        }
    }

    fn collect_vars_in_statement(
        &self,
        stmt: &Statement<'_>,
        captured: &mut Vec<String>,
        defined: &mut std::collections::HashSet<String>,
    ) {
        match stmt {
            Statement::Var(var_stmt) => {
                if let Some(ref init) = var_stmt.init {
                    self.collect_vars_in_expression(init, captured, defined);
                }
                let name = self.interner.resolve(&var_stmt.name.symbol).to_string();
                defined.insert(name);
            }
            Statement::Expression(expr_stmt) => {
                self.collect_vars_in_expression(&expr_stmt.expr, captured, defined);
            }
            Statement::Assign(assign) => {
                self.collect_vars_in_expression(&assign.target, captured, defined);
                self.collect_vars_in_expression(&assign.value, captured, defined);
            }
            Statement::If(if_stmt) => {
                self.collect_vars_in_expression(&if_stmt.condition, captured, defined);
                self.collect_vars_in_block(&if_stmt.then_branch, captured, defined);
            }
            Statement::While(while_stmt) => {
                self.collect_vars_in_expression(&while_stmt.condition, captured, defined);
                self.collect_vars_in_block(&while_stmt.body, captured, defined);
            }
            Statement::For(for_stmt) => {
                self.collect_vars_in_expression(&for_stmt.iterable, captured, defined);
                let val_name = self.interner.resolve(&for_stmt.value.symbol).to_string();
                defined.insert(val_name);
                if let Some(ref idx) = for_stmt.index {
                    let idx_name = self.interner.resolve(&idx.symbol).to_string();
                    defined.insert(idx_name);
                }
                self.collect_vars_in_block(&for_stmt.body, captured, defined);
            }
            Statement::Return(ret) => {
                if let Some(ref value) = ret.value {
                    self.collect_vars_in_expression(value, captured, defined);
                }
            }
            Statement::Locked(locked_stmt) => {
                // Collect the mutex expression (e.g., the variable being locked)
                self.collect_vars_in_expression(&locked_stmt.mutex, captured, defined);
                // The binding is defined within the locked block scope
                let binding_name = self
                    .interner
                    .resolve(&locked_stmt.binding.symbol)
                    .to_string();
                let mut locked_defined = defined.clone();
                locked_defined.insert(binding_name);
                self.collect_vars_in_block(&locked_stmt.body, captured, &mut locked_defined);
            }
            _ => {}
        }
    }

    fn collect_vars_in_expression(
        &self,
        expr: &Expression<'_>,
        captured: &mut Vec<String>,
        defined: &std::collections::HashSet<String>,
    ) {
        match expr {
            Expression::Identifier(ident) => {
                let name = self.interner.resolve(&ident.ident.symbol).to_string();
                if !defined.contains(&name) && !captured.contains(&name) {
                    captured.push(name);
                }
            }
            Expression::Binary(bin) => {
                self.collect_vars_in_expression(bin.left, captured, defined);
                self.collect_vars_in_expression(bin.right, captured, defined);
            }
            Expression::Unary(un) => {
                self.collect_vars_in_expression(un.operand, captured, defined);
            }
            Expression::Call(call) => {
                self.collect_vars_in_expression(call.callee, captured, defined);
                for arg in &call.args {
                    self.collect_vars_in_expression(arg, captured, defined);
                }
            }
            Expression::MethodCall(method) => {
                self.collect_vars_in_expression(method.receiver, captured, defined);
                for arg in &method.args {
                    self.collect_vars_in_expression(arg, captured, defined);
                }
            }
            Expression::Index(idx) => {
                self.collect_vars_in_expression(idx.base, captured, defined);
                self.collect_vars_in_expression(idx.index, captured, defined);
            }
            Expression::Array(arr) => {
                for elem in &arr.elements {
                    self.collect_vars_in_expression(elem, captured, defined);
                }
            }
            Expression::Grouped(grouped) => {
                self.collect_vars_in_expression(grouped.inner, captured, defined);
            }
            Expression::Block(block) => {
                // Create a new defined set for block scope
                let mut block_defined = defined.clone();
                for stmt in &block.statements {
                    self.collect_vars_in_statement(stmt, captured, &mut block_defined);
                }
                if let Some(tail) = block.tail {
                    self.collect_vars_in_expression(tail, captured, &block_defined);
                }
            }
            Expression::Lambda(lambda) => {
                // Lambda creates its own scope - capture variables from outer scope
                let mut lambda_defined = defined.clone();
                for param in &lambda.params {
                    let name = self.interner.resolve(&param.name.symbol).to_string();
                    lambda_defined.insert(name);
                }
                self.collect_vars_in_expression(lambda.body, captured, &lambda_defined);
            }
            Expression::Ternary(ternary) => {
                self.collect_vars_in_expression(ternary.condition, captured, defined);
                self.collect_vars_in_expression(ternary.true_expr, captured, defined);
                self.collect_vars_in_expression(ternary.false_expr, captured, defined);
            }
            Expression::Elvis(elvis) => {
                self.collect_vars_in_expression(elvis.left, captured, defined);
                self.collect_vars_in_expression(elvis.right, captured, defined);
            }
            Expression::FallibleCast(cast) => {
                self.collect_vars_in_expression(cast.expr, captured, defined);
            }
            Expression::ForceUnwrap(unwrap) => {
                self.collect_vars_in_expression(unwrap.expr, captured, defined);
            }
            _ => {}
        }
    }

    fn find_captured_var_heap_types(
        &self,
        block: &crate::ast::BlockExpr<'_>,
        captured_vars: &[String],
    ) -> HashMap<String, HeapType> {
        let mut result = HashMap::new();
        let targets: HashSet<&str> = captured_vars.iter().map(|s| s.as_str()).collect();
        self.find_ident_types_in_block_expr(block, &targets, &mut result);
        result
    }

    fn find_ident_types_in_block_expr(
        &self,
        block: &crate::ast::BlockExpr<'_>,
        targets: &HashSet<&str>,
        result: &mut HashMap<String, HeapType>,
    ) {
        for stmt in &block.statements {
            self.find_ident_types_in_stmt(stmt, targets, result);
            if result.len() == targets.len() {
                return;
            }
        }
    }

    fn find_ident_types_in_stmt(
        &self,
        stmt: &Statement<'_>,
        targets: &HashSet<&str>,
        result: &mut HashMap<String, HeapType>,
    ) {
        match stmt {
            Statement::Expression(expr_stmt) => {
                self.find_ident_types_in_expr(&expr_stmt.expr, targets, result);
            }
            Statement::Var(var_stmt) => {
                if let Some(init) = &var_stmt.init {
                    self.find_ident_types_in_expr(init, targets, result);
                }
            }
            Statement::Assign(assign) => {
                self.find_ident_types_in_expr(&assign.target, targets, result);
                self.find_ident_types_in_expr(&assign.value, targets, result);
            }
            Statement::If(if_stmt) => {
                self.find_ident_types_in_expr(&if_stmt.condition, targets, result);
                for s in &if_stmt.then_branch.statements {
                    self.find_ident_types_in_stmt(s, targets, result);
                }
                if let Some(else_branch) = &if_stmt.else_branch {
                    match else_branch {
                        crate::ast::ElseBranch::ElseIf(elif) => {
                            self.find_ident_types_in_stmt(
                                &Statement::If(*elif.clone()),
                                targets,
                                result,
                            );
                        }
                        crate::ast::ElseBranch::Else(block) => {
                            for s in &block.statements {
                                self.find_ident_types_in_stmt(s, targets, result);
                            }
                        }
                    }
                }
            }
            Statement::While(while_stmt) => {
                self.find_ident_types_in_expr(&while_stmt.condition, targets, result);
                for s in &while_stmt.body.statements {
                    self.find_ident_types_in_stmt(s, targets, result);
                }
            }
            Statement::For(for_stmt) => {
                self.find_ident_types_in_expr(&for_stmt.iterable, targets, result);
                for s in &for_stmt.body.statements {
                    self.find_ident_types_in_stmt(s, targets, result);
                }
            }
            Statement::Return(ret) => {
                if let Some(val) = &ret.value {
                    self.find_ident_types_in_expr(val, targets, result);
                }
            }
            _ => {}
        }
    }

    fn find_ident_types_in_expr(
        &self,
        expr: &Expression<'_>,
        targets: &HashSet<&str>,
        result: &mut HashMap<String, HeapType>,
    ) {
        match expr {
            Expression::Identifier(ident_expr) => {
                let name = self.interner.resolve(&ident_expr.ident.symbol);
                if targets.contains(name) && !result.contains_key(name) {
                    if let Some(ty) = self.annotations.get_type(ident_expr.span) {
                        let resolved = ty.resolve();
                        if let Some(ht) = heap_type_from_type(&resolved, self.interner) {
                            result.insert(name.to_string(), ht);
                        }
                    }
                }
            }
            Expression::Call(call) => {
                self.find_ident_types_in_expr(call.callee, targets, result);
                for arg in &call.args {
                    self.find_ident_types_in_expr(arg, targets, result);
                }
            }
            Expression::MethodCall(mc) => {
                self.find_ident_types_in_expr(mc.receiver, targets, result);
                for arg in &mc.args {
                    self.find_ident_types_in_expr(arg, targets, result);
                }
            }
            Expression::Binary(bin) => {
                self.find_ident_types_in_expr(bin.left, targets, result);
                self.find_ident_types_in_expr(bin.right, targets, result);
            }
            Expression::Field(field) => {
                self.find_ident_types_in_expr(field.base, targets, result);
            }
            Expression::Index(idx) => {
                self.find_ident_types_in_expr(idx.base, targets, result);
                self.find_ident_types_in_expr(idx.index, targets, result);
            }
            Expression::ForceUnwrap(unwrap) => {
                self.find_ident_types_in_expr(unwrap.expr, targets, result);
            }
            _ => {}
        }
    }
}
