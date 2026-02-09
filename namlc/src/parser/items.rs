//!
//! Item Parser
//!
//! Parses top-level items using nom combinators.
//! Handles functions, structs, enums, interfaces, exceptions, imports, and extern.
//!

use nom::multi::separated_list0;

use crate::ast::*;
use crate::lexer::{Keyword, TokenKind};
use crate::source::Spanned;

use super::combinators::*;
use super::expressions::parse_block;
use super::input::TokenStream;
use super::statements::parse_statement;
use super::types::{parse_gt, parse_type};

pub fn parse_item<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
) -> PResult<'a, Item<'ast>> {
    let (input, platforms) = if check(TokenKind::Hash)(input) {
        parse_platforms_attr(input)?
    } else {
        (input, None)
    };

    let (input, is_public) = if check_keyword(Keyword::Pub)(input) {
        let (input, _) = keyword(Keyword::Pub)(input)?;
        (input, true)
    } else {
        (input, false)
    };

    match input.first().map(|t| t.kind) {
        Some(TokenKind::Keyword(Keyword::Fn)) => {
            parse_function_item(arena, input, is_public, platforms)
        }
        Some(TokenKind::Keyword(Keyword::Struct)) => parse_struct_item(input, is_public),
        Some(TokenKind::Keyword(Keyword::Enum)) => parse_enum_item(input, is_public),
        Some(TokenKind::Keyword(Keyword::Interface)) => parse_interface_item(input, is_public),
        Some(TokenKind::Keyword(Keyword::Exception)) => parse_exception_item(input, is_public),
        Some(TokenKind::Keyword(Keyword::Use)) => parse_use_item(input),
        Some(TokenKind::Keyword(Keyword::Extern)) => parse_extern_item(input),
        Some(TokenKind::Keyword(Keyword::Mod)) => parse_mod_item(arena, input, is_public),
        Some(TokenKind::Keyword(Keyword::Type)) => parse_type_alias_item(input, is_public),
        _ => parse_top_level_stmt(arena, input),
    }
}

fn parse_platforms_attr<'a>(input: TokenStream<'a>) -> PResult<'a, Option<Platforms>> {
    let (input, start) = token(TokenKind::Hash)(input)?;
    let (input, _) = token(TokenKind::LBracket)(input)?;
    let (input, _) = keyword(Keyword::Platforms)(input)?;
    let (input, _) = token(TokenKind::LParen)(input)?;

    let mut platforms = Vec::with_capacity(3);
    let mut input = input;

    loop {
        if check(TokenKind::RParen)(input) {
            break;
        }

        let platform = if check_keyword(Keyword::Native)(input) {
            let (new_input, _) = keyword(Keyword::Native)(input)?;
            input = new_input;
            Platform::Native
        } else if check_keyword(Keyword::Edge)(input) {
            let (new_input, _) = keyword(Keyword::Edge)(input)?;
            input = new_input;
            Platform::Edge
        } else if check_keyword(Keyword::Browser)(input) {
            let (new_input, _) = keyword(Keyword::Browser)(input)?;
            input = new_input;
            Platform::Browser
        } else {
            break;
        };

        platforms.push(platform);

        if check(TokenKind::Comma)(input) {
            let (new_input, _) = token(TokenKind::Comma)(input)?;
            input = new_input;
        }
    }

    let (input, end) = token(TokenKind::RParen)(input)?;
    let (input, _) = token(TokenKind::RBracket)(input)?;

    Ok((
        input,
        Some(Platforms {
            platforms,
            span: start.span.merge(end.span),
        }),
    ))
}

