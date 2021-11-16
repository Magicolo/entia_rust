use proc_macro::TokenStream;
use quote::__private::Span;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{
    self, AngleBracketedGenericArguments, Binding, BoundLifetimes, DeriveInput, Fields,
    GenericArgument, GenericParam, Ident, Index, Lifetime, LifetimeDef, Member, Path,
    PathArguments, PathSegment, PredicateType, TraitBound, TraitBoundModifier, Type, TypeParam,
    TypeParamBound, TypePath, TypeTuple, WhereClause, WherePredicate,
};
use syn::{parse_macro_input, Data, DataStruct};

/*
TODO: Inject structs don't seem to work very well since their lifetime seem to conflict.
    #[derive(Inject)]
    struct Common<'a> {
        time: &'a Time, // works
        query: Query<'a, &'a Position> // lifetime error
    }
*/
#[proc_macro_derive(Inject)]
pub fn inject(input: TokenStream) -> TokenStream {
    if let DeriveInput {
        ident,
        generics,
        data: Data::Struct(DataStruct { fields, .. }),
        vis,
        ..
    } = parse_macro_input!(input as DeriveInput)
    {
        let world_path = world_path(ident.span());
        let result_path = result_path(ident.span());
        let context_path = full_path(ident.span(), vec!["entia", "inject", "Context"]);
        let inject_path = full_path(ident.span(), vec!["entia", "inject", "Inject"]);
        let depend_path = full_path(ident.span(), vec!["entia", "Depend"]);
        let get_path = |lifetime, item| {
            let mut arguments = Punctuated::new();
            arguments.push(GenericArgument::Lifetime(lifetime));
            if let Some(item) = item {
                arguments.push(GenericArgument::Binding(Binding {
                    ident: Ident::new("Item", ident.span()),
                    ty: item,
                    eq_token: Default::default(),
                }));
            }
            let mut path = full_path(ident.span(), vec!["entia", "inject"]);
            path.segments.push(PathSegment {
                ident: Ident::new("Get", ident.span()),
                arguments: PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                    args: arguments,
                    colon2_token: Default::default(),
                    gt_token: Default::default(),
                    lt_token: Default::default(),
                }),
            });
            path
        };
        let (impl_generics, type_generics, where_clauses) = generics.split_for_impl();
        let struct_generics = fields
            .iter()
            .enumerate()
            .map(|(index, _)| {
                let index = Index::from(index);
                Ident::new(&format!("T{}", index.index), index.span)
            })
            .collect::<Vec<_>>();

        let input_struct_name = Ident::new(&format!("{}Input", ident), ident.span());
        let state_struct_name = Ident::new(&format!("{}State", ident), ident.span());
        let input_types = unpack_fields(&fields).map(|(_, _, field_type)| {
            quote! { <#field_type as #inject_path>::Input }
        });
        let state_types = unpack_fields(&fields).map(|(_, _, field_type)| {
            quote! { <#field_type as #inject_path>::State }
        });
        let initialize_body = unpack_fields(&fields).map(|(index, _, field_type)| {
            quote! { <#field_type as #inject_path>::initialize(_input.#index, _context.owned())? }
        });
        let update_body = unpack_fields(&fields).map(|(index, _, field_type)| {
            quote! { <#field_type as #inject_path>::update(&mut _state.#index, _context.owned())?; }
        });
        let resolve_body = unpack_fields(&fields).map(|(index, _, field_type)| {
            quote! { <#field_type as #inject_path>::resolve(&mut _state.#index, _context.owned())?; }
        });

        let mut get_generics = generics.clone();
        let get_lifetime = Lifetime::new("'__get__", ident.span());
        get_generics
            .params
            .push(GenericParam::Lifetime(LifetimeDef {
                lifetime: get_lifetime.clone(),
                attrs: Vec::new(),
                bounds: generics
                    .lifetimes()
                    .map(|lifetime| lifetime.lifetime.clone())
                    .collect(),
                colon_token: None,
            }));
        for (index, _, field_type) in unpack_fields(&fields) {
            let path = get_path(get_lifetime.clone(), Some(field_type));
            let mut bounds = Punctuated::new();
            bounds.push(TypeParamBound::Trait(TraitBound {
                path,
                lifetimes: None,
                modifier: TraitBoundModifier::None,
                paren_token: Default::default(),
            }));
            get_generics.params.push(GenericParam::Type(TypeParam {
                ident: Ident::new(&format!("__T{}__", index.index), ident.span()),
                attrs: Default::default(),
                bounds,
                colon_token: Default::default(),
                default: Default::default(),
                eq_token: Default::default(),
            }));
        }
        let get_path = get_path(get_lifetime.clone(), None);
        let (get_impl, _, get_where) = get_generics.split_for_impl();
        let get_types = get_generics
            .type_params()
            .map(|parameter| parameter.ident.clone());
        let get_body = match &fields {
            Fields::Named(fields) => {
                let field_gets = fields.named.iter().enumerate().map(|(index, field)| {
                    let index = Index::from(index);
                    let name = field.ident.clone();
                    quote! { #name: self.#index.get(_world) }
                });
                quote! { #ident { #(#field_gets,)* } }
            }
            Fields::Unnamed(fields) => {
                let field_gets = fields.unnamed.iter().enumerate().map(|(index, _)| {
                    let index = Index::from(index);
                    quote! { self.#index.get(_world) }
                });
                quote! { #ident(#(#field_gets,)*) }
            }
            Fields::Unit => quote! { #ident },
        };

        let code = quote! {
            #[automatically_derived]
            #[derive(Debug, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
            #vis struct #input_struct_name<#(#struct_generics,)*>(#(#struct_generics,)*);

            #[automatically_derived]
            #[derive(Debug, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, #depend_path)]
            #vis struct #state_struct_name<#(#struct_generics,)*>(#(#struct_generics,)*);

            #[automatically_derived]
            impl #impl_generics #inject_path for #ident #type_generics #where_clauses {
                type Input = #input_struct_name<#(#input_types,)*>;
                type State = #state_struct_name<#(#state_types,)*>;

                fn initialize(_input: Self::Input, mut _context: #context_path) -> #result_path<Self::State> {
                    Ok(#state_struct_name(#(#initialize_body,)*))
                }

                #[inline]
                fn update(_state: &mut Self::State, mut _context: #context_path) -> #result_path {
                    #(#update_body)*
                    Ok(())
                }

                #[inline]
                fn resolve(_state: &mut Self::State, mut _context: #context_path) -> #result_path {
                    #(#resolve_body)*
                    Ok(())
                }
            }

            #[automatically_derived]
            #[allow(non_camel_case_types)]
            impl #get_impl #get_path for #state_struct_name<#(#get_types,)*> #get_where {
                type Item = #ident #type_generics;

                #[inline]
                fn get(&#get_lifetime mut self, _world: &#get_lifetime #world_path) -> Self::Item {
                    #get_body
                }
            }
        };
        code.into()
    } else {
        TokenStream::new()
    }
}

#[proc_macro_derive(Filter)]
pub fn filter(input: TokenStream) -> TokenStream {
    if let DeriveInput {
        ident,
        generics,
        data: Data::Struct(DataStruct { fields, .. }),
        ..
    } = parse_macro_input!(input as DeriveInput)
    {
        let world_path = world_path(ident.span());
        let segment_path = segment_path(ident.span());
        let filter_path = full_path(ident.span(), vec!["entia", "query", "filter", "Filter"]);
        let (impl_generics, type_generics, where_clauses) = generics.split_for_impl();
        let filter_body = unpack_fields(&fields).map(|(_, _, field_type)| {
            quote! { <#field_type as #filter_path>::filter(_segment, _world) && }
        });
        let code = quote! {
            #[automatically_derived]
            impl #impl_generics #filter_path for #ident #type_generics #where_clauses {
                #[inline]
                fn filter(_segment: & #segment_path, _world: & #world_path) -> bool {
                    #(#filter_body)* true
                }
            }
        };
        code.into()
    } else {
        TokenStream::new()
    }
}

#[proc_macro_derive(Depend)]
pub fn depend(input: TokenStream) -> TokenStream {
    if let DeriveInput {
        ident,
        mut generics,
        data: Data::Struct(DataStruct { fields, .. }),
        ..
    } = parse_macro_input!(input as DeriveInput)
    {
        let world_path = world_path(ident.span());
        let depend_path = full_path(ident.span(), vec!["entia", "depend", "Depend"]);
        let dependency_path = full_path(ident.span(), vec!["entia", "depend", "Dependency"]);
        for parameter in generics.type_params_mut() {
            parameter.bounds.push(TypeParamBound::Trait(TraitBound {
                path: depend_path.clone(),
                lifetimes: Default::default(),
                modifier: TraitBoundModifier::None,
                paren_token: Default::default(),
            }))
        }

        let (impl_generics, type_generics, where_clauses) = generics.split_for_impl();
        let depend_body = unpack_fields(&fields).map(|(_, member, field_type)| {
            quote! { dependencies.append(&mut <#field_type as #depend_path>::depend(&self.#member, _world)); }
        });

        let code = quote! {
            #[automatically_derived]
            unsafe impl #impl_generics #depend_path for #ident #type_generics #where_clauses {
                #[inline]
                fn depend(&self, _world: & #world_path) -> Vec<#dependency_path> {
                    let mut dependencies = Vec::new();
                    #(#depend_body)*
                    dependencies
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
        let result_path = result_path(ident.span());
        let context_path = full_path(ident.span(), vec!["entia", "template"]);
        let template_path = full_path(ident.span(), vec!["entia", "template", "Template"]);
        let spawn_template_path =
            full_path(ident.span(), vec!["entia", "template", "SpawnTemplate"]);
        let leaf_template_path = full_path(ident.span(), vec!["entia", "template", "LeafTemplate"]);
        let static_template_path =
            full_path(ident.span(), vec!["entia", "template", "StaticTemplate"]);
        let indirect_lifetime = Lifetime::new("'__indirect__", ident.span());
        let indirect_path = |marker| {
            let mut arguments = Punctuated::new();
            arguments.push(GenericArgument::Lifetime(indirect_lifetime.clone()));
            arguments.push(GenericArgument::Type(Type::Path(TypePath {
                path: full_path(ident.span(), vec!["entia", "template", marker]),
                qself: Default::default(),
            })));
            let mut path = full_path(ident.span(), vec!["entia", "core"]);
            path.segments.push(PathSegment {
                ident: Ident::new("Indirect", ident.span()),
                arguments: PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                    args: arguments,
                    colon2_token: Default::default(),
                    gt_token: Default::default(),
                    lt_token: Default::default(),
                }),
            });
            path
        };
        let spawn_indirect_path = indirect_path("SpawnMarker");
        let static_indirect_path = indirect_path("StaticMarker");
        let leaf_indirect_path = indirect_path("LeafMarker");

        let (impl_generics, type_generics, where_clauses) = generics.split_for_impl();
        let input_type = unpack_fields(&fields).map(|(_, _, field_type)| {
            quote! { <#field_type as #template_path>::Input, }
        });
        let declare_type = unpack_fields(&fields).map(|(_, _, field_type)| {
            quote! { <#field_type as #template_path>::Declare, }
        });
        let state_type = unpack_fields(&fields).map(|(_, _, field_type)| {
            quote! { <#field_type as #template_path>::State, }
        });
        let declare_body = unpack_fields(&fields).map(|(index, _, field_type)| {
            quote! { <#field_type as #template_path>::declare(_input.#index, _context.owned()), }
        });
        let initialize_body = unpack_fields(&fields).map(|(index, _, field_type)| {
            quote! { <#field_type as #template_path>::initialize(_state.#index, _context.owned()), }
        });
        let static_count_body = unpack_fields(&fields).map(|(index, _, field_type)| {
            quote! { <#field_type as #template_path>::static_count(&_state.#index, _context.owned())? && }
        });
        let dynamic_count_body = unpack_fields(&fields).map(|(index, member, _)| {
            quote! { self.#member.dynamic_count(&_state.#index, _context.owned()); }
        });
        let apply_body = unpack_fields(&fields).map(|(index, member, _)| {
            quote! { self.#member.apply(&_state.#index, _context.owned()); }
        });

        let marker_impl = |trait_path: &Path, indirect_path: &Path| {
            let mut where_clauses = where_clauses.cloned().unwrap_or_else(|| WhereClause {
                predicates: Default::default(),
                where_token: Default::default(),
            });
            where_clauses
                .predicates
                .push(WherePredicate::Type(PredicateType {
                    bounded_ty: Type::Tuple(TypeTuple {
                        elems: unpack_fields(&fields)
                            .map(|(_, _, field_type)| field_type)
                            .collect(),
                        paren_token: Default::default(),
                    }),
                    bounds: vec![TypeParamBound::Trait(TraitBound {
                        path: indirect_path.clone(),
                        lifetimes: Some(BoundLifetimes {
                            lifetimes: vec![LifetimeDef {
                                lifetime: indirect_lifetime.clone(),
                                attrs: Vec::new(),
                                bounds: Default::default(),
                                colon_token: Default::default(),
                            }]
                            .into_iter()
                            .collect(),
                            ..Default::default()
                        }),
                        paren_token: Default::default(),
                        modifier: TraitBoundModifier::None,
                    })]
                    .into_iter()
                    .collect(),
                    colon_token: Default::default(),
                    lifetimes: Default::default(),
                }));
            quote! { unsafe impl #impl_generics #trait_path for #ident #type_generics #where_clauses { } }
        };
        let spawn_marker_impl = marker_impl(&spawn_template_path, &spawn_indirect_path);
        let leaf_marker_impl = marker_impl(&leaf_template_path, &leaf_indirect_path);
        let static_marker_impl = marker_impl(&static_template_path, &static_indirect_path);

        let code = quote! {
            #[automatically_derived]
            impl #impl_generics #template_path for #ident #type_generics #where_clauses {
                type Input = (#(#input_type)*);
                type Declare = (#(#declare_type)*);
                type State = (#(#state_type)*);

                #[inline]
                fn declare(_input: Self::Input, mut _context: #context_path::DeclareContext) -> Self::Declare {
                    (#(#declare_body)*)
                }

                #[inline]
                fn initialize(_state: Self::Declare, mut _context: #context_path::InitializeContext) -> Self::State {
                    (#(#initialize_body)*)
                }

                #[inline]
                fn static_count(_state: &Self::State, mut _context: #context_path::CountContext) -> #result_path<bool> {
                    Ok(#(#static_count_body)* true)
                }

                #[inline]
                fn dynamic_count(&self, _state: &Self::State, mut _context: #context_path::CountContext) {
                    #(#dynamic_count_body)*
                }

                #[inline]
                fn apply(self, _state: &Self::State, mut _context: #context_path::ApplyContext) {
                    #(#apply_body)*
                }
            }

            #spawn_marker_impl
            #leaf_marker_impl
            #static_marker_impl
        };
        code.into()
    } else {
        TokenStream::new()
    }
}

fn world_path(span: Span) -> Path {
    full_path(span, vec!["entia", "world", "World"])
}

fn result_path(span: Span) -> Path {
    full_path(span, vec!["entia", "error", "Result"])
}

fn segment_path(span: Span) -> Path {
    full_path(span, vec!["entia", "world", "segment", "Segment"])
}

fn unpack_fields(fields: &Fields) -> impl Iterator<Item = (Index, Member, Type)> + '_ {
    fields.iter().enumerate().map(|(index, field)| {
        let index = Index::from(index);
        let member = field
            .ident
            .as_ref()
            .map_or(Member::Unnamed(index.clone()), |name| {
                Member::Named(name.clone())
            });
        (index, member, field.ty.clone())
    })
}

fn full_path<'a>(span: Span, segments: impl IntoIterator<Item = &'a str>) -> Path {
    let mut separated = Punctuated::new();
    for segment in segments {
        separated.push(Ident::new(segment, span).into());
    }
    Path {
        segments: separated,
        leading_colon: Some(Default::default()),
    }
}
