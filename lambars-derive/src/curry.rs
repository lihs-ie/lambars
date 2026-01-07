//! Implementation of the `curry!` procedural macro.
//!
//! This module provides the curry! macro that transforms
//! multi-argument closures or functions into curried form.
//!
//! # Supported Input Forms
//!
//! 1. Closure form: `curry!(|a, b| body)`
//! 2. Function name + arity form: `curry!(function_name, arity)`
//!
//! # Design
//!
//! The macro generates nested closures that capture arguments using `Rc`.
//! This enables:
//!
//! - Reuse of curried closures (can call the same curried function multiple times)
//! - Reuse of partial applications (can apply the same partial application to different arguments)
//! - Support for non-Copy types (arguments are cloned via `Rc::unwrap_or_clone`)
//!
//! # Generated Code Structure
//!
//! ## Closure Form
//!
//! For a closure `|a, b, c| body`, the macro generates:
//!
//! ```text
//! {
//!     let __lambars_function = Rc::new(|a, b, c| body);
//!     move |__lambars_argument_0| {
//!         let __lambars_function = Rc::clone(&__lambars_function);
//!         let __lambars_argument_0 = Rc::new(__lambars_argument_0);
//!         move |__lambars_argument_1| {
//!             let __lambars_function = Rc::clone(&__lambars_function);
//!             let __lambars_argument_0 = Rc::clone(&__lambars_argument_0);
//!             let __lambars_argument_1 = Rc::new(__lambars_argument_1);
//!             move |__lambars_argument_2| {
//!                 __lambars_function(
//!                     Rc::unwrap_or_clone(Rc::clone(&__lambars_argument_0)),
//!                     Rc::unwrap_or_clone(Rc::clone(&__lambars_argument_1)),
//!                     __lambars_argument_2,
//!                 )
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! ## Function Name + Arity Form
//!
//! For `curry!(add, 2)`, the macro generates:
//!
//! ```text
//! {
//!     let __lambars_function = Rc::new(add);
//!     move |__lambars_argument_0| {
//!         let __lambars_function = Rc::clone(&__lambars_function);
//!         let __lambars_argument_0 = Rc::new(__lambars_argument_0);
//!         move |__lambars_argument_1| {
//!             __lambars_function(
//!                 Rc::unwrap_or_clone(Rc::clone(&__lambars_argument_0)),
//!                 __lambars_argument_1,
//!             )
//!         }
//!     }
//! }
//! ```

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{Expr, ExprClosure, ExprLit, ExprPath, Lit, Token, spanned::Spanned};

enum CurryInput {
    Closure(ExprClosure),
    FunctionWithArity { function: ExprPath, arity: usize },
}

pub fn curry_impl(input: TokenStream) -> TokenStream {
    let expanded = match parse_curry_input(input) {
        Ok(CurryInput::Closure(closure)) => generate_curry_from_closure(&closure),
        Ok(CurryInput::FunctionWithArity { function, arity }) => {
            generate_nested_closures(arity, quote! { #function })
        }
        Err(error) => error.to_compile_error(),
    };

    TokenStream::from(expanded)
}

fn parse_curry_input(input: TokenStream) -> syn::Result<CurryInput> {
    let input_tokens: proc_macro2::TokenStream = input.into();
    let parser = Punctuated::<Expr, Token![,]>::parse_terminated;
    let expressions: Punctuated<Expr, Token![,]> =
        syn::parse::Parser::parse2(parser, input_tokens)?;

    match expressions.len() {
        1 => parse_single_expression(expressions),
        2 => parse_function_with_arity(expressions),
        _ => Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "curry! requires a closure or function name with arity",
        )),
    }
}

fn parse_single_expression(expressions: Punctuated<Expr, Token![,]>) -> syn::Result<CurryInput> {
    let expression = expressions
        .into_iter()
        .next()
        .expect("expected one expression");

    match expression {
        Expr::Closure(closure) => Ok(CurryInput::Closure(closure)),
        Expr::Path(path) => Err(syn::Error::new(
            path.span(),
            "curry! with function name requires arity: curry!(function_name, 2)",
        )),
        other => Err(syn::Error::new(
            other.span(),
            "curry! requires a closure or function name with arity",
        )),
    }
}

fn parse_function_with_arity(expressions: Punctuated<Expr, Token![,]>) -> syn::Result<CurryInput> {
    let mut iterator = expressions.into_iter();
    let first = iterator.next().expect("expected first expression");
    let second = iterator.next().expect("expected second expression");

    let function = match first {
        Expr::Path(path) => path,
        other => {
            return Err(syn::Error::new(
                other.span(),
                "expected a function name or path",
            ));
        }
    };

    let arity = match second {
        Expr::Lit(ExprLit {
            lit: Lit::Int(literal_integer),
            ..
        }) => literal_integer.base10_parse::<usize>()?,
        other => {
            return Err(syn::Error::new(
                other.span(),
                "curry! expected an integer literal for arity",
            ));
        }
    };

    if arity < 2 {
        return Err(syn::Error::new(
            function.span(),
            "curry! requires a function with at least 2 arguments",
        ));
    }

    Ok(CurryInput::FunctionWithArity { function, arity })
}

fn generate_curry_from_closure(closure: &ExprClosure) -> TokenStream2 {
    let argument_count = closure.inputs.len();

    if argument_count < 2 {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "curry! requires a function with at least 2 arguments",
        )
        .to_compile_error();
    }

    generate_nested_closures(argument_count, quote! { #closure })
}

fn generate_nested_closures(
    argument_count: usize,
    function_expression: TokenStream2,
) -> TokenStream2 {
    let argument_identifiers: Vec<_> = (0..argument_count)
        .map(|index| quote::format_ident!("__lambars_argument_{}", index))
        .collect();

    let final_arguments: Vec<_> = argument_identifiers
        .iter()
        .enumerate()
        .map(|(index, identifier)| {
            if index < argument_count - 1 {
                quote! {
                    ::std::rc::Rc::unwrap_or_clone(::std::rc::Rc::clone(&#identifier))
                }
            } else {
                quote! { #identifier }
            }
        })
        .collect();

    let function_call = quote! { __lambars_function(#(#final_arguments),*) };

    let closure_chain = build_closure_chain(&argument_identifiers, argument_count, function_call);

    quote! {
        {
            let __lambars_function = ::std::rc::Rc::new(#function_expression);
            #closure_chain
        }
    }
}

fn build_closure_chain(
    argument_identifiers: &[proc_macro2::Ident],
    argument_count: usize,
    innermost_body: TokenStream2,
) -> TokenStream2 {
    let mut current_body = innermost_body;

    for index in (0..argument_count).rev() {
        let identifier = &argument_identifiers[index];

        current_body = if index == argument_count - 1 {
            quote! {
                move |#identifier| { #current_body }
            }
        } else {
            let clones_before: Vec<_> = argument_identifiers[..index]
                .iter()
                .map(|previous_identifier| {
                    quote! {
                        let #previous_identifier = ::std::rc::Rc::clone(&#previous_identifier);
                    }
                })
                .collect();

            quote! {
                move |#identifier| {
                    let __lambars_function = ::std::rc::Rc::clone(&__lambars_function);
                    #(#clones_before)*
                    let #identifier = ::std::rc::Rc::new(#identifier);
                    #current_body
                }
            }
        };
    }

    current_body
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    #[rstest]
    fn test_module_compiles() {}
}
