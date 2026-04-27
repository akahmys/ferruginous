extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, LitStr, LitFloat};

#[proc_macro_derive(FromPdfObject, attributes(pdf_key, pdf_dict))]
pub fn derive_from_pdf_object(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Parse #[pdf_dict(clause = "...")]
    let mut iso_clause = "Unknown".to_string();
    for attr in &input.attrs {
        if attr.path().is_ident("pdf_dict") {
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("clause") {
                    let value = meta.value()?;
                    let s: LitStr = value.parse()?;
                    iso_clause = s.value();
                }
                Ok(())
            });
        }
    }

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("FromPdfObject only supports structs with named fields"),
        },
        _ => panic!("FromPdfObject only supports structs"),
    };

    let field_parsers = fields.iter().map(|f| {
        let field_name = &f.ident;
        let field_type = &f.ty;
        
        let mut pdf_key = field_name.as_ref().map(|id| id.to_string()).unwrap_or_default();
        let mut since_version: Option<f32> = None;
        let mut default_expr: Option<String> = None;

        for attr in &f.attrs {
            if attr.path().is_ident("pdf_key") {
                if let Ok(lit) = attr.parse_args::<LitStr>() {
                    pdf_key = lit.value();
                } else {
                    let _ = attr.parse_nested_meta(|meta| {
                        if meta.path.is_ident("name") {
                            let value = meta.value()?;
                            let s: LitStr = value.parse()?;
                            pdf_key = s.value();
                        } else if meta.path.is_ident("since") {
                            let value = meta.value()?;
                            let f: LitFloat = value.parse()?;
                            since_version = Some(f.base10_parse::<f32>()?);
                        } else if meta.path.is_ident("default") {
                            let value = meta.value()?;
                            let s: LitStr = value.parse()?;
                            default_expr = Some(s.value());
                        }
                        Ok(())
                    });
                }
            }
        }

        let version_check = if let Some(v) = since_version {
            quote! {
                if arena.version() < #v {
                    ferruginous_core::Object::Null
                } else {
                    dict.get(&key).cloned().unwrap_or(ferruginous_core::Object::Null)
                }
            }
        } else {
            quote! {
                dict.get(&key).cloned().unwrap_or(ferruginous_core::Object::Null)
            }
        };

        let parser = if let Some(def) = default_expr {
            match syn::parse_str::<syn::Expr>(&def) {
                Ok(def_token) => quote! {
                    if matches!(val, ferruginous_core::Object::Null) {
                        #def_token
                    } else {
                        <#field_type as ferruginous_core::object::FromPdfObject>::from_pdf_object(val, arena)?
                    }
                },
                Err(_) => quote! {
                    compile_error!(concat!("Invalid default expression: ", #def))
                }
            }
        } else {
            quote! {
                <#field_type as ferruginous_core::object::FromPdfObject>::from_pdf_object(val, arena)?
            }
        };

        quote! {
            let #field_name = {
                let key = arena.name(#pdf_key);
                let val = #version_check;
                #parser
            };
        }
    });

    let field_names = fields.iter().map(|f| &f.ident);
    let iso_clause_str = iso_clause;

    let expanded = quote! {
        impl ferruginous_core::object::FromPdfObject for #name {
            fn from_pdf_object(obj: ferruginous_core::Object, arena: &ferruginous_core::PdfArena) -> ferruginous_core::PdfResult<Self> {
                let dict_handle = obj.resolve(arena).as_dict_handle()
                    .ok_or_else(|| ferruginous_core::PdfError::Parse {
                        pos: 0,
                        message: format!("Expected dictionary for {}, got {:?}", stringify!(#name), obj).into()
                    })?;
                
                let dict = arena.get_dict(dict_handle)
                    .ok_or_else(|| ferruginous_core::PdfError::Arena("Missing dictionary in arena".into()))?;

                #(#field_parsers)*

                Ok(Self {
                    #(#field_names),*
                })
            }
        }

        impl ferruginous_core::object::PdfSchema for #name {
            fn iso_clause() -> &'static str {
                #iso_clause_str
            }
        }
    };

    TokenStream::from(expanded)
}
