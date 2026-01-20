///
/// Type Unification
///
/// This module implements the unification algorithm for type inference.
/// Unification determines if two types can be made equal by binding type
/// variables to concrete types.
///
/// The algorithm:
/// 1. Resolve any type variables to their bound types
/// 2. If both types are identical, succeed
/// 3. If one is a type variable, bind it to the other (occurs check)
/// 4. If both are composite types, recursively unify components
/// 5. Otherwise, fail with a type mismatch error
///
/// The occurs check prevents infinite types like `?0 = [?0]`.
///

use crate::source::Span;

use super::error::{TypeError, TypeResult};
use super::types::{Type, TypeVarRef};

pub fn unify(a: &Type, b: &Type, span: Span) -> TypeResult<()> {
    let a = a.resolve();
    let b = b.resolve();

    match (&a, &b) {
        (Type::Error, _) | (_, Type::Error) => Ok(()),
        (Type::Never, _) | (_, Type::Never) => Ok(()),

        (Type::Int, Type::Int)
        | (Type::Uint, Type::Uint)
        | (Type::Float, Type::Float)
        | (Type::Bool, Type::Bool)
        | (Type::String, Type::String)
        | (Type::Bytes, Type::Bytes)
        | (Type::Unit, Type::Unit) => Ok(()),

        (Type::TypeVar(var), other) | (other, Type::TypeVar(var)) => {
            if let Type::TypeVar(other_var) = other {
                if var.id == other_var.id {
                    return Ok(());
                }
            }

            if other.contains_var(var.id) {
                return Err(TypeError::Custom {
                    message: format!("infinite type: ?{} = {}", var.id, other),
                    span,
                });
            }

            var.bind(other.clone());
            Ok(())
        }

        (Type::Array(a_elem), Type::Array(b_elem)) => {
            unify(a_elem, b_elem, span)
        }

        (Type::FixedArray(a_elem, a_size), Type::FixedArray(b_elem, b_size)) => {
            if a_size != b_size {
                return Err(TypeError::type_mismatch(
                    format!("[_; {}]", a_size),
                    format!("[_; {}]", b_size),
                    span,
                ));
            }
            unify(a_elem, b_elem, span)
        }

        (Type::Option(a_inner), Type::Option(b_inner)) => {
            unify(a_inner, b_inner, span)
        }

        (Type::Map(a_key, a_val), Type::Map(b_key, b_val)) => {
            unify(a_key, b_key, span)?;
            unify(a_val, b_val, span)
        }

        (Type::Channel(a_inner), Type::Channel(b_inner)) => {
            unify(a_inner, b_inner, span)
        }

        (Type::Promise(a_inner), Type::Promise(b_inner)) => {
            unify(a_inner, b_inner, span)
        }

        (Type::Function(a_fn), Type::Function(b_fn)) => {
            if a_fn.params.len() != b_fn.params.len() {
                return Err(TypeError::type_mismatch(
                    format!("fn with {} params", a_fn.params.len()),
                    format!("fn with {} params", b_fn.params.len()),
                    span,
                ));
            }

            for (a_param, b_param) in a_fn.params.iter().zip(b_fn.params.iter()) {
                unify(a_param, b_param, span)?;
            }

            unify(&a_fn.returns, &b_fn.returns, span)?;

            match (&a_fn.throws, &b_fn.throws) {
                (Some(a_throws), Some(b_throws)) => unify(a_throws, b_throws, span)?,
                (None, None) => {}
                _ => {
                    return Err(TypeError::type_mismatch(
                        if a_fn.throws.is_some() {
                            "throwing function"
                        } else {
                            "non-throwing function"
                        },
                        if b_fn.throws.is_some() {
                            "throwing function"
                        } else {
                            "non-throwing function"
                        },
                        span,
                    ));
                }
            }

            Ok(())
        }

        (Type::Struct(a_struct), Type::Struct(b_struct)) => {
            if a_struct.name != b_struct.name {
                return Err(TypeError::type_mismatch(
                    format!("{:?}", a_struct.name),
                    format!("{:?}", b_struct.name),
                    span,
                ));
            }

            if a_struct.type_args.len() != b_struct.type_args.len() {
                return Err(TypeError::WrongTypeArgCount {
                    expected: a_struct.type_params.len(),
                    found: b_struct.type_args.len(),
                    span,
                });
            }

            for (a_arg, b_arg) in a_struct.type_args.iter().zip(b_struct.type_args.iter()) {
                unify(a_arg, b_arg, span)?;
            }

            Ok(())
        }

        (Type::Enum(a_enum), Type::Enum(b_enum)) => {
            if a_enum.name != b_enum.name {
                return Err(TypeError::type_mismatch(
                    format!("{:?}", a_enum.name),
                    format!("{:?}", b_enum.name),
                    span,
                ));
            }

            for (a_arg, b_arg) in a_enum.type_args.iter().zip(b_enum.type_args.iter()) {
                unify(a_arg, b_arg, span)?;
            }

            Ok(())
        }

        (Type::Generic(a_name, a_args), Type::Generic(b_name, b_args)) => {
            if a_name != b_name {
                return Err(TypeError::type_mismatch(
                    format!("{:?}", a_name),
                    format!("{:?}", b_name),
                    span,
                ));
            }

            if a_args.len() != b_args.len() {
                return Err(TypeError::WrongTypeArgCount {
                    expected: a_args.len(),
                    found: b_args.len(),
                    span,
                });
            }

            for (a_arg, b_arg) in a_args.iter().zip(b_args.iter()) {
                unify(a_arg, b_arg, span)?;
            }

            Ok(())
        }

        // Allow Type::Struct to unify with Type::Generic of the same name
        (Type::Struct(s), Type::Generic(name, args)) | (Type::Generic(name, args), Type::Struct(s)) => {
            if s.name != *name {
                return Err(TypeError::type_mismatch(
                    format!("struct:{:?}", s.name),
                    format!("{:?}", name),
                    span,
                ));
            }

            // Unify type arguments
            if s.type_args.len() != args.len() && !s.type_params.is_empty() {
                return Err(TypeError::WrongTypeArgCount {
                    expected: s.type_params.len(),
                    found: args.len(),
                    span,
                });
            }

            for (s_arg, g_arg) in s.type_args.iter().zip(args.iter()) {
                unify(s_arg, g_arg, span)?;
            }

            Ok(())
        }

        _ => Err(TypeError::type_mismatch(a.to_string(), b.to_string(), span)),
    }
}