fn parse_function_item<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
    is_public: bool,
    platforms: Option<Platforms>,
) -> PResult<'a, Item<'ast>> {
    let (input, start) = keyword(Keyword::Fn)(input)?;

    let (input, receiver) = if check(TokenKind::LParen)(input) {
        let (input, recv) = parse_receiver(input)?;
        (input, Some(recv))
    } else {
        (input, None)
    };

    let (input, name) = ident(input)?;

    let (input, generics) = if check(TokenKind::Lt)(input) {
        parse_generic_params(input)?
    } else {
        (input, Vec::new())
    };

    let (input, _) = token(TokenKind::LParen)(input)?;
    let (input, params) = separated_list0(token(TokenKind::Comma), parse_parameter)(input)?;
    let (input, _) = token(TokenKind::RParen)(input)?;

    let (input, return_ty) = if check(TokenKind::Arrow)(input) {
        let (input, _) = token(TokenKind::Arrow)(input)?;
        let (input, ty) = parse_type(input)?;
        (input, Some(ty))
    } else {
        (input, None)
    };

    let (input, throws) = if check_keyword(Keyword::Throws)(input) {
        let (input, _) = keyword(Keyword::Throws)(input)?;
        if check(TokenKind::LBrace)(input) || check(TokenKind::Semicolon)(input) {
            (input, vec![])
        } else {
            let mut throws_types = Vec::new();
            let (mut input, first_ty) = parse_type(input)?;
            throws_types.push(first_ty);

            while check(TokenKind::Comma)(input) {
                let (new_input, _) = token(TokenKind::Comma)(input)?;
                if check(TokenKind::LBrace)(new_input) || check(TokenKind::Semicolon)(new_input) {
                    break;
                }
                let (new_input, ty) = parse_type(new_input)?;
                throws_types.push(ty);
                input = new_input;
            }

            (input, throws_types)
        }
    } else {
        (input, vec![])
    };

    let (input, body, end_span) = if check(TokenKind::LBrace)(input) {
        let (input, block) = parse_block(arena, input)?;
        let end_span = block.span;
        (input, Some(block), end_span)
    } else if check(TokenKind::Semicolon)(input) {
        let (input, tok) = token(TokenKind::Semicolon)(input)?;
        (input, None, tok.span)
    } else {
        return Err(nom::Err::Error(PError {
            input,
            kind: PErrorKind::ExpectedItem,
        }));
    };

    Ok((
        input,
        Item::Function(FunctionItem {
            name,
            receiver,
            generics,
            params,
            return_ty,
            throws,
            is_public,
            body,
            platforms,
            span: start.span.merge(end_span),
        }),
    ))
}

fn parse_receiver<'a>(input: TokenStream<'a>) -> PResult<'a, Receiver> {
    let (input, start) = token(TokenKind::LParen)(input)?;

    // Receivers are always mutable - mut keyword is not allowed
    if check_keyword(Keyword::Mut)(input) {
        return Err(nom::Err::Error(PError {
            input,
            kind: PErrorKind::MutNotAllowedOnReceiver,
        }));
    }

    let (input, name) = ident(input)?;
    let (input, _) = token(TokenKind::Colon)(input)?;
    let (input, ty) = parse_type(input)?;
    let (input, end) = token(TokenKind::RParen)(input)?;

    Ok((
        input,
        Receiver {
            name,
            ty,
            span: start.span.merge(end.span),
        },
    ))
}

fn parse_generic_params<'a>(input: TokenStream<'a>) -> PResult<'a, Vec<GenericParam>> {
    let (input, _) = token(TokenKind::Lt)(input)?;
    let (input, params) = separated_list0(token(TokenKind::Comma), parse_generic_param)(input)?;
    let (input, _) = parse_gt(input)?;
    Ok((input, params))
}

fn parse_generic_param<'a>(input: TokenStream<'a>) -> PResult<'a, GenericParam> {
    let (input, name) = ident(input)?;

    let (input, bounds) = if check(TokenKind::Colon)(input) {
        let (input, _) = token(TokenKind::Colon)(input)?;
        let (input, bounds) = separated_list0(token(TokenKind::Plus), parse_type)(input)?;
        (input, bounds)
    } else {
        (input, Vec::new())
    };

    Ok((
        input,
        GenericParam {
            span: name.span,
            name,
            bounds,
        },
    ))
}

