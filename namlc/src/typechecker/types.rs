//!
//! Internal Type Representation
//!
//! This module defines the internal type representation used during type
//! checking. Unlike the AST NamlType, this representation supports:
//!
//! - Type variables for inference (TypeVar)
//! - Resolved named types with full definitions
//! - Substitution during unification
//!
//! The type checker converts AST types to these internal types, performs
//! inference and checking, then can convert back for error messages.
//!

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use lasso::Spur;

pub type TypeId = u32;

#[derive(Debug, Clone, PartialEq)]
pub struct TypeParam {
    pub name: Spur,
    pub bounds: Vec<Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Uint,
    Float,
    Bool,
    String,
    Bytes,
    Unit,

    Array(Box<Type>),
    FixedArray(Box<Type>, usize),
    Option(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Channel(Box<Type>),

    Struct(StructType),
    Enum(EnumType),
    Interface(InterfaceType),
    Exception(Spur),

    // Built-in stack frame type for exception stack traces
    StackFrame,

    Function(FunctionType),

    TypeVar(TypeVarRef),

    Generic(Spur, Vec<Type>),

    Error,
    Never,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionType {
    pub params: Vec<Type>,
    pub returns: Box<Type>,
    pub throws: Vec<Type>,
    pub is_variadic: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructType {
    pub name: Spur,
    pub fields: Vec<FieldType>,
    pub type_params: Vec<TypeParam>,
    pub type_args: Vec<Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldType {
    pub name: Spur,
    pub ty: Type,
    pub is_public: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumType {
    pub name: Spur,
    pub variants: Vec<VariantType>,
    pub type_params: Vec<TypeParam>,
    pub type_args: Vec<Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VariantType {
    pub name: Spur,
    pub fields: Option<Vec<Type>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceType {
    pub name: Spur,
    pub methods: Vec<MethodType>,
    pub type_params: Vec<TypeParam>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodType {
    pub name: Spur,
    pub params: Vec<Type>,
    pub returns: Type,
    pub throws: Vec<Type>,
}

#[derive(Clone)]
pub struct TypeVarRef {
    pub id: TypeId,
    pub inner: Rc<RefCell<TypeVarInner>>,
}

impl PartialEq for TypeVarRef {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl fmt::Debug for TypeVarRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "?{}", self.id)
    }
}

#[derive(Debug, Clone)]
pub enum TypeVarInner {
    Unbound,
    Bound(Type),
}

impl TypeVarRef {
    pub fn new(id: TypeId) -> Self {
        Self {
            id,
            inner: Rc::new(RefCell::new(TypeVarInner::Unbound)),
        }
    }

    pub fn bind(&self, ty: Type) {
        *self.inner.borrow_mut() = TypeVarInner::Bound(ty);
    }

    pub fn is_bound(&self) -> bool {
        matches!(*self.inner.borrow(), TypeVarInner::Bound(_))
    }

    pub fn get_bound(&self) -> Option<Type> {
        match &*self.inner.borrow() {
            TypeVarInner::Bound(ty) => Some(ty.clone()),
            TypeVarInner::Unbound => None,
        }
    }
}

impl Type {
    pub fn array(elem: Type) -> Self {
        Type::Array(Box::new(elem))
    }

    pub fn option(inner: Type) -> Self {
        Type::Option(Box::new(inner))
    }

    pub fn map(key: Type, value: Type) -> Self {
        Type::Map(Box::new(key), Box::new(value))
    }

    pub fn channel(inner: Type) -> Self {
        Type::Channel(Box::new(inner))
    }

    pub fn function(params: Vec<Type>, returns: Type) -> Self {
        Type::Function(FunctionType {
            params,
            returns: Box::new(returns),
            throws: vec![],
            is_variadic: false,
        })
    }

    pub fn is_numeric(&self) -> bool {
        matches!(self, Type::Int | Type::Uint | Type::Float)
    }

    pub fn is_integer(&self) -> bool {
        matches!(self, Type::Int | Type::Uint)
    }

    pub fn is_comparable(&self) -> bool {
        matches!(
            self,
            Type::Int
                | Type::Uint
                | Type::Float
                | Type::Bool
                | Type::String
                | Type::Bytes
        )
    }

    pub fn resolve(&self) -> Type {
        match self {
            Type::TypeVar(var) => {
                if let Some(bound) = var.get_bound() {
                    bound.resolve()
                } else {
                    self.clone()
                }
            }
            Type::Array(elem) => Type::Array(Box::new(elem.resolve())),
            Type::FixedArray(elem, n) => Type::FixedArray(Box::new(elem.resolve()), *n),
            Type::Option(inner) => Type::Option(Box::new(inner.resolve())),
            Type::Map(k, v) => Type::Map(Box::new(k.resolve()), Box::new(v.resolve())),
            Type::Channel(inner) => Type::Channel(Box::new(inner.resolve())),
            Type::Function(f) => Type::Function(FunctionType {
                params: f.params.iter().map(|p| p.resolve()).collect(),
                returns: Box::new(f.returns.resolve()),
                throws: f.throws.iter().map(|t| t.resolve()).collect(),
                is_variadic: f.is_variadic,
            }),
            _ => self.clone(),
        }
    }

    pub fn contains_var(&self, var_id: TypeId) -> bool {
        match self {
            Type::TypeVar(v) => {
                if v.id == var_id {
                    return true;
                }
                if let Some(bound) = v.get_bound() {
                    return bound.contains_var(var_id);
                }
                false
            }
            Type::Array(elem) | Type::FixedArray(elem, _) => elem.contains_var(var_id),
            Type::Option(inner) | Type::Channel(inner) => inner.contains_var(var_id),
            Type::Map(k, v) => k.contains_var(var_id) || v.contains_var(var_id),
            Type::Function(f) => {
                f.params.iter().any(|p| p.contains_var(var_id))
                    || f.returns.contains_var(var_id)
                    || f.throws.iter().any(|t| t.contains_var(var_id))
            }
            Type::Generic(_, args) => args.iter().any(|a| a.contains_var(var_id)),
            _ => false,
        }
    }

    pub fn substitute(&self, substitutions: &HashMap<Spur, Type>) -> Type {
        match self {
            Type::Generic(name, args) if args.is_empty() => {
                if let Some(ty) = substitutions.get(name) {
                    ty.clone()
                } else {
                    self.clone()
                }
            }
            Type::Generic(name, args) => {
                let new_args = args.iter().map(|a| a.substitute(substitutions)).collect();
                Type::Generic(*name, new_args)
            }
            Type::Array(elem) => Type::Array(Box::new(elem.substitute(substitutions))),
            Type::FixedArray(elem, n) => {
                Type::FixedArray(Box::new(elem.substitute(substitutions)), *n)
            }
            Type::Option(inner) => Type::Option(Box::new(inner.substitute(substitutions))),
            Type::Map(k, v) => Type::Map(
                Box::new(k.substitute(substitutions)),
                Box::new(v.substitute(substitutions)),
            ),
            Type::Channel(inner) => Type::Channel(Box::new(inner.substitute(substitutions))),
            Type::Function(f) => Type::Function(FunctionType {
                params: f.params.iter().map(|p| p.substitute(substitutions)).collect(),
                returns: Box::new(f.returns.substitute(substitutions)),
                throws: f.throws.iter().map(|t| t.substitute(substitutions)).collect(),
                is_variadic: f.is_variadic,
            }),
            _ => self.clone(),
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Int => write!(f, "int"),
            Type::Uint => write!(f, "uint"),
            Type::Float => write!(f, "float"),
            Type::Bool => write!(f, "bool"),
            Type::String => write!(f, "string"),
            Type::Bytes => write!(f, "bytes"),
            Type::Unit => write!(f, "()"),
            Type::Array(elem) => write!(f, "[{}]", elem),
            Type::FixedArray(elem, n) => write!(f, "[{}; {}]", elem, n),
            Type::Option(inner) => write!(f, "option<{}>", inner),
            Type::Map(k, v) => write!(f, "map<{}, {}>", k, v),
            Type::Channel(inner) => write!(f, "channel<{}>", inner),
            Type::Struct(s) => write!(f, "struct:{:?}", s.name),
            Type::Enum(e) => write!(f, "enum:{:?}", e.name),
            Type::Interface(i) => write!(f, "interface:{:?}", i.name),
            Type::Exception(name) => write!(f, "exception:{:?}", name),
            Type::StackFrame => write!(f, "stack_frame"),
            Type::Function(func) => {
                write!(f, "fn(")?;
                for (i, p) in func.params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}", func.returns)
            }
            Type::TypeVar(v) => write!(f, "?{}", v.id),
            Type::Generic(name, args) => {
                write!(f, "{:?}", name)?;
                if !args.is_empty() {
                    write!(f, "<")?;
                    for (i, a) in args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", a)?;
                    }
                    write!(f, ">")?;
                }
                Ok(())
            }
            Type::Error => write!(f, "<error>"),
            Type::Never => write!(f, "never"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_var_binding() {
        let var = TypeVarRef::new(0);
        assert!(!var.is_bound());
        var.bind(Type::Int);
        assert!(var.is_bound());
        assert_eq!(var.get_bound(), Some(Type::Int));
    }

    #[test]
    fn test_type_resolve() {
        let var = TypeVarRef::new(0);
        var.bind(Type::Int);
        let ty = Type::TypeVar(var);
        assert_eq!(ty.resolve(), Type::Int);
    }

    #[test]
    fn test_contains_var() {
        let var = TypeVarRef::new(42);
        let ty = Type::Array(Box::new(Type::TypeVar(var)));
        assert!(ty.contains_var(42));
        assert!(!ty.contains_var(0));
    }
}
