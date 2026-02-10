///
/// LSP Symbol Snapshot
///
/// Thread-safe snapshot of the compiler's SymbolTable extracted during
/// analysis. All Type references are converted to displayable strings
/// using the interner, making this struct Send+Sync safe for use
/// across async boundaries.
///
/// Also contains formatting helpers that convert compiler types to
/// human-readable strings during snapshot creation.
///

use std::collections::HashMap;

use lasso::Rodeo;

use namlc::source::Span;
use namlc::typechecker::symbols::{FunctionSig, MethodSig, ModuleNamespace, SymbolTable, TypeDef};
use namlc::typechecker::types::Type;

#[derive(Debug, Clone)]
pub struct LspFunctionSig {
    pub name: String,
    pub detail: String,
    pub span: Span,
    pub is_std: bool,
}

#[derive(Debug, Clone)]
pub struct LspField {
    pub name: String,
    pub type_str: String,
    pub is_public: bool,
}

#[derive(Debug, Clone)]
pub struct LspVariant {
    pub name: String,
    pub fields: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct LspMethodSig {
    pub name: String,
    pub detail: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum LspTypeDef {
    Struct {
        name: String,
        fields: Vec<LspField>,
        span: Span,
    },
    Enum {
        name: String,
        variants: Vec<LspVariant>,
        span: Span,
    },
    Interface {
        name: String,
        methods: Vec<LspInterfaceMethod>,
        span: Span,
    },
    Exception {
        name: String,
        fields: Vec<LspField>,
        span: Span,
    },
    TypeAlias {
        name: String,
        aliased_type: String,
        span: Span,
    },
}

impl LspTypeDef {
    pub fn name(&self) -> &str {
        match self {
            LspTypeDef::Struct { name, .. } => name,
            LspTypeDef::Enum { name, .. } => name,
            LspTypeDef::Interface { name, .. } => name,
            LspTypeDef::Exception { name, .. } => name,
            LspTypeDef::TypeAlias { name, .. } => name,
        }
    }

    pub fn span(&self) -> Span {
        match self {
            LspTypeDef::Struct { span, .. } => *span,
            LspTypeDef::Enum { span, .. } => *span,
            LspTypeDef::Interface { span, .. } => *span,
            LspTypeDef::Exception { span, .. } => *span,
            LspTypeDef::TypeAlias { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LspInterfaceMethod {
    pub name: String,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct LspModule {
    #[allow(dead_code)]
    pub name: String,
    pub functions: Vec<LspFunctionSig>,
    pub types: Vec<LspTypeDef>,
    pub submodules: HashMap<String, LspModule>,
}

#[derive(Debug, Clone)]
pub struct LspSymbols {
    pub root: LspModule,
    pub types: Vec<LspTypeDef>,
    pub functions: Vec<LspFunctionSig>,
    pub methods: HashMap<String, Vec<LspMethodSig>>,
}

pub fn snapshot_symbols(symbols: &SymbolTable, interner: &Rodeo) -> LspSymbols {
    let root = snapshot_module(&symbols.root, interner);

    let mut types = Vec::new();
    for (_spur, typedef) in symbols.all_types() {
        types.push(snapshot_type_def(typedef, interner));
    }

    let mut functions = Vec::new();
    for sig in symbols.root.all_functions() {
        let name = interner.resolve(&sig.name).to_string();
        let detail = format_function_sig(sig, interner);
        functions.push(LspFunctionSig {
            name,
            detail,
            span: sig.span,
            is_std: sig.module.as_ref().is_some_and(|m| m.starts_with("std")),
        });
    }

    let mut methods = HashMap::new();
    for (spur, _typedef) in symbols.all_types() {
        let type_name = interner.resolve(spur).to_string();
        if let Some(method_list) = symbols.get_methods(*spur) {
            let mut lsp_methods = Vec::new();
            for method in method_list {
                let mname = interner.resolve(&method.name).to_string();
                let detail = format_method_sig(method, interner);
                lsp_methods.push(LspMethodSig {
                    name: mname,
                    detail,
                    span: method.span,
                });
            }
            methods.insert(type_name, lsp_methods);
        }
    }

    LspSymbols { root, types, functions, methods }
}

fn snapshot_module(module: &ModuleNamespace, interner: &Rodeo) -> LspModule {
    let name = interner.resolve(&module.name).to_string();
    let mut functions = Vec::new();
    for sig in module.all_functions() {
        let fname = interner.resolve(&sig.name).to_string();
        let detail = format_function_sig(sig, interner);
        functions.push(LspFunctionSig {
            name: fname,
            detail,
            span: sig.span,
            is_std: sig.module.as_ref().is_some_and(|m| m.starts_with("std")),
        });
    }

    let mut types = Vec::new();
    for (_, typedef) in module.all_types() {
        types.push(snapshot_type_def(typedef, interner));
    }

    let mut submodules = HashMap::new();
    for (spur, submod) in module.all_submodules() {
        let sub_name = interner.resolve(spur).to_string();
        submodules.insert(sub_name.clone(), snapshot_module(submod, interner));
    }

    LspModule { name, functions, types, submodules }
}

fn snapshot_type_def(typedef: &TypeDef, interner: &Rodeo) -> LspTypeDef {
    match typedef {
        TypeDef::Struct(sdef) => {
            let name = interner.resolve(&sdef.name).to_string();
            let fields = sdef.fields.iter().map(|(fspur, fty, is_pub)| {
                LspField {
                    name: interner.resolve(fspur).to_string(),
                    type_str: format_type(fty, interner),
                    is_public: *is_pub,
                }
            }).collect();
            LspTypeDef::Struct { name, fields, span: sdef.span }
        }
        TypeDef::Enum(edef) => {
            let name = interner.resolve(&edef.name).to_string();
            let variants = edef.variants.iter().map(|(vspur, fields)| {
                LspVariant {
                    name: interner.resolve(vspur).to_string(),
                    fields: fields.as_ref().map(|fs| {
                        fs.iter().map(|t| format_type(t, interner)).collect()
                    }),
                }
            }).collect();
            LspTypeDef::Enum { name, variants, span: edef.span }
        }
        TypeDef::Interface(idef) => {
            let name = interner.resolve(&idef.name).to_string();
            let methods = idef.methods.iter().map(|m| {
                let mname = interner.resolve(&m.name).to_string();
                let mut detail = format!("fn {}(", mname);
                for (i, (pname, pty)) in m.params.iter().enumerate() {
                    if i > 0 { detail.push_str(", "); }
                    detail.push_str(interner.resolve(pname));
                    detail.push_str(": ");
                    detail.push_str(&format_type(pty, interner));
                }
                detail.push(')');
                let ret = format_type(&m.return_ty, interner);
                if ret != "()" {
                    detail.push_str(" -> ");
                    detail.push_str(&ret);
                }
                LspInterfaceMethod { name: mname, detail }
            }).collect();
            LspTypeDef::Interface { name, methods, span: idef.span }
        }
        TypeDef::Exception(edef) => {
            let name = interner.resolve(&edef.name).to_string();
            let fields = edef.fields.iter().map(|(fspur, fty)| {
                LspField {
                    name: interner.resolve(fspur).to_string(),
                    type_str: format_type(fty, interner),
                    is_public: true,
                }
            }).collect();
            LspTypeDef::Exception { name, fields, span: edef.span }
        }
        TypeDef::TypeAlias(adef) => {
            let name = interner.resolve(&adef.name).to_string();
            let aliased_type = format_type(&adef.aliased_type, interner);
            LspTypeDef::TypeAlias { name, aliased_type, span: adef.span }
        }
    }
}

fn format_function_sig(sig: &FunctionSig, interner: &Rodeo) -> String {
    let name = interner.resolve(&sig.name);
    let mut s = String::from("fn ");
    if !sig.type_params.is_empty() {
        s.push('<');
        for (i, tp) in sig.type_params.iter().enumerate() {
            if i > 0 { s.push_str(", "); }
            s.push_str(interner.resolve(&tp.name));
        }
        s.push('>');
    }
    s.push_str(name);
    s.push('(');
    for (i, (pname, pty)) in sig.params.iter().enumerate() {
        if i > 0 { s.push_str(", "); }
        s.push_str(interner.resolve(pname));
        s.push_str(": ");
        s.push_str(&format_type(pty, interner));
    }
    if sig.is_variadic {
        if !sig.params.is_empty() { s.push_str(", "); }
        s.push_str("...");
    }
    s.push(')');
    let ret = format_type(&sig.return_ty, interner);
    if ret != "()" {
        s.push_str(" -> ");
        s.push_str(&ret);
    }
    if !sig.throws.is_empty() {
        s.push_str(" throws ");
        for (i, t) in sig.throws.iter().enumerate() {
            if i > 0 { s.push_str(", "); }
            s.push_str(&format_type(t, interner));
        }
    }
    s
}

fn format_method_sig(sig: &MethodSig, interner: &Rodeo) -> String {
    let name = interner.resolve(&sig.name);
    let mut s = String::from("fn ");
    s.push_str(name);
    s.push('(');
    for (i, (pname, pty)) in sig.params.iter().enumerate() {
        if i > 0 { s.push_str(", "); }
        s.push_str(interner.resolve(pname));
        s.push_str(": ");
        s.push_str(&format_type(pty, interner));
    }
    s.push(')');
    let ret = format_type(&sig.return_ty, interner);
    if ret != "()" {
        s.push_str(" -> ");
        s.push_str(&ret);
    }
    s
}

fn format_type(ty: &Type, interner: &Rodeo) -> String {
    match ty {
        Type::Int => "int".to_string(),
        Type::Uint => "uint".to_string(),
        Type::Float => "float".to_string(),
        Type::Bool => "bool".to_string(),
        Type::String => "string".to_string(),
        Type::Bytes => "bytes".to_string(),
        Type::Unit => "()".to_string(),
        Type::Array(elem) => format!("[{}]", format_type(elem, interner)),
        Type::FixedArray(elem, n) => format!("[{}; {}]", format_type(elem, interner), n),
        Type::Option(inner) => format!("option<{}>", format_type(inner, interner)),
        Type::Map(k, v) => {
            format!("map<{}, {}>", format_type(k, interner), format_type(v, interner))
        }
        Type::Channel(inner) => format!("channel<{}>", format_type(inner, interner)),
        Type::Mutex(inner) => format!("mutex<{}>", format_type(inner, interner)),
        Type::Rwlock(inner) => format!("rwlock<{}>", format_type(inner, interner)),
        Type::Atomic(inner) => format!("atomic<{}>", format_type(inner, interner)),
        Type::Struct(s) => interner.resolve(&s.name).to_string(),
        Type::Enum(e) => interner.resolve(&e.name).to_string(),
        Type::Interface(i) => interner.resolve(&i.name).to_string(),
        Type::Exception(name) => interner.resolve(name).to_string(),
        Type::StackFrame => "stack_frame".to_string(),
        Type::Json => "json".to_string(),
        Type::Function(f) => {
            let mut s = "fn(".to_string();
            for (i, p) in f.params.iter().enumerate() {
                if i > 0 { s.push_str(", "); }
                s.push_str(&format_type(p, interner));
            }
            s.push_str(") -> ");
            s.push_str(&format_type(&f.returns, interner));
            s
        }
        Type::Generic(name, args) => {
            let mut s = interner.resolve(name).to_string();
            if !args.is_empty() {
                s.push('<');
                for (i, a) in args.iter().enumerate() {
                    if i > 0 { s.push_str(", "); }
                    s.push_str(&format_type(a, interner));
                }
                s.push('>');
            }
            s
        }
        Type::TypeVar(_) => "?".to_string(),
        Type::Error => "<error>".to_string(),
        Type::Never => "never".to_string(),
    }
}
