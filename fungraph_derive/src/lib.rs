use proc_macro::TokenStream;
use quote::quote;
use syn::*;

#[proc_macro_derive(ToolParameters)]
pub fn tool_parameters_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();

    match impl_tool_parameters(&ast) {
        Ok(expanded) => expanded.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn impl_tool_parameters(ast: &DeriveInput) -> Result<TokenStream> {
    let name = &ast.ident;

    let fields = match &ast.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => Err(syn::Error::new_spanned(
                &ast,
                "ToolParameters derive only supports named fields",
            ))?,
        },
        _ => Err(syn::Error::new_spanned(
            &ast,
            "ToolParameters derive only supports structs",
        ))?,
    };

    let properties: Result<Vec<(String, proc_macro2::TokenStream, bool)>> = fields
        .iter()
        .map(|field| {
            let name = field.ident.as_ref().unwrap().to_string();
            let data_type = get_data_type(field)?;
            let description = get_description(field);
            let required = is_required(field);

            // TODO: Handle enum values
            let prop = match description {
                Some(desc) => {
                    quote! {
                        fungraph::types::openai::Property {
                            r#type: #data_type.to_string(),
                            description: Some(#desc.to_string()),
                            enum_values: None,
                        }
                    }
                }
                None => {
                    quote! {
                        fungraph::types::openai::Property {
                            r#type: #data_type.to_string(),
                            description: None,
                            enum_values: None,
                        }
                    }
                }
            };

            Ok((name, prop, required))
        })
        .collect();

    let properties = properties?;

    let required_fields = properties
        .clone()
        .into_iter()
        .filter(|(_, _, required)| *required)
        .map(|(name, _, _)| name)
        .collect::<Vec<_>>();
    let keys = properties
        .clone()
        .into_iter()
        .map(|(name, _, _)| name)
        .collect::<Vec<_>>();
    let values = properties
        .clone()
        .into_iter()
        .map(|(_, prop, _)| prop)
        .collect::<Vec<_>>();

    let gen_code = quote! {
        impl fungraph::tools::ToolParameters for #name {
            fn parameters() -> fungraph::types::openai::Parameters {
                fungraph::types::openai::Parameters {
                    r#type: "object".to_string(),
                    properties: {
                        let mut map = std::collections::HashMap::new();
                        #(
                            map.insert(#keys.to_string(), #values);
                        )*
                        map
                    },
                    required: vec![#(#required_fields.to_string()),*],
                }
            }
        }
    };

    Ok(gen_code.into())
}

fn get_data_type(field: &Field) -> Result<String> {
    let ty = &field.ty;
    let js_type = get_data_type_inner(ty)?;

    if js_type.is_empty() {
        Err(syn::Error::new_spanned(
            ty,
            "Unsupported type for ToolParameters derive",
        ))
    } else {
        Ok(js_type)
    }
}

fn get_data_type_inner(ty: &Type) -> Result<String> {
    match ty {
        Type::Path(type_path) => {
            if type_path.path.segments.len() > 0 {
                let ident = &type_path.path.segments[0].ident;
                match ident.to_string().as_str() {
                    "String" => Ok("string".to_string()),
                    "str" => Ok("string".to_string()),
                    "i32" | "i64" | "u32" | "u64" | "usize" | "isize" => Ok("number".to_string()),
                    "f32" | "f64" => Ok("number".to_string()),
                    "bool" => Ok("boolean".to_string()),
                    "Vec" => Ok("array".to_string()),
                    "Option" => get_option_type(type_path),
                    _ => Ok("object".to_string()), // Default to object for custom types
                }
            } else {
                Ok("object".to_string())
            }
        }
        _ => Err(syn::Error::new_spanned(
            ty,
            "Unsupported type for ToolParameters derive",
        )),
    }
}

fn get_option_type(type_path: &TypePath) -> Result<String> {
    // Option 型のジェネリック引数を取得
    if let PathArguments::AngleBracketed(args) = &type_path.path.segments[0].arguments {
        if args.args.len() == 1 {
            if let GenericArgument::Type(inner_type) = &args.args[0] {
                // Option 型の内部の型に対して再帰的に get_data_type を呼び出す
                return get_data_type_inner(inner_type);
            }
        }
    };
    Err(syn::Error::new_spanned(
        type_path,
        "Unsupported type for ToolParameters derive",
    ))
}

fn get_description(field: &Field) -> Option<String> {
    let description = field
        .attrs
        .iter()
        .filter_map(|attr: &Attribute| {
            if let Meta::NameValue(name_value) = &attr.meta {
                if name_value.path.is_ident("doc") {
                    if let syn::Expr::Lit(expr_lit) = &name_value.value {
                        if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                            return Some(lit_str.value().trim().to_string());
                        }
                    }
                }
            }
            None
        })
        .collect::<Vec<_>>()
        .join("\n")
        .to_string();

    if description.is_empty() {
        None
    } else {
        Some(description)
    }
}

fn is_required(field: &Field) -> bool {
    // Check if the field is an Option. If it is, it's not required.
    let ty_string = quote!(#field.ty).to_string();
    !ty_string.starts_with("Option <")
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use syn::{DeriveInput, Field, Fields};

    fn parse_field(input: proc_macro2::TokenStream) -> Field {
        let derive_input: DeriveInput = syn::parse2(quote! {
            struct Temp {
                #input
            }
        })
        .unwrap();

        match derive_input.data {
            syn::Data::Struct(data) => match data.fields {
                Fields::Named(fields) => fields.named.into_iter().next().unwrap(),
                _ => panic!("Expected named fields"),
            },
            _ => panic!("Expected struct"),
        }
    }

    // cargo test tests
    #[test]
    fn test_get_description() {
        let field = parse_field(quote! {
            #[doc = "This is a test description."]
            #[doc = "It has multiple lines."]
            pub field_name: String,
        });

        let description = get_description(&field);
        assert_eq!(
            description,
            Some("This is a test description.\nIt has multiple lines.".to_string())
        );
    }

    #[test]
    fn test_get_description_slash_comment() {
        let field = parse_field(quote! {
            /// This is a test description.
            /// It has multiple lines.
            pub field_name: String,
        });

        let description = get_description(&field);
        assert_eq!(
            description,
            Some("This is a test description.\nIt has multiple lines.".to_string())
        );
    }

    #[test]
    fn test_get_description_no_doc() {
        let field = parse_field(quote! {
            pub field_name: String,
        });

        let description = get_description(&field);
        assert_eq!(description, None);
    }

    #[test]
    fn test_get_description_other_attribute() {
        let field = parse_field(quote! {
            #[serde(default)]
            pub field_name: String,
        });

        let description = get_description(&field);
        assert_eq!(description, None);
    }
}
