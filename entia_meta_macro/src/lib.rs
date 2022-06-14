use std::{fs::read_to_string, mem};

use proc_macro::TokenStream;
use quote::{__private::Span, quote, quote_spanned, spanned::Spanned, ToTokens};
use syn::{
    parse_file, parse_macro_input, parse_quote, Attribute, ConstParam, Data, DataEnum, DataStruct,
    DataUnion, DeriveInput, Expr, ExprLit, ExprPath, ExprTuple, Field, Fields, File, FnArg,
    GenericParam, Generics, Ident, Item, ItemConst, ItemEnum, ItemFn, ItemMod, ItemStatic,
    ItemStruct, ItemUnion, Lifetime, LifetimeDef, Lit, LitInt, Pat, PatBox, PatIdent, PatReference,
    PatType, Path, PathSegment, Receiver, ReturnType, Signature, Type, TypeParam, TypeReference,
    UseGlob, UseGroup, UseName, UsePath, UseRename, UseTree, VisPublic, Visibility,
};

#[derive(Clone)]
struct Context {
    crate_path: Path,
    super_path: Path,
    self_path: Path,
    meta_path: Path,
    external: bool,
    implement: bool,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            crate_path: path(["crate"]),
            super_path: path(["super"]),
            self_path: path(["self"]),
            meta_path: path(["entia", "meta"]),
            external: false,
            implement: false,
        }
    }
}

