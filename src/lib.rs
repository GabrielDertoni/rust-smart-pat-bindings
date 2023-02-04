use std::collections::VecDeque;

use proc_macro::TokenStream;
use syn::{*, spanned::Spanned, parse::{Parse, Parser}};
use quote::quote;

#[proc_macro_attribute]
pub fn smart_pat_bindings(_attr: TokenStream, expr: TokenStream) -> TokenStream {
    let input = parse_macro_input!(expr as Expr);
    match input {
        Expr::Match(mut expr_match) => {
            let ExprMatch { ref mut arms, .. } = expr_match;
            arms
                .iter_mut()
                .for_each(rewrite_arm);
            quote! { #expr_match }
        }
        _ => quote! {
            compile_error!("#[smart_pat_bindings] can only be used on match expression")
        },
    }.into()
}

fn rewrite_arm(arm: &mut Arm) {
    let Arm { pat, body, .. } = arm;

    let mut bindings = VecDeque::new();
    *pat = get_toplevel_bindings_and_remove_others(pat.clone(), &mut bindings);
    let mut stmts = Vec::new();
    while let Some((binding, pat)) = bindings.pop_front() {
        if let Some(pat) = pat {
            let pat = get_toplevel_bindings_and_remove_others(pat, &mut bindings);
            let q = quote!(let #pat = #binding else { unsafe { ::core::hint::unreachable_unchecked() } };);
            stmts.push(Stmt::parse.parse2(q).unwrap());
        }
    }
    // FIXME: This clone is avoidable and may be expensive
    stmts.push(Stmt::Expr(*body.clone()));
    *body = Box::new(Expr::Block(ExprBlock {
        attrs: vec![],
        label: None,
        block: Block {
            brace_token: token::Brace { span: body.span() },
            stmts,
        }
    }));
}

fn get_toplevel_bindings_and_remove_others(pat: Pat, bindings: &mut VecDeque<(Ident, Option<Pat>)>) -> Pat {
    match pat {
        Pat::Ident(PatIdent { ident, subpat, attrs, by_ref, mutability }) => {
            bindings.push_back((ident.clone(), subpat.as_ref().map(|(_, subpat)| *subpat.clone())));
            Pat::Ident(PatIdent {
                attrs,
                ident,
                by_ref,
                mutability,
                subpat: subpat
                    .map(|(at, subpat)| (at, Box::new(remove_all_bindings(*subpat)))),
            })
        }
        Pat::Box(PatBox { attrs, box_token, pat }) =>
            Pat::Box(PatBox {
                attrs,
                box_token,
                pat: Box::new(get_toplevel_bindings_and_remove_others(*pat, bindings))
            }),
        Pat::Or(PatOr { attrs, leading_vert, cases }) =>
            Pat::Or(PatOr {
                attrs,
                leading_vert,
                cases: cases
                    .into_iter()
                    .map(|case| get_toplevel_bindings_and_remove_others(case, bindings))
                    .collect(),
            }),
        Pat::Struct(PatStruct { attrs, path, brace_token, fields, dot2_token }) =>
            Pat::Struct(PatStruct { attrs,
                path,
                brace_token,
                dot2_token,
                fields: fields
                    .into_iter()
                    .map(|FieldPat { attrs, member, colon_token, pat }| FieldPat {
                        attrs,
                        member,
                        colon_token,
                        pat: Box::new(get_toplevel_bindings_and_remove_others(*pat, bindings)),
                    })
                    .collect(),
            }),
        Pat::Tuple(PatTuple { attrs, paren_token, elems }) =>
            Pat::Tuple(PatTuple {
                attrs,
                paren_token,
                elems: elems
                    .into_iter()
                    .map(|elem| get_toplevel_bindings_and_remove_others(elem, bindings))
                    .collect(),
            }),
        Pat::TupleStruct(PatTupleStruct { attrs, path, pat: PatTuple { attrs: pat_attrs, paren_token, elems } }) =>
            Pat::TupleStruct(PatTupleStruct {
                attrs,
                path,
                pat: PatTuple {
                    attrs: pat_attrs,
                    paren_token,
                    elems: elems
                        .into_iter()
                        .map(|elem| get_toplevel_bindings_and_remove_others(elem, bindings))
                        .collect(),
                },
            }),
        Pat::Slice(PatSlice { attrs, bracket_token, elems }) =>
            Pat::Slice(PatSlice {
                attrs,
                bracket_token,
                elems: elems
                    .into_iter()
                    .map(|elem| get_toplevel_bindings_and_remove_others(elem, bindings))
                    .collect(),
            }),
        Pat::Reference(PatReference { attrs, and_token, mutability, pat }) =>
            Pat::Reference(PatReference {
                attrs,
                and_token,
                mutability,
                pat: Box::new(get_toplevel_bindings_and_remove_others(*pat, bindings)),
            }),
        // Patterns without bindings or inner patterns
        | Pat::Range(_)
        | Pat::Lit(_)
        | Pat::Path(_)
        | Pat::Rest(_)
        | Pat::Type(_)
        | Pat::Wild(_) => pat,
        Pat::Macro(_) => panic!("#[smart_pat_bindings] cannot process macro in match"),
        pat => pat,
    }
}

fn remove_all_bindings(pat: Pat) -> Pat {
    match pat {
        // TODO: Keep the same attributes, instead of discarding them
        Pat::Ident(PatIdent { subpat: Some((_, subpat)), .. }) =>
            remove_all_bindings(*subpat),

        Pat::Ident(pat@PatIdent { subpat: None, .. }) =>
            Pat::Wild(PatWild { attrs: vec![], underscore_token: Token![_](pat.span()) }),

        Pat::Box(PatBox { attrs, box_token, pat }) =>
            Pat::Box(PatBox {
                attrs,
                box_token,
                pat: Box::new(remove_all_bindings(*pat)),
            }),
        Pat::Or(PatOr { attrs, leading_vert, cases }) =>
            Pat::Or(PatOr {
                attrs,
                leading_vert,
                cases: cases
                    .into_iter()
                    .map(|case| remove_all_bindings(case))
                    .collect(),
            }),
        Pat::Struct(PatStruct { attrs, path, brace_token, fields, dot2_token }) =>
            Pat::Struct(PatStruct {
                attrs,
                path,
                brace_token,
                dot2_token,
                fields: fields
                    .into_iter()
                    .map(|FieldPat { attrs, member, pat, .. }| FieldPat {
                        attrs,
                        colon_token: Some(Token![:](member.span())),
                        member,
                        pat: Box::new(remove_all_bindings(*pat)),
                    })
                    .collect(),
            }),
        Pat::Tuple(PatTuple { attrs, paren_token, elems }) =>
            Pat::Tuple(PatTuple {
                attrs,
                paren_token,
                elems: elems
                    .into_iter()
                    .map(|elem| remove_all_bindings(elem))
                    .collect(),
            }),
        Pat::TupleStruct(PatTupleStruct { attrs, path, pat: PatTuple { attrs: pat_attrs, paren_token, elems } }) =>
            Pat::TupleStruct(PatTupleStruct {
                attrs,
                path,
                pat: PatTuple {
                    attrs: pat_attrs,
                    paren_token,
                    elems: elems
                        .into_iter()
                        .map(|elem| remove_all_bindings(elem))
                        .collect(),
                },
            }),
        Pat::Slice(PatSlice { attrs, bracket_token, elems }) =>
            Pat::Slice(PatSlice {
                attrs,
                bracket_token,
                elems: elems
                    .into_iter()
                    .map(|elem| remove_all_bindings(elem))
                    .collect(),
            }),
        Pat::Reference(PatReference { attrs, and_token, mutability, pat }) =>
            Pat::Reference(PatReference {
                attrs,
                and_token,
                mutability,
                pat: Box::new(remove_all_bindings(*pat)),
            }),
        Pat::Macro(PatMacro { attrs, mac }) =>
            Pat::Macro(PatMacro { attrs, mac }),
        // Patterns without bindings or inner patterns
        | Pat::Range(_)
        | Pat::Lit(_)
        | Pat::Path(_)
        | Pat::Rest(_)
        | Pat::Type(_)
        | Pat::Wild(_) => pat,
        pat => pat,
    }
}
