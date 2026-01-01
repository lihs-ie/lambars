//! Implementation of the `#[derive(Prisms)]` macro.
//!
//! This module contains the procedural macro implementation that generates
//! prism accessor methods for enum variants.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields, Generics, Ident, Variant};

/// Main implementation of the Prisms derive macro.
pub fn derive_prisms_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let generics = &input.generics;

    let expanded = match &input.data {
        Data::Enum(data_enum) => {
            generate_enum_prisms(name, generics, &data_enum.variants.iter().collect::<Vec<_>>())
        }
        Data::Struct(_) => {
            syn::Error::new_spanned(
                &input.ident,
                "Prisms can only be derived for enums, not structs. Use #[derive(Lenses)] for structs.",
            )
            .to_compile_error()
        }
        Data::Union(_) => {
            syn::Error::new_spanned(&input.ident, "Prisms cannot be derived for unions.")
                .to_compile_error()
        }
    };

    TokenStream::from(expanded)
}

/// Generates prism methods for an enum's variants.
fn generate_enum_prisms(name: &Ident, generics: &Generics, variants: &[&Variant]) -> TokenStream2 {
    let prism_methods: Vec<TokenStream2> = variants
        .iter()
        .map(|variant| generate_variant_prism(name, variant))
        .collect();

    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics #name #type_generics #where_clause {
            #(#prism_methods)*
        }
    }
}

/// Generates a prism method for a single enum variant.
fn generate_variant_prism(enum_name: &Ident, variant: &Variant) -> TokenStream2 {
    let variant_name = &variant.ident;
    let method_name = format_ident!("{}_prism", to_snake_case(&variant_name.to_string()));

    match &variant.fields {
        // Unit variant: e.g., `None` or `Point`
        Fields::Unit => generate_unit_variant_prism(enum_name, variant_name, &method_name),

        // Tuple variant: e.g., `Some(T)` or `Rectangle(f64, f64)`
        Fields::Unnamed(fields) => {
            let field_types: Vec<_> = fields.unnamed.iter().map(|f| &f.ty).collect();

            if field_types.len() == 1 {
                generate_single_field_tuple_prism(
                    enum_name,
                    variant_name,
                    &method_name,
                    field_types[0],
                )
            } else {
                generate_multi_field_tuple_prism(
                    enum_name,
                    variant_name,
                    &method_name,
                    &field_types,
                )
            }
        }

        // Struct variant: e.g., `Click { x: i32, y: i32 }`
        Fields::Named(fields) => {
            let field_names: Vec<_> = fields
                .named
                .iter()
                .map(|f| f.ident.as_ref().expect("Named field must have ident"))
                .collect();
            let field_types: Vec<_> = fields.named.iter().map(|f| &f.ty).collect();

            generate_struct_variant_prism(
                enum_name,
                variant_name,
                &method_name,
                &field_names,
                &field_types,
            )
        }
    }
}

/// Generates a prism for a unit variant.
fn generate_unit_variant_prism(
    _enum_name: &Ident,
    variant_name: &Ident,
    method_name: &Ident,
) -> TokenStream2 {
    quote! {
        /// Returns a prism focusing on the `#variant_name` variant.
        ///
        /// This prism provides preview/review access to the variant.
        /// For unit variants, the target type is `()`.
        #[inline]
        #[must_use]
        pub fn #method_name() -> impl ::lambars::optics::Prism<Self, ()> + Clone {
            ::lambars::optics::FunctionPrism::new(
                |source: &Self| match source {
                    Self::#variant_name => Some(&()),
                    #[allow(unreachable_patterns)]
                    _ => None,
                },
                |_: ()| Self::#variant_name,
                |source: Self| match source {
                    Self::#variant_name => Some(()),
                    #[allow(unreachable_patterns)]
                    _ => None,
                },
            )
        }
    }
}

/// Generates a prism for a tuple variant with a single field.
fn generate_single_field_tuple_prism(
    _enum_name: &Ident,
    variant_name: &Ident,
    method_name: &Ident,
    field_type: &syn::Type,
) -> TokenStream2 {
    quote! {
        /// Returns a prism focusing on the `#variant_name` variant.
        ///
        /// This prism provides preview/review access to the variant's value.
        #[inline]
        #[must_use]
        pub fn #method_name() -> impl ::lambars::optics::Prism<Self, #field_type> + Clone {
            ::lambars::optics::FunctionPrism::new(
                |source: &Self| match source {
                    Self::#variant_name(value) => Some(value),
                    #[allow(unreachable_patterns)]
                    _ => None,
                },
                |value: #field_type| Self::#variant_name(value),
                |source: Self| match source {
                    Self::#variant_name(value) => Some(value),
                    #[allow(unreachable_patterns)]
                    _ => None,
                },
            )
        }
    }
}