fn parse_parameter<'a>(input: TokenStream<'a>) -> PResult<'a, Parameter> {
    let (input, name) = ident(input)?;
    let start_span = name.span;
    let (input, _) = token(TokenKind::Colon)(input)?;
    let (input, ty) = parse_type(input)?;

    Ok((
        input,
        Parameter {
            name,
            ty,
            span: start_span,
        },
    ))
}

fn parse_struct_item<'a, 'ast>(input: TokenStream<'a>, is_public: bool) -> PResult<'a, Item<'ast>> {
    let (input, start) = keyword(Keyword::Struct)(input)?;
    let (input, name) = ident(input)?;

    let (input, generics) = if check(TokenKind::Lt)(input) {
        parse_generic_params(input)?
    } else {
        (input, Vec::new())
    };

    let (input, implements) = if check_keyword(Keyword::Implements)(input) {
        let (input, _) = keyword(Keyword::Implements)(input)?;
        let (input, impls) = separated_list0(token(TokenKind::Comma), parse_type)(input)?;
        (input, impls)
    } else {
        (input, Vec::new())
    };

    let (input, _) = token(TokenKind::LBrace)(input)?;
    let (input, fields) = parse_struct_fields(input)?;
    let (input, end) = token(TokenKind::RBrace)(input)?;

    Ok((
        input,
        Item::Struct(StructItem {
            name,
            generics,
            implements,
            fields,
            is_public,
            span: start.span.merge(end.span),
        }),
    ))
}

fn parse_struct_fields<'a>(input: TokenStream<'a>) -> PResult<'a, Vec<StructField>> {
    let mut fields = Vec::with_capacity(6);
    let mut input = input;

    loop {
        if check(TokenKind::RBrace)(input) {
            break;
        }

        let (new_input, is_pub) = if check_keyword(Keyword::Pub)(input) {
            let (i, _) = keyword(Keyword::Pub)(input)?;
            (i, true)
        } else {
            (input, false)
        };

        let (new_input, name) = ident(new_input)?;
        let start_span = name.span;
        let (new_input, _) = token(TokenKind::Colon)(new_input)?;
        let (new_input, ty) = parse_type(new_input)?;

        fields.push(StructField {
            name,
            ty,
            is_public: is_pub,
            span: start_span,
        });

        input = new_input;
        if check(TokenKind::Comma)(input) {
            let (new_input, _) = token(TokenKind::Comma)(input)?;
            input = new_input;
        }
    }

    Ok((input, fields))
}

fn parse_enum_item<'a, 'ast>(input: TokenStream<'a>, is_public: bool) -> PResult<'a, Item<'ast>> {
    let (input, start) = keyword(Keyword::Enum)(input)?;
    let (input, name) = ident(input)?;

    let (input, generics) = if check(TokenKind::Lt)(input) {
        parse_generic_params(input)?
    } else {
        (input, Vec::new())
    };

    let (input, _) = token(TokenKind::LBrace)(input)?;
    let (input, variants) = parse_enum_variants(input)?;
    let (input, end) = token(TokenKind::RBrace)(input)?;

    Ok((
        input,
        Item::Enum(EnumItem {
            name,
            generics,
            variants,
            is_public,
            span: start.span.merge(end.span),
        }),
    ))
}

fn parse_enum_variants<'a>(input: TokenStream<'a>) -> PResult<'a, Vec<EnumVariant>> {
    let mut variants = Vec::with_capacity(6);
    let mut input = input;

    loop {
        if check(TokenKind::RBrace)(input) {
            break;
        }

        let (new_input, name) = ident(input)?;

        let (new_input, fields, end_span) = if check(TokenKind::LParen)(new_input) {
            let (i, _) = token(TokenKind::LParen)(new_input)?;
            let (i, types) = separated_list0(token(TokenKind::Comma), parse_type)(i)?;
            let (i, rparen) = token(TokenKind::RParen)(i)?;
            (i, Some(types), rparen.span)
        } else {
            (new_input, None, name.span)
        };

        variants.push(EnumVariant {
            span: name.span.merge(end_span),
            name,
            fields,
        });

        input = new_input;
        if check(TokenKind::Comma)(input) {
            let (new_input, _) = token(TokenKind::Comma)(input)?;
            input = new_input;
        }
    }

    Ok((input, variants))
}

