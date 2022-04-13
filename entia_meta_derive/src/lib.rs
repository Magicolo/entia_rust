use proc_macro::TokenStream;
use quote::{__private::Span, quote};
use syn::{
    parse_macro_input, punctuated::Punctuated, Attribute, Data, DataEnum, DataStruct, DeriveInput,
    Field, Fields, Ident, Path,
};

#[proc_macro_derive(Meta)]
pub fn filter(input: TokenStream) -> TokenStream {
    if let DeriveInput {
        ident,
        generics,
        data,
        attrs,
        ..
    } = parse_macro_input!(input as DeriveInput)
    {
        let (impl_generics, type_generics, where_clauses) = generics.split_for_impl();
        let meta_path = full_path(ident.span(), ["entia_meta", "meta", "Meta"]);
        let type_id_of_path = full_path(ident.span(), ["std", "any", "TypeId", "of"]);
        let type_name_path = full_path(ident.span(), ["std", "any", "type_name"]);
        let type_path = full_path(ident.span(), ["entia_meta", "meta", "Type"]);
        let type_structure_path =
            full_path(ident.span(), ["entia_meta", "meta", "Type", "Structure"]);
        let structure_macro_path = full_path(ident.span(), ["entia_meta", "structure"]);
        let structure_path = full_path(ident.span(), ["entia_meta", "meta", "Structure"]);
        let structures_path = full_path(ident.span(), ["entia_meta", "meta", "Structures"]);
        let attribute_path = full_path(ident.span(), ["entia_meta", "meta", "Attribute"]);
        let field_path = full_path(ident.span(), ["entia_meta", "meta", "Field"]);
        let get_attributes = |attributes: &[Attribute]| {
            attributes
                .iter()
                .map(|Attribute { path, tokens, .. }| {
                    quote! { #attribute_path { name: #path, content: stringify!(#tokens) }, }
                })
                .collect::<Vec<_>>()
        };
        let attributes = get_attributes(&attrs);
        match data {
            Data::Struct(DataStruct { fields, .. }) => {
                let (fields, structures, new, index) = match fields {
                    syn::Fields::Named(fields) => {
                        let new = fields.named.iter().map(
                            |Field { ident, .. }| quote! { #ident = *values.next()?.cast().ok()?, },
                        );
                        let index = fields.named
                            .iter()
                            .enumerate()
                            .map(|(i, Field { ident, .. })| quote! { stringify!(#ident) | stringify!(#i) => Some(#i), } );
                        (
                            fields
                                .named
                                .iter()
                                .map(|field| (quote! { field.ident }, field.clone()))
                                .collect::<Vec<_>>(),
                            quote! { #structures_path ::Map },
                            quote! { |values| #ident { #(#new)* } },
                            quote! { |name| match name { #(#index)* _ => None } },
                        )
                    }
                    syn::Fields::Unnamed(fields) => {
                        let new = fields
                            .unnamed
                            .iter()
                            .map(|_| quote! { *values.next()?.cast().ok()?, })
                            .collect::<Vec<_>>();
                        let index = fields
                            .unnamed
                            .iter()
                            .enumerate()
                            .map(|(i, _)| quote! { stringify!(#i) => Some(#i), })
                            .collect::<Vec<_>>();
                        (
                            fields
                                .unnamed
                                .iter()
                                .enumerate()
                                .map(|(i, field)| (quote! { stringify!(#i) }, field.clone()))
                                .collect::<Vec<_>>(),
                            quote! { #structures_path ::Tuple },
                            quote! { #ident(#(#new)*) },
                            quote! { |name| match name { #(#index)* _ => None } },
                        )
                    }
                    syn::Fields::Unit => (
                        Vec::new(),
                        quote! { #structures_path ::Unit },
                        quote! { #ident },
                        quote! { |_| None },
                    ),
                };
                let fields = fields.iter().map(|(name, Field { ty, attrs, .. })| {
                    let attributes = get_attributes(attrs);
                    quote! {
                        #field_path {
                            name: #name,
                            meta: #ty ::meta,
                            get:,
                            get_mut:,
                            set:,
                            attributes: &[#(#attributes)*],
                        }
                    }
                });
                /*
                Field {
                        name: "0",
                        meta: usize::meta,
                        get: |instance| match instance.cast_ref() {
                            Some(Fett::A(a)) => Some(a),
                            _ => None,
                        },
                        get_mut: |instance| match instance.cast_mut() {
                            Some(Fett::A(a)) => Some(a),
                            _ => None,
                        },
                        set: |instance, value| match instance.cast_mut() {
                            Some(Fett::A(a)) => {
                                *a = *value.downcast()?;
                                Ok(())
                            }
                            _ => Err(value),
                        },
                        attributes: &[],
                    }
                */
                let code = quote! {
                    #[automatically_derived]
                    impl #impl_generics #meta_path for #ident #type_generics #where_clauses {
                        #[inline]
                        fn meta() -> &'static #type_path {
                            & #type_structure_path(#structure_path {
                                name: #ident,
                                full_name: #type_name_path ::<#ident>,
                                kind: #structures,
                                identifier: #type_id_of_path ::<#ident>,
                                new: #new,
                                index: #index,
                                attributes: &[#(#attributes)*],
                                fields: &[],
                            })
                        }
                    }
                };
                return code.into();
            }
            Data::Enum(DataEnum { variants, .. }) => {
                let code = quote! {
                    #[automatically_derived]
                    impl #impl_generics #meta_path for #ident #type_generics #where_clauses {
                        #[inline]
                        fn meta() -> &'static #type_path {

                        }
                    }
                };
                return code.into();
            }
            _ => {}
        }
    }
    TokenStream::new()
}

fn full_path<'a>(span: Span, segments: impl IntoIterator<Item = &'a str>) -> Path {
    let mut separated = Punctuated::new();
    for segment in segments {
        separated.push(Ident::new(segment, span).into());
    }
    Path {
        segments: separated,
        leading_colon: None,
    }
}
