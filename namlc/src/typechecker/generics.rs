//!
//! Generics Module - Type Substitution and Bound Checking
//!
//! This module handles generic type operations:
//!
//! - Building substitution maps from type params to concrete types
//! - Finding methods on type parameters by checking their bounds
//! - Instantiating generic functions with fresh type variables
//! - Checking that concrete types satisfy their bounds
//!
//! These operations enable proper generic type inference and trait method
//! resolution for code like `T: Comparable<T>` where `T.compare()` is called.
//!

use std::collections::HashMap;

use lasso::Spur;

use super::env::TypeEnv;
use super::symbols::{InterfaceDef, SymbolTable, TypeDef};
use super::types::{MethodType, Type, TypeParam};
use super::unify::fresh_type_var;

pub fn build_substitution(params: &[TypeParam], args: &[Type]) -> HashMap<Spur, Type> {
    params
        .iter()
        .zip(args.iter())
        .map(|(param, arg)| (param.name, arg.clone()))
        .collect()
}

pub fn find_method_from_bounds(
    param_name: Spur,
    method_name: Spur,
    env: &TypeEnv,
    symbols: &SymbolTable,
) -> Option<MethodType> {
    let bounds = env.get_type_param_bounds(param_name)?;

    for bound in bounds {
        if let Some(method) = find_method_in_bound(bound, method_name, symbols) {
            return Some(method);
        }
    }

    None
}

fn find_method_in_bound(bound: &Type, method_name: Spur, symbols: &SymbolTable) -> Option<MethodType> {
    match bound {
        Type::Generic(interface_name, _type_args) => {
            if let Some(TypeDef::Interface(interface)) = symbols.get_type(*interface_name) {
                return find_method_in_interface(interface, method_name);
            }
        }
        Type::Interface(interface_type) => {
            for method in &interface_type.methods {
                if method.name == method_name {
                    return Some(method.clone());
                }
            }
        }
        _ => {}
    }
    None
}

fn find_method_in_interface(interface: &InterfaceDef, method_name: Spur) -> Option<MethodType> {
    for method_def in &interface.methods {
        if method_def.name == method_name {
            return Some(MethodType {
                name: method_def.name,
                params: method_def.params.iter().map(|(_, ty)| ty.clone()).collect(),
                returns: method_def.return_ty.clone(),
                throws: method_def.throws.clone(),
            });
        }
    }
    None
}

pub fn instantiate_generic_function(
    type_params: &[TypeParam],
    next_var_id: &mut u32,
) -> (Vec<Type>, HashMap<Spur, Type>) {
    let mut fresh_vars = Vec::new();
    let mut substitution = HashMap::new();

    for param in type_params {
        let var = fresh_type_var(next_var_id);
        fresh_vars.push(var.clone());
        substitution.insert(param.name, var);
    }

    (fresh_vars, substitution)
}

pub fn check_bounds_satisfied(
    concrete: &Type,
    bounds: &[Type],
    symbols: &SymbolTable,
) -> Result<(), String> {
    for bound in bounds {
        if !type_satisfies_bound(concrete, bound, symbols) {
            return Err(format!(
                "type {} does not satisfy bound {}",
                concrete, bound
            ));
        }
    }
    Ok(())
}

fn type_satisfies_bound(concrete: &Type, bound: &Type, symbols: &SymbolTable) -> bool {
    match bound {
        Type::Generic(interface_name, _) => {
            check_type_implements_interface(concrete, *interface_name, symbols)
        }
        Type::Interface(interface_type) => {
            check_type_implements_interface(concrete, interface_type.name, symbols)
        }
        _ => false,
    }
}

fn check_type_implements_interface(ty: &Type, interface_name: Spur, symbols: &SymbolTable) -> bool {
    match ty {
        Type::Struct(struct_type) => {
            if let Some(TypeDef::Struct(struct_def)) = symbols.get_type(struct_type.name) {
                for impl_ty in &struct_def.implements {
                    let impl_name = match impl_ty {
                        Type::Generic(name, _) => Some(*name),
                        Type::Interface(i) => Some(i.name),
                        _ => None,
                    };
                    if impl_name == Some(interface_name) {
                        return true;
                    }
                }
            }
        }
        Type::Generic(name, _) => {
            if let Some(TypeDef::Struct(struct_def)) = symbols.get_type(*name) {
                for impl_ty in &struct_def.implements {
                    let impl_name = match impl_ty {
                        Type::Generic(name, _) => Some(*name),
                        Type::Interface(i) => Some(i.name),
                        _ => None,
                    };
                    if impl_name == Some(interface_name) {
                        return true;
                    }
                }
            }
        }
        _ => {}
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::Span;
    use crate::typechecker::symbols::{InterfaceMethodDef, StructDef};
    use lasso::Rodeo;

    #[test]
    fn test_build_substitution() {
        let mut rodeo = Rodeo::default();
        let t = rodeo.get_or_intern("T");
        let u = rodeo.get_or_intern("U");

        let params = vec![
            TypeParam {
                name: t,
                bounds: vec![],
            },
            TypeParam {
                name: u,
                bounds: vec![],
            },
        ];
        let args = vec![Type::Int, Type::String];

        let subst = build_substitution(&params, &args);

        assert_eq!(subst.get(&t), Some(&Type::Int));
        assert_eq!(subst.get(&u), Some(&Type::String));
    }

    #[test]
    fn test_instantiate_generic_function() {
        let mut rodeo = Rodeo::default();
        let t = rodeo.get_or_intern("T");

        let params = vec![TypeParam {
            name: t,
            bounds: vec![],
        }];
        let mut next_var_id = 0;

        let (vars, subst) = instantiate_generic_function(&params, &mut next_var_id);

        assert_eq!(vars.len(), 1);
        assert!(matches!(vars[0], Type::TypeVar(_)));
        assert!(subst.contains_key(&t));
        assert_eq!(next_var_id, 1);
    }

    #[test]
    fn test_check_type_implements_interface() {
        let mut rodeo = Rodeo::default();
        let my_struct = rodeo.get_or_intern("MyStruct");
        let comparable = rodeo.get_or_intern("Comparable");
        let compare = rodeo.get_or_intern("compare");

        let mut symbols = SymbolTable::new();

        symbols.define_type(
            comparable,
            TypeDef::Interface(crate::typechecker::symbols::InterfaceDef {
                name: comparable,
                type_params: vec![],
                extends: vec![],
                methods: vec![InterfaceMethodDef {
                    name: compare,
                    type_params: vec![],
                    params: vec![],
                    return_ty: Type::Int,
                    throws: None,
                }],
                is_public: true,
                span: Span::dummy(),
            }),
        );

        symbols.define_type(
            my_struct,
            TypeDef::Struct(StructDef {
                name: my_struct,
                type_params: vec![],
                fields: vec![],
                implements: vec![Type::Generic(comparable, vec![])],
                is_public: true,
                span: Span::dummy(),
            }),
        );

        let struct_type = crate::typechecker::types::StructType {
            name: my_struct,
            fields: vec![],
            type_params: vec![],
            type_args: vec![],
        };

        assert!(check_type_implements_interface(
            &Type::Struct(struct_type),
            comparable,
            &symbols
        ));
    }
}