#[proc_macro]
pub fn meta_extern(input: TokenStream) -> TokenStream {
    let tuple = parse_macro_input!(input as ExprTuple);
    let (meta_path, crate_path, self_path, file_path) = match (
        &tuple.elems[0],
        &tuple.elems[1],
        &tuple.elems[2],
        &tuple.elems[3],
    ) {
        (
            Expr::Path(ExprPath {
                path: meta_path, ..
            }),
            Expr::Path(ExprPath {
                path: crate_path, ..
            }),
            Expr::Path(ExprPath {
                path: self_path, ..
            }),
            Expr::Lit(ExprLit {
                lit: Lit::Str(file_path),
                ..
            }),
        ) => (meta_path, crate_path, self_path, file_path),
        _ => {
            return quote! { compile_error!("Expected crate path and corresponding file path.") }
                .into()
        }
    };
    let (path, span) = (file_path.value(), file_path.span());
    let path = std::path::Path::new(&path);
    let name = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    let file = read_to_string(path).unwrap();
    let File { attrs, items, .. } = parse_file(&file).unwrap();
    let mut module = ItemMod {
        attrs,
        ident: Ident::new(name, span),
        vis: Visibility::Public(VisPublic {
            pub_token: Default::default(),
        }),
        content: Some((Default::default(), items)),
        mod_token: Default::default(),
        semi: Default::default(),
    };
    let mut super_path = self_path.clone();
    super_path.segments.pop();
    let context = Context {
        meta_path: meta_path.clone(),
        crate_path: crate_path.clone(),
        super_path,
        self_path: self_path.clone(),
        external: true,
        implement: false,
    };
    let module = context.module(&mut module);
    quote_spanned!(span => #module).into()
}

#[proc_macro_derive(Meta)]
pub fn derive(input: TokenStream) -> TokenStream {
    let DeriveInput {
        attrs,
        vis,
        ident,
        generics,
        data,
    } = parse_macro_input!(input as DeriveInput);
    let context = Context::default();
    match data {
        Data::Struct(DataStruct {
            fields,
            semi_token,
            struct_token,
        }) => context
            .implement_structure(&ItemStruct {
                attrs,
                fields,
                generics,
                ident,
                vis,
                semi_token,
                struct_token,
            })
            .to_token_stream(),
        Data::Enum(DataEnum {
            enum_token,
            brace_token,
            variants,
        }) => context
            .implement_enumeration(&ItemEnum {
                attrs,
                variants,
                generics,
                ident,
                vis,
                brace_token,
                enum_token,
            })
            .to_token_stream(),
        Data::Union(DataUnion {
            fields,
            union_token,
        }) => context
            .implement_union(&ItemUnion {
                attrs,
                fields,
                generics,
                ident,
                vis,
                union_token,
            })
            .to_token_stream(),
    }
    .into()
}

#[proc_macro_attribute]
pub fn meta(attribute: TokenStream, item: TokenStream) -> TokenStream {
    let context = if attribute.is_empty() {
        Context::default()
    } else {
        let mut context = Context::default();
        context.meta_path = parse_macro_input!(attribute as Path);
        context
    };

    match parse_macro_input!(item as Item) {
        Item::Mod(mut module) => {
            let meta = context.module(&mut module);
            if let Some((_, content)) = &mut module.content {
                let meta_path = &context.meta_path;
                content.push(parse_quote! { pub static META: #meta_path::Module = #meta; });
            }
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
    pub fn implement(&self) -> Self {
        let mut context = self.clone();
        context.implement = true;
        context
    }

    pub fn implement_structure(&self, item: &ItemStruct) -> impl ToTokens {
        let context = self.implement();
        let meta = context.structure(item);
        context
            .implement_body(meta, &item.ident, &item.generics, "Structure")
            .to_token_stream()
    }

    pub fn implement_enumeration(&self, item: &ItemEnum) -> impl ToTokens {
        let context = self.implement();
        let meta = context.enumeration(item);
        context
            .implement_body(meta, &item.ident, &item.generics, "Enumeration")
            .to_token_stream()
    }

    pub fn implement_union(&self, item: &ItemUnion) -> impl ToTokens {
        let context = self.implement();
        let meta = context.union(item);
        context
            .implement_body(meta, &item.ident, &item.generics, "Union")
            .to_token_stream()
    }

    pub fn item(&self, item: &mut Item) -> Option<impl ToTokens> {
        match item {
            Item::Const(item) => Some(self.constant(item).to_token_stream()),
            Item::Enum(item) => Some(self.enumeration(item).to_token_stream()),
            Item::Fn(item) if item.sig.generics.params.is_empty() => {
                Some(self.function(item).to_token_stream())
            }
            Item::Mod(item) if item.content.is_some() => Some(self.module(item).to_token_stream()),
            Item::Static(item) => Some(self.r#static(item).to_token_stream()),
            Item::Struct(item) => Some(self.structure(item).to_token_stream()),
            _ => None,
        }
    }

    pub fn access(&self, visibility: &Visibility) -> impl ToTokens {
        let meta_path = &self.meta_path;
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

    pub fn attribute(&self, Attribute { path, tokens, .. }: &Attribute) -> Option<impl ToTokens> {
        if let Some(ident) = path.get_ident() {
            match ident.to_string().as_str() {
                "doc" => return None,
                _ => {}
            }
        }
        let meta_path = &self.meta_path;
        Some(
            quote_spanned!(path.__span() => #meta_path::Attribute { path: stringify!(#path), content: stringify!(#tokens) }),
        )
    }

    pub fn push(&self, identifier: Ident) -> Self {
        let mut context = self.clone();
        if context.external {
            let mut self_path = context.self_path.clone();
            self_path.segments.push(PathSegment {
                ident: identifier,
                arguments: Default::default(),
            });
            context.super_path = mem::replace(&mut context.self_path, self_path);
        }
        context
    }

    pub fn pop(&self) -> Self {
        let mut context = self.clone();
        if context.external {
            let mut super_path = context.super_path.clone();
            super_path.segments.pop();
            context.self_path = mem::replace(&mut context.super_path, super_path);
        }
        context
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
        let meta_path = &self.meta_path;
        let access = self.access(vis);
        let attributes = attrs
            .iter()
            .filter_map(|attribute| self.attribute(attribute));

        if self.external && !matches!(vis, Visibility::Public(_)) {
            let index = self.index([]);
            quote_spanned!(ident.span() => {
                #meta_path::Module {
                    access: #access,
                    name: stringify!(#ident),
                    attributes: &[#(#attributes,)*],
                    members: #meta_path::Index(&[], #index),
                }
            })
        } else {
            let uses = content
                .iter_mut()
                .flat_map(|(_, items)| items)
                .chain(&mut [parse_quote! { use self::*; }])
                .filter_map(|item| match item {
                    Item::Use(item) => {
                        self.resolve_use(&mut item.tree);
                        Some(item.to_token_stream())
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
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
                    Item::Fn(item) => {
                        let meta = self.function(item);
                        Some((&item.sig.ident, quote_spanned!(item.sig.ident.span() => #meta_path::module::Member::Function(#meta))))
                    }
                    Item::Enum(item) => {
                        let implementation = self.implement_enumeration(item);
                        let name = &item.ident;
                        Some((name, quote_spanned!(item.ident.span() => {
                            #implementation
                            #meta_path::module::Member::Enumeration(<#name as #meta_path::Meta<&'static #meta_path::Enumeration>>::meta)
                        })))
                    }
                    Item::Struct(item) => {
                        let implementation = self.implement_structure(item);
                        let name = &item.ident;
                        Some((name, quote_spanned!(item.ident.span() => {
                            #implementation
                            #meta_path::module::Member::Structure(<#name as #meta_path::Meta<&'static #meta_path::Structure>>::meta)
                        })))
                    }
                    Item::Mod(item) => {
                        let context = self.push(item.ident.clone());
                        let meta = context.module(item);
                        Some(if context.external {
                            (&item.ident, quote_spanned!(item.ident.span() => #meta_path::module::Member::Module(&#meta)))
                        } else {
                            if let Some((_, content)) = &mut item.content {
                                let meta_path = &context.meta_path;
                                content.push(parse_quote! { pub static META: #meta_path::Module = #meta; });
                            }
                            let name = &item.ident;
                        (name, quote_spanned!(name.span() => #meta_path::module::Member::Module(&#name::META)))
                        })
                    }
                    // Item::Impl(_) => todo!(),
                    // Item::Trait(_) => todo!(),
                    // Item::Union(_) => todo!(),
                    _ => None,
                }).unzip::<_, _, Vec<_>, Vec<_>>();

            let index = self.index(names.into_iter().cloned().map(Some));
            quote_spanned! (ident.span() => {
                #(#uses)*
                #meta_path::Module {
                    access: #access,
                    name: stringify!(#ident),
                    attributes: &[#(#attributes,)*],
                    members: #meta_path::Index(&[#(#members,)*], #index),
                }
            })
        }
    }

    pub fn generic(&self, generic: &GenericParam) -> impl ToTokens {
        let meta_path = &self.meta_path;
        match generic {
            GenericParam::Type(TypeParam {
                attrs,
                ident,
                default,
                ..
            }) => {
                let attributes = attrs
                    .iter()
                    .filter_map(|attribute| self.attribute(attribute));
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
                let attributes = attrs
                    .iter()
                    .filter_map(|attribute| self.attribute(attribute));
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
                let attributes = attrs
                    .iter()
                    .filter_map(|attribute| self.attribute(attribute));
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
        parent: &Type,
        index: usize,
        deconstruct: &impl ToTokens,
    ) -> impl ToTokens {
        let meta_path = &self.meta_path;
        let access = self.access(vis);
        let attributes = attrs
            .iter()
            .filter_map(|attribute| self.attribute(attribute));
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
        parent: &Type,
        generics: &Generics,
        path: &Path,
    ) -> (
        impl ToTokens,
        impl ToTokens,
        impl ToTokens,
        impl ToTokens,
        impl ToTokens,
    ) {
        let span = parent.__span();
        let meta_path = &self.meta_path;
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
                let construct = quote_spanned!(span => #path { #(#pairs,)* });
                let deconstruct = quote_spanned!(span => #path { #(#keys,)* });
                let (names, fields) = fields
                    .named
                    .iter()
                    .enumerate()
                    .map(|(index, field)| {
                        (&field.ident, self.field(field, parent, index, &deconstruct))
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
                let construct = quote_spanned!(span => #path (#(#pairs,)*));
                let deconstruct = quote_spanned!(span => #path (#(#keys,)*));
                let (names, fields) = fields
                    .unnamed
                    .iter()
                    .enumerate()
                    .map(|(index, field)| {
                        (&field.ident, self.field(field, parent, index, &deconstruct))
                    })
                    .unzip::<_, _, Vec<_>, Vec<_>>();
                (construct, deconstruct, names, values, fields)
            }
            Fields::Unit => (
                quote_spanned!(span => #path),
                quote_spanned!(span => #path),
                vec![],
                vec![],
                vec![],
            ),
        };

        let (new, values) = if self.implement || generics.params.is_empty() {
            (
                quote_spanned!(span => Some(|values| Some(Box::new(#construct)))),
                quote_spanned!(span => Some(|instance| match *instance.downcast::<#parent>()? {
                    #deconstruct => Ok(Box::new([#(#values,)*])),
                    #[allow(unreachable_patterns)]
                    instance => Err(Box::new(instance)),
                })),
            )
        } else {
            (quote_spanned!(span => None), quote_spanned!(span => None))
        };
        let index = self.index(names.into_iter().cloned());
        let fields = quote_spanned!(span => #meta_path::Index(&[#(#fields,)*], #index));
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
        }: &ItemStruct,
    ) -> impl ToTokens {
        let meta_path = &self.meta_path;
        let access = self.access(vis);
        let attributes = attrs
            .iter()
            .filter_map(|attribute| self.attribute(attribute));
        let (_, type_generics, _) = generics.split_for_impl();
        let parent: Type = parse_quote!(#ident #type_generics);
        let (size, identifier) = if self.implement || generics.params.is_empty() {
            (
                quote_spanned!(ident.span() => Some(::std::mem::size_of::<#parent>())),
                quote_spanned!(ident.span() => Some(::std::any::TypeId::of::<#parent>)),
            )
        } else {
            (
                quote_spanned!(ident.span() => None),
                quote_spanned!(ident.span() => None),
            )
        };
        let (_, _, new, values, fields) =
            self.fields(fields, &parent, &generics, &parse_quote!(#ident));
        let generics = generics.params.iter().map(|generic| self.generic(generic));
        quote_spanned!(ident.span() => #meta_path::Structure {
            access: #access,
            name: stringify!(#ident),
            size: #size,
            identifier: #identifier,
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
        let meta_path = &self.meta_path;
        let access = self.access(vis);
        let attributes = attrs
            .iter()
            .filter_map(|attribute| self.attribute(attribute));
        let (_, type_generics, _) = generics.split_for_impl();
        let parent: Type = parse_quote!(#ident #type_generics);
        let (names, variants) = variants
            .iter()
            .map(|variant| {
                let attributes = variant
                    .attrs
                    .iter()
                    .filter_map(|attribute| self.attribute(attribute));
                let name = &variant.ident;
                let (_, deconstruct, new, values, fields) = self.fields(
                    &variant.fields,
                    &parent,
                    &generics,
                    &parse_quote!(#ident::#name),
                );
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
        let generics = generics.params.iter().map(|generic| self.generic(generic));
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

        let meta_path = &self.meta_path;
        let access = self.access(vis);
        let attributes = attrs
            .iter()
            .filter_map(|attribute| self.attribute(attribute));
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
        let meta_path = &self.meta_path;
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
            let meta_path = &context.meta_path;
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

        let meta_path = &self.meta_path;
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
        let attributes = attributes
            .iter()
            .filter_map(|attribute| self.attribute(attribute));
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
        let meta_path = &self.meta_path;
        let access = self.access(vis);
        let attributes = attrs
            .iter()
            .filter_map(|attribute| self.attribute(attribute));
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
        let meta_path = &self.meta_path;
        let access = self.access(vis);
        let attributes = attrs
            .iter()
            .filter_map(|attribute| self.attribute(attribute));
        quote_spanned!(ident.span() => #meta_path::Constant {
            access: #access,
            name: stringify!(#ident),
            meta: <#ty as #meta_path::Meta>::meta,
            value: &#ident,
            attributes: &[#(#attributes,)*],
        })
    }

    fn resolve_use(&self, tree: &mut UseTree) {
        match tree {
            UseTree::Path(UsePath { ident, .. })
            | UseTree::Name(UseName { ident, .. })
            | UseTree::Rename(UseRename { ident, .. }) => {
                if let Some(path) = match ident.to_string().as_str() {
                    "crate" => Some(&self.crate_path),
                    "super" => Some(&self.super_path),
                    "self" => Some(&self.self_path),
                    _ => None,
                } {
                    let mut segments = path.segments.iter();
                    *ident = segments.next_back().unwrap().ident.clone();

                    let mut tree = tree;
                    for PathSegment { ident, .. } in segments {
                        *tree = UseTree::Path(UsePath {
                            ident: ident.clone(),
                            tree: Box::new(mem::replace(
                                tree,
                                UseTree::Glob(UseGlob {
                                    star_token: Default::default(),
                                }),
                            )),
                            colon2_token: Default::default(),
                        });
                        match tree {
                            UseTree::Path(path) => tree = &mut path.tree,
                            _ => unreachable!(),
                        }
                    }
                }
            }
            UseTree::Group(UseGroup { items, .. }) => {
                for item in items {
                    self.resolve_use(item);
                }
            }
            _ => {}
        }
    }

    fn implement_body(
        &self,
        meta: impl ToTokens,
        ident: &Ident,
        generics: &Generics,
        suffix: &str,
    ) -> impl ToTokens {
        let meta_path = &self.meta_path;
        let suffix = Ident::new(suffix, Span::call_site());
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