/// Generates a prism for a tuple variant with multiple fields.
///
/// Note: Due to Rust's enum layout, `preview` always returns `None` for
/// multi-field tuple variants. Use `preview_owned` instead.
fn generate_multi_field_tuple_prism(
    _enum_name: &Ident,
    variant_name: &Ident,
    method_name: &Ident,
    field_types: &[&syn::Type],
) -> TokenStream2 {
    let tuple_type = quote! { (#(#field_types),*) };

    // Generate pattern variable names: v0, v1, v2, ...
    let pattern_vars: Vec<_> = (0..field_types.len())
        .map(|index| format_ident!("v{}", index))
        .collect();

    // Generate tuple construction
    let tuple_construct = quote! { (#(#pattern_vars),*) };

    // Generate variant construction
    let variant_construct = quote! { Self::#variant_name(#(#pattern_vars),*) };

    quote! {
        /// Returns a prism focusing on the `#variant_name` variant.
        ///
        /// **Note**: For multi-field tuple variants, `preview` always returns `None`
        /// because Rust's enum layout doesn't store the fields as a tuple in memory.
        /// Use `preview_owned` instead to extract the values.
        ///
        /// This prism provides review and preview_owned access to the variant's values as a tuple.
        #[inline]
        #[must_use]
        pub fn #method_name() -> impl ::lambars::optics::Prism<Self, #tuple_type> + Clone {
            ::lambars::optics::FunctionPrism::new(
                // preview always returns None for multi-field variants
                // because we cannot return a reference to a tuple that doesn't exist in memory
                |_source: &Self| -> Option<&#tuple_type> {
                    None
                },
                |tuple: #tuple_type| {
                    let #tuple_construct = tuple;
                    #variant_construct
                },
                |source: Self| match source {
                    Self::#variant_name(#(#pattern_vars),*) => Some(#tuple_construct),
                    #[allow(unreachable_patterns)]
                    _ => None,
                },
            )
        }
    }
}

/// Generates a prism for a struct variant.
///
/// Note: Due to Rust's enum layout, `preview` always returns `None` for
/// struct variants. Use `preview_owned` instead.
fn generate_struct_variant_prism(
    _enum_name: &Ident,
    variant_name: &Ident,
    method_name: &Ident,
    field_names: &[&Ident],
    field_types: &[&syn::Type],
) -> TokenStream2 {
    let tuple_type = quote! { (#(#field_types),*) };

    // Generate tuple variable names for destructuring: t0, t1, t2, ...
    let tuple_vars: Vec<_> = (0..field_types.len())
        .map(|index| format_ident!("t{}", index))
        .collect();

    // Generate tuple construction for owned extraction
    let tuple_construct_owned = quote! { (#(#field_names),*) };

    // Generate struct construction from tuple
    let struct_construct = quote! {
        Self::#variant_name { #(#field_names: #tuple_vars),* }
    };

    // Generate pattern for matching struct variant
    let struct_pattern = quote! {
        Self::#variant_name { #(#field_names),* }
    };

    quote! {
        /// Returns a prism focusing on the `#variant_name` variant.
        ///
        /// **Note**: For struct variants, `preview` always returns `None`
        /// because Rust's enum layout doesn't store the fields as a tuple in memory.
        /// Use `preview_owned` instead to extract the values.
        ///
        /// This prism provides review and preview_owned access to the variant's fields as a tuple.
        #[inline]
        #[must_use]
        pub fn #method_name() -> impl ::lambars::optics::Prism<Self, #tuple_type> + Clone {
            ::lambars::optics::FunctionPrism::new(
                // preview always returns None for struct variants
                // because we cannot return a reference to a tuple that doesn't exist in memory
                |_source: &Self| -> Option<&#tuple_type> {
                    None
                },
                |tuple: #tuple_type| {
                    let (#(#tuple_vars),*) = tuple;
                    #struct_construct
                },
                |source: Self| match source {
                    #struct_pattern => Some(#tuple_construct_owned),
                    #[allow(unreachable_patterns)]
                    _ => None,
                },
            )
        }
    }
}

/// Converts a `CamelCase` or `PascalCase` string to `snake_case`.
fn to_snake_case(input: &str) -> String {
    let mut result = String::with_capacity(input.len() + 4);
    let chars: Vec<char> = input.chars().collect();

    for (index, &character) in chars.iter().enumerate() {
        if character.is_uppercase() {
            // Check if we need to insert an underscore before this uppercase letter
            if index > 0 {
                let previous_char = chars[index - 1];
                let next_is_lowercase = chars.get(index + 1).is_some_and(|c| c.is_lowercase());

                // Insert underscore if:
                // 1. Previous char is lowercase (e.g., "keyPress" -> "key_press")
                // 2. Previous char is uppercase and next char is lowercase (e.g., "XMLParser" -> "xml_parser")
                if previous_char.is_lowercase() || (previous_char.is_uppercase() && next_is_lowercase)
                {
                    result.push('_');
                }
            }
            result.push(character.to_lowercase().next().unwrap_or(character));
        } else {
            result.push(character);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_snake_case_simple() {
        assert_eq!(to_snake_case("Circle"), "circle");
        assert_eq!(to_snake_case("Rectangle"), "rectangle");
        assert_eq!(to_snake_case("Point"), "point");
    }

    #[test]
    fn test_to_snake_case_multi_word() {
        assert_eq!(to_snake_case("KeyPress"), "key_press");
        assert_eq!(to_snake_case("MouseClick"), "mouse_click");
        assert_eq!(to_snake_case("DataPoint"), "data_point");
    }

    #[test]
    fn test_to_snake_case_already_lowercase() {
        assert_eq!(to_snake_case("none"), "none");
        assert_eq!(to_snake_case("some"), "some");
    }

    #[test]
    fn test_to_snake_case_acronyms() {
        assert_eq!(to_snake_case("HTTPRequest"), "http_request");
        assert_eq!(to_snake_case("XMLParser"), "xml_parser");
    }

    #[test]
    fn test_to_snake_case_single_char() {
        assert_eq!(to_snake_case("A"), "a");
        assert_eq!(to_snake_case("X"), "x");
    }
}
