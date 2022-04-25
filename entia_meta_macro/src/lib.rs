use proc_macro::TokenStream;
use quote::{__private::Span, quote, quote_spanned, spanned::Spanned, ToTokens};
use syn::{
    parse_macro_input, parse_quote, Attribute, ConstParam, Data, DataEnum, DataStruct, DataUnion,
    DeriveInput, Field, Fields, FnArg, GenericParam, Generics, Ident, Item, ItemConst, ItemEnum,
    ItemFn, ItemMod, ItemStatic, ItemStruct, ItemUnion, Lifetime, LifetimeDef, LitInt, Pat, PatBox,
    PatIdent, PatReference, PatType, Path, PathSegment, Receiver, ReturnType, Signature, Type,
    TypeParam, TypeReference, Visibility,
};

struct Context {
    meta: Path,
}

#[proc_macro_derive(Meta)]
pub fn derive(input: TokenStream) -> TokenStream {
    fn body(
        context: &Context,
        meta: impl ToTokens,
        ident: Ident,
        generics: Generics,
        suffix: Ident,
    ) -> impl ToTokens {
        let meta_path = &context.meta;
        let (impl_generics, type_generics, where_clauses) = generics.split_for_impl();
        quote_spanned!(ident.span() =>
            #[automatically_derived]
            impl #impl_generics #meta_path::Meta<&'static #meta_path::#suffix> for #ident #type_generics #where_clauses {
                #[inline]
                fn meta() -> &'static #meta_path::#suffix {
                    &#meta
                }
            }

            #[automatically_derived]
            impl #impl_generics #meta_path::Meta<#meta_path::Data> for #ident #type_generics #where_clauses {
                #[inline]
                fn meta() -> #meta_path::Data {
                    #meta_path::Data::#suffix(<Self as #meta_path::Meta<&'static #meta_path::#suffix>>::meta())
                }
            }

            #[automatically_derived]
            impl #impl_generics #meta_path::Meta<#meta_path::module::Member> for #ident #type_generics #where_clauses {
                #[inline]
                fn meta() -> #meta_path::module::Member {
                    #meta_path::module::Member::#suffix(<Self as #meta_path::Meta<&'static #meta_path::#suffix>>::meta)
                }
            }
        )
    }

    let DeriveInput {
        attrs,
        vis,
        ident,
        generics,
        data,
    } = parse_macro_input!(input as DeriveInput);
    let meta_path = path(["entia", "meta"]);
    let context = Context {
        meta: meta_path.clone(),
    };
    match data {
        Data::Struct(DataStruct {
            fields,
            semi_token,
            struct_token,
        }) => {
            let mut item = ItemStruct {
                attrs,
                fields,
                generics,
                ident,
                vis,
                semi_token,
                struct_token,
            };
            let meta = context.structure(&mut item);
            let ItemStruct {
                ident, generics, ..
            } = item;
            body(
                &context,
                meta,
                ident,
                generics,
                Ident::new("Structure", Span::call_site()),
            )
            .to_token_stream()
        }
        Data::Enum(DataEnum {
            enum_token,
            brace_token,
            variants,
        }) => {
            let mut item = ItemEnum {
                attrs,
                variants,
                generics,
                ident,
                vis,
                brace_token,
                enum_token,
            };
            let meta = context.enumeration(&mut item);
            let ItemEnum {
                ident, generics, ..
            } = item;
            body(
                &context,
                meta,
                ident,
                generics,
                Ident::new("Enumeration", Span::call_site()),
            )
            .to_token_stream()
        }
        Data::Union(DataUnion {
            fields,
            union_token,
        }) => {
            let mut item = ItemUnion {
                attrs,
                fields,
                generics,
                ident,
                vis,
                union_token,
            };
            let meta = context.union(&mut item);
            let ItemUnion {
                ident, generics, ..
            } = item;
            body(
                &context,
                meta,
                ident,
                generics,
                Ident::new("Union", Span::call_site()),
            )
            .to_token_stream()
        }
    }
    .into()
}

