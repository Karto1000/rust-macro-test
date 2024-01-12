#![feature(proc_macro_quote)]

use proc_macro::TokenStream;
use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;

use quote::{quote, ToTokens};
use syn::{Field, ItemStruct, Meta};
use syn::punctuated::Punctuated;

#[proc_macro_attribute]
pub fn from_file(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_clone = item.clone();
    let input = syn::parse_macro_input!(item_clone as ItemStruct);
    let attributes = syn::parse_macro_input!(attr with Punctuated::<Meta, syn::Token![,]>::parse_terminated);

    let path = attributes.first()
        .unwrap()
        .require_name_value()
        .unwrap()
        .value
        .to_token_stream();

    let load_static = attributes.iter()
        .find(
            |v| {
                v.require_name_value()
                    .unwrap()
                    .path
                    .get_ident()
                    .unwrap()
                    .to_string() == "is_static"
            }
        );


    let should_load_static: bool = match load_static {
        None => false,
        Some(v) => {
            v.require_name_value()
                .unwrap()
                .value
                .to_token_stream()
                .to_string()
                .eq("true")
        }
    };

    let struct_name = &input.ident;

    let path_param_str = path.to_string().replace("\"", "");
    let search_path = Path::new(std::env::current_dir().unwrap().to_str().unwrap()).join(path_param_str);

    let mut file = match std::fs::File::open(search_path) {
        Ok(f) => f,
        Err(e) => {
            let e_str = e.to_string();
            return quote! { compile_error!(#e_str); }.into();
        }
    };

    let mut content = String::new();

    match <std::fs::File as std::io::Read>::read_to_string(&mut file, &mut content) {
        Ok(_) => {}
        Err(e) => {
            let e_str = e.to_string();
            return quote! { compile_error!(#e_str); }.into();
        }
    };


    let instance: HashMap<String, serde_json::Value> = match serde_json::from_str(content.as_str()) {
        Err(e) => {
            let e_str = e.to_string();
            return quote! { compile_error!(#e_str); }.into();
        }
        Ok(i) => i
    };


    let load_static_impl = match should_load_static {
        false => proc_macro2::TokenStream::new(),
        true => {
            // Remove any extra values from the json that we do not need, this makes sure that the order of the fields
            // is correct when we sort the input and struct values
            let mut filtered_values: Vec<(String, serde_json::Value)> = instance.into_iter()
                .filter(|(k, _)| input.fields.iter().map(|f| f.ident
                    .to_token_stream()
                    .to_string())
                    .collect::<Vec<String>>()
                    .contains(k)
                )
                .collect();

            filtered_values.sort_by(|(first_key, _), (second_key, _)| first_key.cmp(&second_key));

            let mut sorted_struct_fields = input.fields.iter()
                .collect::<Vec<&Field>>();

            sorted_struct_fields.sort_by(|first, second| first.ident
                .to_token_stream()
                .to_string()
                .cmp(
                    &second.ident
                        .clone()
                        .unwrap()
                        .to_token_stream()
                        .to_string()
                )
            );

            // Create iterators over both the name and value field of the struct.
            // The order of the names and values must be the exact same, that is why we sorted the fields and values.

            let name_iter = sorted_struct_fields.iter()
                .map(|f| f.ident.clone().unwrap()).clone();

            let mut value_iter = filtered_values.into_iter()
                .zip(&sorted_struct_fields)
                .map(|((_, v), field)| {
                    let v_string = v.to_string();
                    let mut result: proc_macro2::TokenStream = proc_macro2::TokenStream::from_str(v_string.as_str()).unwrap();

                    let field_type = field.ty.to_token_stream();

                    if v.is_object() || v.is_array() {
                        result = quote! {
                            serde_json::from_str::<#field_type>(#v_string).unwrap()
                        }
                    }

                    result
                });

            quote!(
                impl macro_test_traits::LoadStatic for #struct_name {
                    /// Create a new instance of Self with values that are inserted at compile time by
                    /// opening the file and inserting the appropriate fields into the constructor
                    fn load_static() -> Self {
                        return Self {
                            #(#name_iter: #value_iter.into(), )*
                        }
                    }
                }
            )
        }
    };

    return TokenStream::from(
        quote!(
            #input

            impl macro_test_traits::Load for #struct_name {
                /// Load the values from the file by opening it and parsing the contents using serde_json
                fn load() -> Result<Self, macro_test_traits::LoadError> {
                    let mut file = match std::fs::File::open(#path) {
                        Err(_) => return Err(macro_test_traits::LoadError::FileNotFound),
                        Ok(f) => f
                    };

                    let mut content = String::new();
                    match <std::fs::File as std::io::Read>::read_to_string(&mut file, &mut content) {
                        Ok(_) => {},
                        Err(_) => return Err(macro_test_traits::LoadError::ReadError)
                    };

                    let instance: #struct_name = match serde_json::from_str(content.as_str()) {
                        Err(_) => return Err(macro_test_traits::LoadError::ParseError),
                        Ok(i) => i
                    };

                    return Ok(instance)
                }
            }

            #load_static_impl
        )
    );
}