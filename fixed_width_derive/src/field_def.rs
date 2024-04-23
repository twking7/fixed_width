use std::{collections::HashMap, ops::Range};
use syn::LitStr;

pub struct Container {
    pub fixed_width_fn: Option<syn::Ident>,
}

impl Container {
    pub fn from_ast(ast: &syn::DeriveInput) -> Self {
        let mut fixed_width_fn: Option<syn::Ident> = None;

        for attr in &ast.attrs {
            if attr.path().is_ident("fixed_width") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("field_def") {
                        let value = meta.value().expect("expected to find an expression, ie fixed_width(field_def = function_name)");
                        let fixed_width_fn_name: LitStr = value.parse().expect("expected to find a function name, ie fixed_width(field_def = function_name)");

                        if fixed_width_fn.is_some() {
                            panic!("expected only 1 function to be specified for the field_def");
                        } else {
                            fixed_width_fn = Some(syn::Ident::new(&fixed_width_fn_name.value(), proc_macro2::Span::call_site()));
                        }
                    }
                    Ok(())
                }).expect("expected fixed_width(...)");
            }
        }

        Self { fixed_width_fn }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct FieldDef {
    pub ident: syn::Ident,
    pub field_type: syn::Type,
    pub name: String,
    pub pad_with: char,
    pub range: Range<usize>,
    pub justify: String,
}

pub struct Context {
    pub field: syn::Field,
    pub skip: bool,
    pub metadata: HashMap<String, Metadata>,
}

impl Context {
    pub fn from_field(field: &syn::Field) -> Self {
        let mut fixed_width_attr_seen = 0;
        let mut metadata = HashMap::new();
        let mut skip = false;

        for attr in &field.attrs {
            if attr.path().is_ident("fixed_width") {
                fixed_width_attr_seen += 1;
                if fixed_width_attr_seen > 1 {
                    panic!(
                        "Field: {} has more than 1 fixed_width attribute",
                        field.ident.clone().unwrap(),
                    );
                }

                let parse_result = attr.parse_nested_meta(|meta| {
                    let ident = meta.path.get_ident().unwrap().clone();
                    let s: LitStr = meta
                        .value()
                        .expect(
                            "expected to find an expression, ie fixed_width(<field> = <metadata>)",
                        )
                        .parse()
                        .expect("fixed width values must be strings");

                    let mdata = Metadata {
                        name: ident.clone().to_string(),
                        value: s.value().to_string(),
                    };
                    metadata.insert(ident.clone().to_string(), mdata);
                    Ok(())
                });

                if parse_result.is_err() {
                    panic!(
                        "could not parse fixed_width metadata for field: {}",
                        field.ident.clone().unwrap()
                    );
                }
            } else if attr.path().is_ident("serde") {
                let parse_result = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("skip") {
                        skip = true;
                    }
                    Ok(())
                });

                if parse_result.is_err() {
                    panic!(
                        "could not parse serde metadata for field: {}",
                        field.ident.clone().unwrap()
                    );
                }
            }
        }

        Self {
            field: field.clone(),
            skip,
            metadata,
        }
    }

    pub fn field_name(&self) -> String {
        self.field.ident.clone().unwrap().to_string()
    }
}

#[allow(dead_code)]
pub struct Metadata {
    pub name: String,
    pub value: String,
}