#[proc_macro_attribute]
pub fn meta(attribute: TokenStream, item: TokenStream) -> TokenStream {
    let meta_path = if attribute.is_empty() {
        path(["entia", "meta"])
    } else {
        parse_macro_input!(attribute as Path)
    };

    let context = Context { meta: meta_path };
    match parse_macro_input!(item as Item) {
        Item::Mod(mut module) => {
            let meta = context.module(&mut module);
            if let Some((_, content)) = &mut module.content {
                let meta_path = &context.meta;
                content.push(parse_quote! { pub static META: #meta_path::Module = #meta; });
            }
            println!("{}", module.to_token_stream());
            module.to_token_stream().into()
        }
        // Item::Const(_) => todo!(),
        // Item::Enum(_) => todo!(),
        // Item::Fn(_) => todo!(),
        // Item::Impl(_) => todo!(),
        // Item::Struct(_) => todo!(),
        // Item::Trait(_) => todo!(),
        // Item::Union(_) => todo!(),
        _ => panic!("Invalid item."),
    }
}

impl Context {
    pub fn access(&self, visibility: &Visibility) -> impl ToTokens {
        let meta_path = &self.meta;
        match visibility {
            Visibility::Public(_) => quote! { #meta_path::Access::Public },
            Visibility::Crate(_) => quote! { #meta_path::Access::Crate },
            Visibility::Restricted(restricted) => {
                if let Some(_) = restricted.in_token {
                    quote! { #meta_path::Access::Super }
                } else {
                    quote! { #meta_path::Access::Crate }
                }
            }
            Visibility::Inherited => quote! { #meta_path::Access::Private },
        }
    }

    pub fn attribute(&self, Attribute { path, tokens, .. }: &Attribute) -> impl ToTokens {
        let meta_path = &self.meta;
        quote_spanned!(path.__span() => #meta_path::Attribute { path: stringify!(#path), content: stringify!(#tokens) })
    }

    pub fn module(
        &self,
        ItemMod {
            attrs,
            content,
            ident,
            vis,
            ..
        }: &mut ItemMod,
    ) -> impl ToTokens {
        let meta_path = &self.meta;
        let access = self.access(vis);
        let attributes = attrs.iter().map(|attribute| self.attribute(attribute));
        let (names, members) = content
            .iter_mut()
            .flat_map(|(_, content)| content)
            .filter_map(|item| match item {
                Item::Const(item) => {
                    let meta = self.constant(item);
                    Some((&item.ident, quote_spanned!(item.ident.span() => #meta_path::module::Member::Constant(#meta))))
                }
                Item::Static(item) => {
                    let meta = self.r#static(item);
                    Some((&item.ident, quote_spanned!(item.ident.span() => #meta_path::module::Member::Static(#meta))))
                }
                Item::Enum(item) => {
                    item.attrs.push(parse_quote! { #[derive(#meta_path::Meta)] });
                    let name = &item.ident;
                    Some((name, quote_spanned!(item.ident.span() => #meta_path::module::Member::Enumeration(<#name as #meta_path::Meta<&'static #meta_path::Enumeration>>::meta))))
                }
                Item::Fn(item) => {
                    let meta = self.function(item);
                    Some((&item.sig.ident, quote_spanned!(item.sig.ident.span() => #meta_path::module::Member::Function(#meta))))
                }
                Item::Struct(item) => {
                    item.attrs.push(parse_quote! { #[derive(#meta_path::Meta)] });
                    let name = &item.ident;
                    Some((name, quote_spanned!(item.ident.span() => #meta_path::module::Member::Structure(<#name as #meta_path::Meta<&'static #meta_path::Structure>>::meta))))
                }
                Item::Mod(item) => {
                    let meta = self.module(item);
                    if let Some((_, content)) = &mut item.content {
                        let meta_path = &self.meta;
                        content.push(parse_quote! { pub static META: #meta_path::Module = #meta; });
                    }
                    let name = &item.ident;
                    Some((name, quote_spanned!(name.span() => #meta_path::module::Member::Module(&#name::META))))
                }
                // Item::Impl(_) => todo!(),
                // Item::Trait(_) => todo!(),
                // Item::Union(_) => todo!(),
                _ => None,
            }).unzip::<_, _, Vec<_>, Vec<_>>();
        let index = self.index(names.into_iter().cloned().map(Some));
        quote_spanned! (ident.span() => #meta_path::Module {
            access: #access,
            name: stringify!(#ident),
            attributes: &[#(#attributes,)*],
            members: #meta_path::Index(&[#(#members,)*], #index),
        })
    }

    pub fn generic(&self, generic: &GenericParam) -> impl ToTokens {
        let meta_path = &self.meta;
        match generic {
            GenericParam::Type(TypeParam {
                attrs,
                ident,
                default,
                ..
            }) => {
                let attributes = attrs.iter().map(|attribute| self.attribute(attribute));
                let default = default.as_ref().map_or_else(
                    || quote! { None },
                    |default| quote! { Some(<#default as #meta_path::Meta>::meta) },
                );
                quote_spanned!(ident.span() => #meta_path::Generic::Type(#meta_path::generic::Type {
                    name: stringify!(#ident),
                    attributes: &[#(#attributes,)*],
                    default: #default,
                }))
            }
            GenericParam::Lifetime(LifetimeDef {
                attrs,
                lifetime: Lifetime { ident, .. },
                ..
            }) => {
                let attributes = attrs.iter().map(|attribute| self.attribute(attribute));
                quote_spanned!(ident.span() => #meta_path::Generic::Lifetime(#meta_path::generic::Lifetime {
                    name: stringify!(#ident),
                    attributes: &[#(#attributes,)*],
                }))
            }
            GenericParam::Const(ConstParam {
                attrs,
                default,
                ident,
                ty,
                ..
            }) => {
                let attributes = attrs.iter().map(|attribute| self.attribute(attribute));
                let default = default.as_ref().map_or_else(
                    || quote! { None },
                    |default| quote! { Some(#meta_path::Value::from(#default)) },
                );
                quote_spanned!(ident.span() => #meta_path::Generic::Constant(#meta_path::generic::Constant {
                    name: stringify!(#ident),
                    default: #default,
                    meta: <#ty as #meta_path::Meta>::meta,
                    attributes: &[#(#attributes,)*],
                }))
            }
        }
    }

    pub fn field(
        &self,
        Field {
            attrs,
            ident,
            ty,
            vis,
            ..
        }: &Field,
        parent: &Ident,
        index: usize,
        deconstruct: &impl ToTokens,
    ) -> impl ToTokens {
        let meta_path = &self.meta;
        let access = self.access(vis);
        let attributes = attrs.iter().map(|attribute| self.attribute(attribute));
        let name = ident.as_ref().map_or_else(
            || {
                let index = LitInt::new(&index.to_string(), ty.__span());
                quote! { #index }
            },
            |name| quote! { #name },
        );
        let key = ident.as_ref().map_or_else(
            || {
                let index = Ident::new(&format!("_{}", index), ty.__span());
                quote! { #index }
            },
            |name| quote! { #name },
        );
        quote_spanned!(ty.__span() => #meta_path::Field {
            access: #access,
            name: stringify!(#name),
            meta: <#ty as #meta_path::Meta<#meta_path::Data>>::meta,
            get: |instance| match instance.downcast_ref::<#parent>()? {
                #deconstruct => Some(#key),
                #[allow(unreachable_patterns)]
                _ => None,
            },
            get_mut: |instance| match instance.downcast_mut::<#parent>()? {
                #deconstruct => Some(#key),
                #[allow(unreachable_patterns)]
                _ => None,
            },
            set: |instance, value| match instance.downcast_mut::<#parent>()? {
                #deconstruct => Some(::std::mem::swap(#key, value.downcast_mut::<#ty>()?)),
                #[allow(unreachable_patterns)]
                _ => None,
            },
            attributes: &[#(#attributes,)*],
        })
    }

    pub fn fields(
        &self,
        fields: &Fields,
        ident: &Ident,
        path: &Path,
    ) -> (
        impl ToTokens,
        impl ToTokens,
        impl ToTokens,
        impl ToTokens,
        impl ToTokens,
    ) {
        let meta_path = &self.meta;
        let (construct, deconstruct, names, values, fields) = match fields {
            Fields::Named(fields) => {
                let pairs = fields.named.iter().map(
                    |Field { ident, ty, .. }| quote! { #ident: values.next()?.downcast::<#ty>().ok()? },
                );
                let (keys, values) = fields
                    .named
                    .iter()
                    .map(|Field { ident, .. }| (ident, quote! { #meta_path::Value::from(#ident) }))
                    .unzip::<_, _, Vec<_>, Vec<_>>();
                let construct = quote_spanned!(ident.span() => #path { #(#pairs,)* });
                let deconstruct = quote_spanned!(ident.span() => #path { #(#keys,)* });
                let (names, fields) = fields
                    .named
                    .iter()
                    .enumerate()
                    .map(|(index, field)| {
                        (&field.ident, self.field(field, ident, index, &deconstruct))
                    })
                    .unzip::<_, _, Vec<_>, Vec<_>>();
                (construct, deconstruct, names, values, fields)
            }
            Fields::Unnamed(fields) => {
                let pairs = fields
                    .unnamed
                    .iter()
                    .map(|Field { ty, .. }| quote! { values.next()?.downcast::<#ty>().ok()? });
                let (keys, values) = (0..fields.unnamed.len())
                    .map(|index| {
                        let key = Ident::new(&format!("_{}", index), Span::call_site());
                        (key.clone(), quote! { #meta_path::Value::from(#key) })
                    })
                    .unzip::<_, _, Vec<_>, Vec<_>>();
                let construct = quote_spanned!(ident.span() => #path (#(#pairs,)*));
                let deconstruct = quote_spanned!(ident.span() => #path (#(#keys,)*));
                let (names, fields) = fields
                    .unnamed
                    .iter()
                    .enumerate()
                    .map(|(index, field)| {
                        (&field.ident, self.field(field, ident, index, &deconstruct))
                    })
                    .unzip::<_, _, Vec<_>, Vec<_>>();
                (construct, deconstruct, names, values, fields)
            }
            Fields::Unit => (
                quote_spanned!(ident.span() => #path),
                quote_spanned!(ident.span() => #path),
                vec![],
                vec![],
                vec![],
            ),
        };

        let new = quote_spanned!(ident.span() => |values| Some(Box::new(#construct)));
        let values = quote_spanned!(ident.span() => |instance| match *instance.downcast::<#ident>()? {
            #deconstruct => Ok(Box::new([#(#values,)*])),
            #[allow(unreachable_patterns)]
            instance => Err(Box::new(instance)),
        });
        let index = self.index(names.into_iter().cloned());
        let fields = quote_spanned!(ident.span() => #meta_path::Index(&[#(#fields,)*], #index));
        (construct, deconstruct, new, values, fields)
    }

    pub fn structure(
        &self,
        ItemStruct {
            ident,
            vis,
            attrs,
            generics,
            fields,
            ..
        }: &mut ItemStruct,
    ) -> impl ToTokens {
        let meta_path = &self.meta;
        let access = self.access(vis);
        let attributes = attrs.iter().map(|attribute| self.attribute(attribute));
        let generics = generics.params.iter().map(|generic| self.generic(generic));
        let (_, _, new, values, fields) = self.fields(fields, ident, &parse_quote!(#ident));
        quote_spanned!(ident.span() => #meta_path::Structure {
            access: #access,
            name: stringify!(#ident),
            size: ::std::mem::size_of::<#ident>(),
            identifier: ::std::any::TypeId::of::<#ident>,
            new: #new,
            values: #values,
            attributes: &[#(#attributes,)*],
            generics: &[#(#generics,)*],
            fields: #fields,
        })
    }

    pub fn enumeration(
        &self,
        ItemEnum {
            ident,
            vis,
            attrs,
            generics,
            variants,
            ..
        }: &ItemEnum,
    ) -> impl ToTokens {
        let meta_path = &self.meta;
        let access = self.access(vis);
        let attributes = attrs.iter().map(|attribute| self.attribute(attribute));
        let generics = generics.params.iter().map(|generic| self.generic(generic));
        let (names, variants) = variants
            .iter()
            .map(|variant| {
                let attributes = variant
                    .attrs
                    .iter()
                    .map(|attribute| self.attribute(attribute));
                let name = &variant.ident;
                let (_, deconstruct, new, values, fields) =
                    self.fields(&variant.fields, ident, &parse_quote!(#ident::#name));
                (
                    (&variant.ident, deconstruct),
                    quote_spanned!(variant.ident.span() => #meta_path::Variant {
                        name: stringify!(#name),
                        new: #new,
                        values: #values,
                        attributes: &[#(#attributes,)*],
                        fields: #fields,
                    }),
                )
            })
            .unzip::<_, _, Vec<_>, Vec<_>>();
        let value_matches = names
            .iter()
            .enumerate()
            .map(|(index, (name, deconstruct))| {
                let index = LitInt::new(&format!("{}", index), name.span());
                quote_spanned!(ident.span() => #deconstruct => Some(#index))
            });
        let value_index = quote_spanned!(ident.span() => |instance| match instance.downcast_ref::<#ident>()? {
            #(#value_matches,)*
            #[allow(unreachable_patterns)]
            _ => None
        });
        let index = self.index(names.into_iter().map(|(name, _)| Some(name.clone())));
        quote_spanned!(ident.span() => #meta_path::Enumeration {
            access: #access,
            name: stringify!(#ident),
            size: ::std::mem::size_of::<#ident>(),
            identifier: ::std::any::TypeId::of::<#ident>,
            generics: &[#(#generics,)*],
            attributes: &[#(#attributes,)*],
            variants: #meta_path::Index(&[#(#variants,)*], #index),
            index: #value_index,
        })
    }

    pub fn function(
        &self,
        ItemFn {
            sig, attrs, vis, ..
        }: &ItemFn,
    ) -> impl ToTokens {
        fn borrow_ty(ty: &Type) -> Option<bool> {
            match ty {
                Type::Reference(TypeReference { mutability, .. }) => Some(mutability.is_some()),
                _ => None,
            }
        }

        fn borrow_input(input: &FnArg) -> Option<bool> {
            match input {
                FnArg::Receiver(Receiver {
                    reference,
                    mutability,
                    ..
                }) => reference.as_ref().map(|_| mutability.is_some()),
                FnArg::Typed(PatType { ty, .. }) => borrow_ty(ty),
            }
        }

        fn borrow_output(output: &ReturnType) -> Option<bool> {
            match output {
                ReturnType::Default => None,
                ReturnType::Type(_, ty) => borrow_ty(ty),
            }
        }

        let meta_path = &self.meta;
        let access = self.access(vis);
        let attributes = attrs.iter().map(|attribute| self.attribute(attribute));
        let signature = self.signature(sig);
        let name = &sig.ident;
        let invoke_inputs = sig.inputs.iter().map(|input| {
            let argument = match borrow_input(input) {
                Some(true) => quote_spanned!(name.span() => exclusive()),
                Some(false) => quote_spanned!(name.span() => shared()),
                None => quote_spanned!(name.span() => owned().ok()),
            };
            quote_spanned!(name.span() => arguments.next()?.#argument?)
        });
        let invoke_body = quote_spanned!(name.span() => #name(#(#invoke_inputs),*));
        let invoke_output = match borrow_output(&sig.output) {
            Some(true) => quote_spanned!(name.span() => Exclusive(#invoke_body)),
            Some(false) => quote_spanned!(name.span() => Shared(#invoke_body)),
            None => quote_spanned!(name.span() => Owned(#meta_path::Value::from(#invoke_body))),
        };
        quote_spanned!(sig.ident.span() => #meta_path::Function {
            access: #access,
            signature: #signature,
            invoke: |arguments| Some(#meta_path::Argument::#invoke_output),
            attributes: &[#(#attributes,)*],
        })
    }

    pub fn signature(
        &self,
        Signature {
            ident,
            inputs,
            output,
            asyncness,
            constness,
            unsafety,
            generics,
            ..
        }: &Signature,
    ) -> impl ToTokens {
        let meta_path = &self.meta;
        let modifiers = asyncness.map_or(0 as u8, |_| 1 << 0)
            | constness.map_or(0 as u8, |_| 1 << 1)
            | unsafety.map_or(0 as u8, |_| 1 << 2);
        let generics = generics.params.iter().map(|generic| self.generic(generic));
        let parameters = inputs
            .iter()
            .enumerate()
            .map(|(index, input)| self.parameter(input, index));
        let output = match output {
            ReturnType::Default => quote! { () },
            ReturnType::Type(_, output) => {
                let output = Self::unwrap(output);
                quote_spanned!(output.__span() => #output)
            }
        };
        quote_spanned!(ident.span() => #meta_path::Signature {
            modifiers: #modifiers,
            name: stringify!(#ident),
            meta: <#output as #meta_path::Meta>::meta,
            generics: &[#(#generics,)*],
            parameters: &[#(#parameters,)*],
        })
    }

    pub fn parameter(&self, parameter: &FnArg, index: usize) -> impl ToTokens {
        fn borrow(
            context: &Context,
            reference: Option<Option<&Lifetime>>,
            mutability: bool,
        ) -> impl ToTokens {
            let meta_path = &context.meta;
            match (reference, mutability) {
                (Some(_), true) => {
                    // TODO: Add 'lifetime' to 'Borrow'
                    quote! { #meta_path::function::Borrow::Exclusive }
                }
                (Some(_), false) => {
                    // TODO: Add 'lifetime' to 'Borrow'
                    quote! { #meta_path::function::Borrow::Shared }
                }
                (None, _) => {
                    quote! { #meta_path::function::Borrow::Owned }
                }
            }
        }

        fn borrow_ty(context: &Context, ty: &Type) -> impl ToTokens {
            match ty {
                Type::Reference(TypeReference { elem, .. })
                    if matches!(&**elem, Type::Reference(..)) =>
                {
                    borrow_ty(context, elem)
                }
                Type::Reference(TypeReference {
                    lifetime,
                    mutability,
                    ..
                }) => {
                    borrow(context, Some(lifetime.as_ref()), mutability.is_some()).to_token_stream()
                }
                _ => borrow(context, None, false).to_token_stream(),
            }
        }

        fn name(pattern: &Pat) -> Option<String> {
            match pattern {
                Pat::Ident(PatIdent { ident, .. }) => Some(ident.to_string()),
                Pat::Wild(_) => Some("_".into()),
                Pat::Reference(PatReference { pat, .. }) => name(pat),
                Pat::Box(PatBox { pat, .. }) => name(pat),
                _ => None,
            }
        }

        let meta_path = &self.meta;
        let (name, attributes, meta, borrow) = match parameter {
            FnArg::Receiver(Receiver {
                attrs,
                reference,
                mutability,
                ..
            }) => (
                "self".into(),
                attrs,
                quote_spanned!(parameter.__span() => <Self as #meta_path::Meta<#meta_path::Data>>::meta),
                borrow(
                    self,
                    reference.as_ref().map(|(_, lifetime)| lifetime.as_ref()),
                    mutability.is_some(),
                )
                .to_token_stream(),
            ),
            FnArg::Typed(PatType { attrs, pat, ty, .. }) => {
                let inner = Self::unwrap(ty);
                (
                    name(pat).unwrap_or_else(|| index.to_string()),
                    attrs,
                    quote_spanned!(ty.__span() => <#inner as #meta_path::Meta<#meta_path::Data>>::meta),
                    borrow_ty(self, ty).to_token_stream(),
                )
            }
        };
        let attributes = attributes.iter().map(|attribute| self.attribute(attribute));
        quote_spanned!(parameter.__span() => #meta_path::Parameter {
            borrow: #borrow,
            name: #name,
            meta: #meta,
            attributes: &[#(#attributes,)*],
        })
    }

    pub fn union(
        &self,
        ItemUnion {
            attrs,
            vis,
            ident,
            generics,
            fields,
            ..
        }: &ItemUnion,
    ) -> impl ToTokens {
        quote! {}
    }

    pub fn r#static(
        &self,
        ItemStatic {
            vis,
            ty,
            ident,
            attrs,
            mutability,
            ..
        }: &ItemStatic,
    ) -> impl ToTokens {
        let meta_path = &self.meta;
        let access = self.access(vis);
        let attributes = attrs.iter().map(|attribute| self.attribute(attribute));
        let get_mut = if mutability.is_some() {
            quote_spanned!(ident.span() => Some(|| &mut #ident))
        } else {
            quote_spanned!(ident.span() => None)
        };
        quote_spanned!(ident.span() => #meta_path::Static {
            access: #access,
            name: stringify!(#ident),
            meta: <#ty as #meta_path::Meta>::meta,
            get: || &#ident,
            get_mut: #get_mut,
            attributes: &[#(#attributes,)*],
        })
    }

    pub fn constant(
        &self,
        ItemConst {
            vis,
            ty,
            ident,
            attrs,
            ..
        }: &ItemConst,
    ) -> impl ToTokens {
        let meta_path = &self.meta;
        let access = self.access(vis);
        let attributes = attrs.iter().map(|attribute| self.attribute(attribute));
        quote_spanned!(ident.span() => #meta_path::Constant {
            access: #access,
            name: stringify!(#ident),
            meta: <#ty as #meta_path::Meta>::meta,
            value: &#ident,
            attributes: &[#(#attributes,)*],
        })
    }

    fn unwrap(ty: &Type) -> &Type {
        match ty {
            Type::Reference(TypeReference { elem, .. }) => Self::unwrap(elem),
            ty => ty,
        }
    }

    fn index(&self, names: impl IntoIterator<Item = Option<Ident>>) -> impl ToTokens {
        let matches = names.into_iter().enumerate().map(|(index, name)| {
            let index = LitInt::new(&format!("{}", index), Span::call_site());
            let key = name.as_ref().map_or_else(
                || quote! { stringify!(#index) },
                |name| quote! { stringify!(#index) | stringify!(#name) },
            );
            quote! { #key => Some(#index) }
        });
        quote! { |name| match name { #(#matches,)* _ => None } }
    }
}

fn path<'a>(segments: impl IntoIterator<Item = &'a str>) -> Path {
    Path {
        segments: segments
            .into_iter()
            .map(|segment| PathSegment::from(Ident::new(segment, Span::call_site())))
            .collect(),
        leading_colon: Some(Default::default()),
    }
}