fn parse_interface_item<'a, 'ast>(
    input: TokenStream<'a>,
    is_public: bool,
) -> PResult<'a, Item<'ast>> {
    let (input, start) = keyword(Keyword::Interface)(input)?;
    let (input, name) = ident(input)?;

    let (input, generics) = if check(TokenKind::Lt)(input) {
        parse_generic_params(input)?
    } else {
        (input, Vec::new())
    };

    let (input, extends) = if check(TokenKind::Colon)(input) {
        let (input, _) = token(TokenKind::Colon)(input)?;
        let (input, exts) = separated_list0(token(TokenKind::Comma), parse_type)(input)?;
        (input, exts)
    } else {
        (input, Vec::new())
    };

    let (input, _) = token(TokenKind::LBrace)(input)?;
    let (input, methods) = parse_interface_methods(input)?;
    let (input, end) = token(TokenKind::RBrace)(input)?;

    Ok((
        input,
        Item::Interface(InterfaceItem {
            name,
            generics,
            extends,
            methods,
            is_public,
            span: start.span.merge(end.span),
        }),
    ))
}

fn parse_interface_methods<'a>(input: TokenStream<'a>) -> PResult<'a, Vec<InterfaceMethod>> {
    let mut methods = Vec::with_capacity(8);
    let mut input = input;

    loop {
        if check(TokenKind::RBrace)(input) {
            break;
        }

        let (new_input, start) = keyword(Keyword::Fn)(input)?;
        let (new_input, name) = ident(new_input)?;

        let (new_input, generics) = if check(TokenKind::Lt)(new_input) {
            parse_generic_params(new_input)?
        } else {
            (new_input, Vec::new())
        };

        let (new_input, _) = token(TokenKind::LParen)(new_input)?;
        let (new_input, params) =
            separated_list0(token(TokenKind::Comma), parse_parameter)(new_input)?;
        let (new_input, _) = token(TokenKind::RParen)(new_input)?;

        let (new_input, return_ty) = if check(TokenKind::Arrow)(new_input) {
            let (i, _) = token(TokenKind::Arrow)(new_input)?;
            let (i, ty) = parse_type(i)?;
            (i, Some(ty))
        } else {
            (new_input, None)
        };

        let (new_input, throws) = if check_keyword(Keyword::Throws)(new_input) {
            let (mut i, _) = keyword(Keyword::Throws)(new_input)?;
            if check(TokenKind::Semicolon)(i) {
                (i, vec![])
            } else {
                let mut throws_types = Vec::new();
                let (new_i, first_ty) = parse_type(i)?;
                throws_types.push(first_ty);
                i = new_i;

                while check(TokenKind::Comma)(i) {
                    let (new_i, _) = token(TokenKind::Comma)(i)?;
                    if check(TokenKind::Semicolon)(new_i) {
                        break;
                    }
                    let (new_i, ty) = parse_type(new_i)?;
                    throws_types.push(ty);
                    i = new_i;
                }

                (i, throws_types)
            }
        } else {
            (new_input, vec![])
        };

        let (new_input, end) = token(TokenKind::Semicolon)(new_input)?;

        methods.push(InterfaceMethod {
            name,
            generics,
            params,
            return_ty,
            throws,
            span: start.span.merge(end.span),
        });

        input = new_input;
    }

    Ok((input, methods))
}

