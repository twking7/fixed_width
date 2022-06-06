use std::{collections::HashMap, ops::Range};
use syn::parse_quote;

pub struct Container {
    pub fixed_width_fn: Option<syn::Ident>,
}

impl Container {
    pub fn from_ast(ast: &syn::DeriveInput) -> Self {
        let mut fixed_width_fn: Option<syn::Ident> = None;

        for attr in &ast.attrs {
            if attr.path.is_ident("fixed_width") {
                match attr.parse_meta() {
                    Ok(syn::Meta::List(metalist)) => {
                        if metalist.nested.len() > 1 {
                            panic!("unexpected multiple values in fixed_width(...)")
                        } else {
                            match metalist.nested.first() {
                                Some(syn::NestedMeta::Meta(syn::Meta::NameValue(
                                    syn::MetaNameValue { path, lit, .. },
                                ))) => match path.get_ident() {
                                    Some(id) if id == "field_def" => {
                                        if fixed_width_fn.is_some() {
                                            panic!("unexpected multiple definition of field_def");
                                        }
                                        match lit {
                                            syn::Lit::Str(litstr) => {
                                                fixed_width_fn = Some(syn::Ident::new(
                                                    &litstr.value(),
                                                    proc_macro2::Span::call_site(),
                                                ))
                                            }
                                            _ => panic!("expected string literal for field_def"),
                                        };
                                    }
                                    Some(id) => {
                                        panic!("unknown fixed_width container attribute: {}", id)
                                    }
                                    _ => unreachable!(),
                                },
                                Some(syn::NestedMeta::Meta(meta)) => {
                                    panic!(
                                        "invalid fixed_width container attribute: {}",
                                        meta.path().get_ident().unwrap()
                                    )
                                }
                                Some(syn::NestedMeta::Lit(_)) => {
                                    panic!("unexpected literal in fixed_width container attribute")
                                }
                                None => panic!("expected fixed_width(field_def = \"...\")"),
                            }
                        }
                    }
                    _ => {
                        panic!("expected fixed_width(...)")
                    }
                }
            }
        }

        Self { fixed_width_fn }
    }
}

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
            if attr.path == parse_quote!(fixed_width) {
                fixed_width_attr_seen += 1;
                if fixed_width_attr_seen > 1 {
                    panic!(
                        "Field: {} has more than 1 fixed_width attribute",
                        field.ident.clone().unwrap(),
                    );
                }

                match attr.parse_meta() {
                    Ok(syn::Meta::List(syn::MetaList { ref nested, .. })) => {
                        let meta_items: Vec<&syn::NestedMeta> = nested.iter().collect();

                        for meta_item in meta_items {
                            if let syn::NestedMeta::Meta(syn::Meta::NameValue(
                                syn::MetaNameValue {
                                    ref path, ref lit, ..
                                },
                            )) = meta_item
                            {
                                if let syn::Lit::Str(ref s) = lit {
                                    let ident = path.get_ident().unwrap().clone();
                                    let mdata = Metadata {
                                        name: ident.clone().to_string(),
                                        value: s.value().to_string(),
                                    };
                                    metadata.insert(ident.clone().to_string(), mdata);
                                } else {
                                    panic!("fixed_width attribute values must be strings");
                                }
                            }
                        }
                    }
                    _ => unreachable!("Did not get a meta list"),
                }
            } else if attr.path == parse_quote!(serde) {
                if let Ok(syn::Meta::List(syn::MetaList { ref nested, .. })) = attr.parse_meta() {
                    let meta_items: Vec<&syn::NestedMeta> = nested.iter().collect();

                    for meta_item in meta_items {
                        if let syn::NestedMeta::Meta(syn::Meta::Path(syn::Path {
                            ref segments,
                            ..
                        })) = meta_item
                        {
                            for segment in segments {
                                if segment.ident == "skip" {
                                    skip = true;
                                }
                            }
                        }
                    }
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

pub struct Metadata {
    pub name: String,
    pub value: String,
}