pub fn unify_with_expected(found: &Type, expected: &Type, span: Span) -> TypeResult<()> {
    unify(found, expected, span)
}

pub fn fresh_type_var(id: &mut u32) -> Type {
    let var = TypeVarRef::new(*id);
    *id += 1;
    Type::TypeVar(var)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::Span;

    #[test]
    fn test_unify_same_primitives() {
        assert!(unify(&Type::Int, &Type::Int, Span::dummy()).is_ok());
        assert!(unify(&Type::String, &Type::String, Span::dummy()).is_ok());
    }

    #[test]
    fn test_unify_different_primitives_fails() {
        assert!(unify(&Type::Int, &Type::String, Span::dummy()).is_err());
    }

    #[test]
    fn test_unify_type_var() {
        let var = TypeVarRef::new(0);
        let ty_var = Type::TypeVar(var.clone());

        assert!(unify(&ty_var, &Type::Int, Span::dummy()).is_ok());
        assert_eq!(var.get_bound(), Some(Type::Int));
    }

    #[test]
    fn test_unify_two_type_vars() {
        let var1 = TypeVarRef::new(0);
        let var2 = TypeVarRef::new(1);
        let ty1 = Type::TypeVar(var1.clone());
        let ty2 = Type::TypeVar(var2.clone());

        assert!(unify(&ty1, &ty2, Span::dummy()).is_ok());
        assert!(var1.is_bound() || var2.is_bound());
    }

    #[test]
    fn test_unify_arrays() {
        let a = Type::Array(Box::new(Type::Int));
        let b = Type::Array(Box::new(Type::Int));
        assert!(unify(&a, &b, Span::dummy()).is_ok());

        let c = Type::Array(Box::new(Type::String));
        assert!(unify(&a, &c, Span::dummy()).is_err());
    }

    #[test]
    fn test_unify_array_with_var() {
        let var = TypeVarRef::new(0);
        let ty_var = Type::TypeVar(var.clone());
        let arr = Type::Array(Box::new(ty_var));

        let expected = Type::Array(Box::new(Type::Int));
        assert!(unify(&arr, &expected, Span::dummy()).is_ok());
        assert_eq!(var.get_bound(), Some(Type::Int));
    }

    #[test]
    fn test_occurs_check() {
        let var = TypeVarRef::new(0);
        let ty_var = Type::TypeVar(var.clone());
        let arr = Type::Array(Box::new(ty_var.clone()));

        let result = unify(&ty_var, &arr, Span::dummy());
        assert!(result.is_err());
    }

    #[test]
    fn test_unify_functions() {
        let f1 = Type::function(vec![Type::Int], Type::Bool);
        let f2 = Type::function(vec![Type::Int], Type::Bool);
        assert!(unify(&f1, &f2, Span::dummy()).is_ok());

        let f3 = Type::function(vec![Type::String], Type::Bool);
        assert!(unify(&f1, &f3, Span::dummy()).is_err());
    }
}
