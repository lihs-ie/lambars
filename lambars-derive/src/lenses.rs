//! Implementation of the `#[derive(Lenses)]` macro.
//!
//! This module contains the procedural macro implementation that generates
//! lens accessor methods for struct fields.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields, Generics, Ident};

/// Main implementation of the Lenses derive macro.
pub fn derive_lenses_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let generics = &input.generics;

    let expanded = match &input.data {
        Data::Struct(data_struct) => generate_struct_lenses(name, generics, &data_struct.fields),
        Data::Enum(_) => {
            syn::Error::new_spanned(
                &input.ident,
                "Lenses can only be derived for structs, not enums. Use #[derive(Prisms)] for enums.",
            )
            .to_compile_error()
        }
        Data::Union(_) => {
            syn::Error::new_spanned(&input.ident, "Lenses cannot be derived for unions.")
                .to_compile_error()
        }
    };

    TokenStream::from(expanded)
}

/// Generates lens methods for a struct's fields.
fn generate_struct_lenses(name: &Ident, generics: &Generics, fields: &Fields) -> TokenStream2 {
    match fields {
        Fields::Named(named_fields) => {
            let lens_methods: Vec<TokenStream2> = named_fields
                .named
                .iter()
                .map(|field| {
                    let field_name = field.ident.as_ref().expect("Named field must have ident");
                    let field_type = &field.ty;
                    let method_name = format_ident!("{}_lens", field_name);

                    quote! {
                        /// Returns a lens focusing on the `#field_name` field.
                        ///
                        /// This lens provides get/set access to the field.
                        #[inline]
                        #[must_use]
                        pub fn #method_name() -> impl ::lambars::optics::Lens<Self, #field_type> + Clone {
                            ::lambars::optics::FunctionLens::new(
                                |source: &Self| &source.#field_name,
                                |mut source: Self, value: #field_type| {
                                    source.#field_name = value;
                                    source
                                },
                            )
                        }
                    }
                })
                .collect();

            let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

            quote! {
                impl #impl_generics #name #type_generics #where_clause {
                    #(#lens_methods)*
                }
            }
        }
        Fields::Unnamed(_) => syn::Error::new_spanned(
            name,
            "Lenses can only be derived for structs with named fields, not tuple structs.",
        )
        .to_compile_error(),
        Fields::Unit => syn::Error::new_spanned(
            name,
            "Lenses cannot be derived for unit structs (structs with no fields).",
        )
        .to_compile_error(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_exists() {
        // Placeholder test to verify module compiles
    }
}
