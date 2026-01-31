use cranelift::prelude::*;

use crate::codegen::cranelift::literal::compile_literal;
use crate::codegen::cranelift::CompileContext;
use crate::codegen::CodegenError;

pub fn compile_pattern_match(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    pattern: &crate::ast::Pattern<'_>,
    scrutinee: Value,
) -> Result<Value, CodegenError> {
    use crate::ast::Pattern;

    match pattern {
        Pattern::Literal(lit) => {
            let lit_val = compile_literal(ctx, builder, &lit.value)?;
            Ok(builder.ins().icmp(IntCC::Equal, scrutinee, lit_val))
        }

        Pattern::Identifier(ident) => {
            let name = ctx.interner.resolve(&ident.ident.symbol).to_string();
            for enum_def in ctx.enum_defs.values() {
                if let Some(variant) = enum_def.variants.iter().find(|v| v.name == name) {
                    let tag = builder.ins().load(
                        cranelift::prelude::types::I64,
                        MemFlags::new(),
                        scrutinee,
                        0,
                    );
                    let expected_tag = builder
                        .ins()
                        .iconst(cranelift::prelude::types::I64, variant.tag as i64);
                    return Ok(builder.ins().icmp(IntCC::Equal, tag, expected_tag));
                }
            }
            Ok(builder.ins().iconst(cranelift::prelude::types::I8, 1))
        }

        Pattern::Variant(variant) => {
            if variant.path.is_empty() {
                return Err(CodegenError::JitCompile("Empty variant path".to_string()));
            }
            let (enum_name, variant_name) = if variant.path.len() == 1 {
                let var_name = ctx.interner.resolve(&variant.path[0].symbol).to_string();
                let mut found = None;
                for (e_name, enum_def) in ctx.enum_defs.iter() {
                    if enum_def.variants.iter().any(|v| v.name == var_name) {
                        found = Some((e_name.clone(), var_name.clone()));
                        break;
                    }
                }

                match found {
                    Some(pair) => pair,
                    None => {
                        return Err(CodegenError::JitCompile(format!(
                            "Unknown variant: {}",
                            var_name
                        )));
                    }
                }
            } else {
                // Qualified path
                let enum_name = ctx.interner.resolve(&variant.path[0].symbol).to_string();
                let variant_name = ctx
                    .interner
                    .resolve(&variant.path.last().unwrap().symbol)
                    .to_string();
                (enum_name, variant_name)
            };

            if let Some(enum_def) = ctx.enum_defs.get(&enum_name)
                && let Some(var_def) = enum_def.variants.iter().find(|v| v.name == variant_name)
            {
                let tag = builder.ins().load(
                    cranelift::prelude::types::I64,
                    MemFlags::new(),
                    scrutinee,
                    0,
                );
                let expected_tag = builder
                    .ins()
                    .iconst(cranelift::prelude::types::I64, var_def.tag as i64);
                return Ok(builder.ins().icmp(IntCC::Equal, tag, expected_tag));
            }

            Err(CodegenError::JitCompile(format!(
                "Unknown enum variant: {}::{}",
                enum_name, variant_name
            )))
        }

        Pattern::Wildcard(_) => Ok(builder.ins().iconst(cranelift::prelude::types::I8, 1)),

        Pattern::_Phantom(_) => Ok(builder.ins().iconst(cranelift::prelude::types::I8, 0)),
    }
}