fn parse_exception_item<'a, 'ast>(
    input: TokenStream<'a>,
    is_public: bool,
) -> PResult<'a, Item<'ast>> {
    let (input, start) = keyword(Keyword::Exception)(input)?;
    let (input, name) = ident(input)?;
    let (input, _) = token(TokenKind::LBrace)(input)?;

    let mut fields = Vec::new();
    let mut input = input;

    loop {
        if check(TokenKind::RBrace)(input) {
            break;
        }

        let (new_input, field_name) = ident(input)?;
        let start_span = field_name.span;
        let (new_input, _) = token(TokenKind::Colon)(new_input)?;
        let (new_input, ty) = parse_type(new_input)?;

        fields.push(ExceptionField {
            name: field_name,
            ty,
            span: start_span,
        });

        input = new_input;
        if check(TokenKind::Comma)(input) {
            let (new_input, _) = token(TokenKind::Comma)(input)?;
            input = new_input;
        }
    }

    let (input, end) = token(TokenKind::RBrace)(input)?;

    Ok((
        input,
        Item::Exception(ExceptionItem {
            name,
            fields,
            is_public,
            span: start.span.merge(end.span),
        }),
    ))
}

fn parse_use_item<'a, 'ast>(input: TokenStream<'a>) -> PResult<'a, Item<'ast>> {
    let (input, start) = keyword(Keyword::Use)(input)?;

    let (input, first) = ident(input)?;
    let mut path = vec![first];
    let mut input = input;

    while check(TokenKind::ColonColon)(input) {
        let (new_input, _) = token(TokenKind::ColonColon)(input)?;

        if check(TokenKind::LBrace)(new_input) || check(TokenKind::Star)(new_input) {
            input = new_input;
            break;
        }

        let (new_input, segment) = ident(new_input)?;
        path.push(segment);
        input = new_input;
    }

    let (input, items) = if check(TokenKind::Star)(input) {
        let (input, _) = token(TokenKind::Star)(input)?;
        (input, UseItems::All)
    } else if check(TokenKind::LBrace)(input) {
        let (input, _) = token(TokenKind::LBrace)(input)?;
        let (input, entries) = parse_use_entries(input)?;
        let (input, _) = token(TokenKind::RBrace)(input)?;
        (input, UseItems::Specific(entries))
    } else {
        let last = path.pop().unwrap();
        let entry = UseItemEntry {
            span: last.span,
            name: last,
            alias: None,
        };
        (input, UseItems::Specific(vec![entry]))
    };

    let (input, end) = token(TokenKind::Semicolon)(input)?;

    Ok((
        input,
        Item::Use(UseItem {
            path,
            items,
            span: start.span.merge(end.span),
        }),
    ))
}

fn parse_use_entries<'a>(input: TokenStream<'a>) -> PResult<'a, Vec<UseItemEntry>> {
    let mut entries = Vec::new();
    let mut input = input;

    loop {
        if check(TokenKind::RBrace)(input) {
            break;
        }

        let (new_input, name) = ident(input)?;
        let (new_input, alias) = if check_keyword(Keyword::As)(new_input) {
            let (i, _) = keyword(Keyword::As)(new_input)?;
            let (i, alias_name) = ident(i)?;
            (i, Some(alias_name))
        } else {
            (new_input, None)
        };

        entries.push(UseItemEntry {
            span: name.span,
            name,
            alias,
        });

        input = new_input;
        if check(TokenKind::Comma)(input) {
            let (new_input, _) = token(TokenKind::Comma)(input)?;
            input = new_input;
        }
    }

    Ok((input, entries))
}

