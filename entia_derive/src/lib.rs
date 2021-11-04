use proc_macro::TokenStream;
use quote::quote;
use syn::{self, DeriveInput, Index, Member};
use syn::{parse_macro_input, Data, DataStruct};

#[proc_macro_derive(Filter)]
pub fn filter(input: TokenStream) -> TokenStream {
    if let DeriveInput {
        ident,
        generics,
        data: Data::Struct(DataStruct { fields, .. }),
        ..
    } = parse_macro_input!(input as DeriveInput)
    {
        let (impl_generics, types_generics, where_clauses) = generics.split_for_impl();
        let filter_body = fields.iter().map(|field| {
            let field_type = field.ty.clone();
            quote! { <#field_type as ::entia::query::filter::Filter>::filter(segment, world) && }
        });
        let code = quote! {
            impl #impl_generics ::entia::query::filter::Filter for #ident #types_generics #where_clauses {
                #[inline]
                fn filter(segment: & ::entia::world::segment::Segment, world: & ::entia::world::World) -> bool {
                    #(#filter_body)* true
                }
            }
        };
        code.into()
    } else {
        TokenStream::new()
    }
}

#[proc_macro_derive(Template)]
pub fn template(input: TokenStream) -> TokenStream {
    if let DeriveInput {
        ident,
        generics,
        data: Data::Struct(DataStruct { fields, .. }),
        ..
    } = parse_macro_input!(input as DeriveInput)
    {
        let (impl_generics, types_generics, where_clauses) = generics.split_for_impl();
        let input_type = fields.iter().map(|field| {
            let field_type = field.ty.clone();
            quote! { <#field_type as ::entia::template::Template>::Input, }
        });
        let declare_type = fields.iter().map(|field| {
            let field_type = field.ty.clone();
            quote! { <#field_type as ::entia::template::Template>::Declare, }
        });
        let state_type = fields.iter().map(|field| {
            let field_type = field.ty.clone();
            quote! { <#field_type as ::entia::template::Template>::State, }
        });
        let declare_body = fields.iter().enumerate().map(|(index, field)| {
            let index = Index::from(index);
            let field_type = field.ty.clone();
            quote! { <#field_type as ::entia::template::Template>::declare(_input.#index, _context.owned()), }
        });
        let initialize_body = fields.iter().enumerate().map(|(index, field)| {
            let index = Index::from(index);
            let field_type = field.ty.clone();
            quote! { <#field_type as ::entia::template::Template>::initialize(_state.#index, _context.owned()), }
        });
        let static_count_body = fields.iter().enumerate().map(|(index, field)| {
            let index = Index::from(index);
            let field_type = field.ty.clone();
            quote! { <#field_type as ::entia::template::Template>::static_count(&_state.#index, _context.owned()) && }
        });
        let dynamic_count_body = fields.iter().enumerate().map(|(index, field)| {
            let index = Index::from(index);
            let field_member = field
                .ident
                .as_ref()
                .map(|name| Member::Named(name.clone()))
                .unwrap_or(Member::Unnamed(index.clone()));
            quote! { self.#field_member.dynamic_count(&_state.#index, _context.owned()); }
        });
        let apply_body = fields.iter().enumerate().map(|(index, field)| {
            let index = Index::from(index);
            let field_member = field
                .ident
                .as_ref()
                .map(|name| Member::Named(name.clone()))
                .unwrap_or(Member::Unnamed(index.clone()));
            quote! { self.#field_member.apply(&_state.#index, _context.owned()); }
        });

        let code = quote! {
            impl #impl_generics ::entia::template::Template for #ident #types_generics #where_clauses {
                type Input = (#(#input_type)*);
                type Declare = (#(#declare_type)*);
                type State = (#(#state_type)*);

                #[inline]
                fn declare(_input: Self::Input, mut _context: ::entia::template::DeclareContext) -> Self::Declare {
                    (#(#declare_body)*)
                }

                #[inline]
                fn initialize(_state: Self::Declare, mut _context: ::entia::template::InitializeContext) -> Self::State {
                    (#(#initialize_body)*)
                }

                #[inline]
                fn static_count(_state: &Self::State, mut _context: ::entia::template::CountContext) -> bool {
                    #(#static_count_body)* true
                }

                #[inline]
                fn dynamic_count(&self, _state: &Self::State, mut _context: ::entia::template::CountContext) {
                    #(#dynamic_count_body)*
                }

                #[inline]
                fn apply(self, _state: &Self::State, mut _context: ::entia::template::ApplyContext) {
                    #(#apply_body)*
                }
            }
        };
        code.into()
    } else {
        TokenStream::new()
    }
}