fn parse_extern_item<'a, 'ast>(input: TokenStream<'a>) -> PResult<'a, Item<'ast>> {
    let (input, start) = keyword(Keyword::Extern)(input)?;
    let (input, _) = keyword(Keyword::Fn)(input)?;
    let (input, name) = ident(input)?;

    let (input, _) = token(TokenKind::LParen)(input)?;
    let (input, params) = separated_list0(token(TokenKind::Comma), parse_parameter)(input)?;
    let (input, _) = token(TokenKind::RParen)(input)?;

    let (input, return_ty) = if check(TokenKind::Arrow)(input) {
        let (input, _) = token(TokenKind::Arrow)(input)?;
        let (input, ty) = parse_type(input)?;
        (input, Some(ty))
    } else {
        (input, None)
    };

    let (input, throws) = if check_keyword(Keyword::Throws)(input) {
        let (mut input, _) = keyword(Keyword::Throws)(input)?;
        if check(TokenKind::Semicolon)(input) || check_keyword(Keyword::As)(input) {
            (input, vec![])
        } else {
            let mut throws_types = Vec::new();
            let (new_input, first_ty) = parse_type(input)?;
            throws_types.push(first_ty);
            input = new_input;

            while check(TokenKind::Comma)(input) {
                let (new_input, _) = token(TokenKind::Comma)(input)?;
                if check(TokenKind::Semicolon)(new_input) || check_keyword(Keyword::As)(new_input) {
                    break;
                }
                let (new_input, ty) = parse_type(new_input)?;
                throws_types.push(ty);
                input = new_input;
            }

            (input, throws_types)
        }
    } else {
        (input, vec![])
    };

    let (input, link_name) = if check_keyword(Keyword::As)(input) {
        let (input, _) = keyword(Keyword::As)(input)?;
        let (input, ln) = ident(input)?;
        (input, Some(ln))
    } else {
        (input, None)
    };

    let (input, end) = token(TokenKind::Semicolon)(input)?;

    Ok((
        input,
        Item::Extern(ExternItem {
            name,
            params,
            return_ty,
            throws,
            link_name,
            span: start.span.merge(end.span),
        }),
    ))
}

fn parse_type_alias_item<'a, 'ast>(
    input: TokenStream<'a>,
    is_public: bool,
) -> PResult<'a, Item<'ast>> {
    let (input, start) = keyword(Keyword::Type)(input)?;
    let (input, name) = ident(input)?;

    let (input, generics) = if check(TokenKind::Lt)(input) {
        parse_generic_params(input)?
    } else {
        (input, Vec::new())
    };

    let (input, _) = token(TokenKind::Eq)(input)?;
    let (input, aliased_type) = parse_type(input)?;
    let (input, end) = token(TokenKind::Semicolon)(input)?;

    Ok((
        input,
        Item::TypeAlias(TypeAliasItem {
            name,
            generics,
            aliased_type,
            is_public,
            span: start.span.merge(end.span),
        }),
    ))
}

fn parse_top_level_stmt<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
) -> PResult<'a, Item<'ast>> {
    let (input, stmt) = parse_statement(arena, input)?;
    let span = stmt.span();
    Ok((input, Item::TopLevelStmt(TopLevelStmtItem { stmt, span })))
}
fn parse_mod_item<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
    is_public: bool,
) -> PResult<'a, Item<'ast>> {
    let (input, start) = keyword(Keyword::Mod)(input)?;
    let (input, name) = ident(input)?;

    if check(TokenKind::Semicolon)(input) {
        let (input, end) = token(TokenKind::Semicolon)(input)?;
        Ok((
            input,
            Item::Mod(ModuleItem {
                name,
                body: None,
                is_public,
                span: start.span.merge(end.span),
            }),
        ))
    } else {
        let (input, _) = token(TokenKind::LBrace)(input)?;
        let mut items = Vec::new();
        let mut input = input;

        while !check(TokenKind::RBrace)(input) && !input.is_empty() {
            let (new_input, item) = parse_item(arena, input)?;
            items.push(item);
            input = new_input;
        }

        let (input, end) = token(TokenKind::RBrace)(input)?;
        Ok((
            input,
            Item::Mod(ModuleItem {
                name,
                body: Some(items),
                is_public,
                span: start.span.merge(end.span),
            }),
        ))
    }
}
