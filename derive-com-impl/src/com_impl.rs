use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    AttributeArgs, Block, Expr, FnArg, Generics, Ident, ImplItem, ImplItemMethod, Item, ItemImpl,
    Lit, Meta, MetaNameValue, NestedMeta, Pat, Path, ReturnType, Type,
};

pub fn expand_com_impl(args: &AttributeArgs, item: &Item) -> Result<TokenStream, String> {
    let item = match item {
        Item::Impl(item) => item,
        _ => return Err("#[com_impl] may only be used on an `impl` block".into()),
    };

    let info = ComImpl::parse(args, item)?;
    let result = info.quote();

    Ok(result)
}

struct ComImpl<'a> {
    has_parent: bool,
    self_ty: &'a Type,
    com_ty: &'a Path,
    com_vtbl: Path,
    com_ty_name: &'a Ident,
    functions: Vec<ComFunction<'a>>,
    generics: &'a Generics,
}

impl<'a> ComImpl<'a> {
    fn quote(&self) -> TokenStream {
        let vtbl_impl = self.quote_vtbl_impl();
        let fn_impls = self.quote_fn_impls();

        quote! {
            #vtbl_impl
            #fn_impls
        }
    }

    fn quote_vtbl_impl(&self) -> TokenStream {
        let self_ty = self.self_ty;
        let (impgen, _, wherec) = self.generics.split_for_impl();
        let com_vtbl = &self.com_vtbl;
        let parent_entry = self.quote_parent_entry();
        let com_entries = self
            .functions
            .iter()
            .map(|f| f.quote_vtbl_entry(self.com_ty_name));

        quote! {
            unsafe impl #impgen com_impl::BuildVTable<#com_vtbl> for #self_ty #wherec {
                const VTBL: #com_vtbl = #com_vtbl {
                    #parent_entry
                    #(#com_entries,)*
                };

                fn static_vtable() -> com_impl::VTable<#com_vtbl> {
                    com_impl::VTable::new(&Self::VTBL)
                }
            }
        }
    }

    fn quote_fn_impls(&self) -> TokenStream {
        let self_ty = self.self_ty;
        let (impgen, _, wherec) = self.generics.split_for_impl();
        let fn_stubs = self.functions.iter().map(|f| f.quote_stub(self));
        let fn_bodies = self.functions.iter().map(|f| f.quote_body(self));

        quote! {
            #[allow(non_snake_case)]
            impl #impgen #self_ty #wherec {
                #(#fn_stubs)*
                #(#fn_bodies)*
            }
        }
    }

    fn quote_parent_entry(&self) -> TokenStream {
        if self.has_parent {
            quote! { parent: <Self as com_impl::BuildVTable<_>>::VTBL, }
        } else {
            quote!{}
        }
    }

    // ----------------------------------------------------------------

    fn parse(args: &'a AttributeArgs, item: &'a ItemImpl) -> Result<Self, String> {
        if item.unsafety.is_none() {
            return Err(
                "Implementing COM interfaces is inherently unsafe. Please use \
                 `unsafe impl` to signify your understanding of this fact."
                    .into(),
            );
        }

        let has_parent = Self::has_parent(args);
        let self_ty = &item.self_ty;
        let com_ty = Self::com_ty(item)?;
        let com_vtbl = Self::com_vtbl(com_ty);
        let com_ty_name = Self::com_ty_name(com_ty);
        let functions = ComFunction::parse_all(item)?;
        let generics = &item.generics;

        Ok(ComImpl {
            has_parent,
            self_ty,
            com_ty,
            com_vtbl,
            com_ty_name,
            functions,
            generics,
        })
    }

    fn has_parent(args: &AttributeArgs) -> bool {
        for arg in args {
            match arg {
                NestedMeta::Meta(Meta::Word(word)) if word == "no_parent" => return false,
                _ => continue,
            }
        }
        true
    }

    fn com_ty(item: &ItemImpl) -> Result<&Path, String> {
        match &item.trait_ {
            Some((None, path, _)) => Ok(path),

            Some((Some(_bang), _, _)) => Err("Cannot anti-impl a COM interface. (impl !T)".into()),
            None => Err("You must specify an interface to implement. \
                         (impl ISomething for MyTy)"
                .into()),
        }
    }

    fn com_vtbl(com_ty: &Path) -> Path {
        let mut path = com_ty.clone();

        match path.segments.last_mut() {
            Some(mut pair) => {
                let last = pair.value_mut();
                let new_id = Ident::new(&format!("{}Vtbl", last.ident), last.ident.span());
                last.ident = new_id;
            }
            None => unreachable!(),
        }

        path
    }

    fn com_ty_name(ty: &Path) -> &Ident {
        assert!(ty.segments.len() > 0);
        &ty.segments.last().unwrap().value().ident
    }
}

struct ComFunction<'a> {
    is_mut: bool,
    is_unsafe: bool,
    com_name: Ident,
    panic_behavior: OnPanic,
    abi: String,
    args: Vec<Arg<'a>>,
    ret: &'a ReturnType,
    body: &'a Block,
}

enum OnPanic {
    Nothing,
    Abort,
    Hresult(Box<TokenStream>),
}

impl<'a> ComFunction<'a> {
    fn quote_stub(&self, context: &ComImpl) -> TokenStream {
        let (refderef, ptrkind) = if self.is_mut {
            (quote! { &mut * }, quote! { mut })
        } else {
            (quote! { &* }, quote! { const })
        };

        let abi = &self.abi;
        let name = self.stub_name(context.com_ty_name);
        let body_name = self.body_name(context.com_ty_name);
        let args = self.quote_stub_args(context);
        let pass = self.quote_pass_args();
        let ret = self.ret;
        let call_body = self.quote_stub_call(
            context,
            quote! {
                let this = #refderef(this as *#ptrkind Self);
                Self::#body_name(this, #pass)
            },
        );

        quote! {
            #[inline(never)]
            unsafe extern #abi fn #name(#args) #ret {
                #call_body
            }
        }
    }

    fn quote_body(&self, context: &ComImpl) -> TokenStream {
        let unsafemod = if self.is_unsafe {
            quote! { unsafe }
        } else {
            quote!{}
        };

        let abi = &self.abi;
        let name = self.body_name(context.com_ty_name);
        let args = self.quote_body_args();
        let ret = self.ret;
        let body = &self.body;

        quote! {
            #[inline(always)]
            #unsafemod extern #abi fn #name(#args) #ret
            #body
        }
    }

    fn quote_body_args(&self) -> TokenStream {
        let selfarg = if self.is_mut {
            quote! { &mut self }
        } else {
            quote! { &self }
        };

        let args = self.args.iter().map(|a| a.quote_body_arg());
        quote! {
            #selfarg,
            #(#args),*
        }
    }

    fn quote_stub_args(&self, context: &ComImpl) -> TokenStream {
        let com_ty = context.com_ty;
        let args = self.args.iter().map(|a| a.quote_stub_arg());
        quote! {
            this: *mut #com_ty,
            #(#args),*
        }
    }

    fn quote_pass_args(&self) -> TokenStream {
        let args = self.args.iter().map(|a| &a.id);
        quote! {
            #(#args),*
        }
    }

    fn quote_stub_call(&self, context: &ComImpl, inner: TokenStream) -> TokenStream {
        match &self.panic_behavior {
            OnPanic::Nothing => inner,
            OnPanic::Abort => {
                let message = self.abort_message(context);
                quote! {
                    let result = std::panic::catch_unwind(move || {
                        #inner
                    });
                    match result {
                        Ok(result) => result,
                        Err(_) => {
                            let stderr = std::io::stderr();
                            let _ = std::io::Write::write_all(&mut stderr.lock(), #message);
                            std::process::abort();
                        }
                    }
                }
            }
            OnPanic::Hresult(expr) => quote! {
                let __com_impl_result = std::panic::catch_unwind(move || {
                    #inner
                });
                match __com_impl_result {
                    Ok(result) => result,
                    Err(_) => { #expr }
                }
            },
        }
    }

    // ----------------------------------------------------------------

    fn stub_name(&self, com_ty_name: &Ident) -> Ident {
        let name = format!("__com_impl_stub__{}__{}", com_ty_name, self.com_name);
        Ident::new(&name, com_ty_name.span())
    }

    fn body_name(&self, com_ty_name: &Ident) -> Ident {
        let name = format!("__com_impl_body__{}__{}", com_ty_name, self.com_name);
        Ident::new(&name, com_ty_name.span())
    }

    fn quote_vtbl_entry(&self, com_ty_name: &Ident) -> TokenStream {
        let com_name = &self.com_name;
        let stub_name = self.stub_name(com_ty_name);

        quote! {
            #com_name: Self::#stub_name
        }
    }

    fn abort_message(&self, context: &ComImpl) -> syn::LitByteStr {
        syn::LitByteStr::new(
            &format!(
                "User-implemented COM method for {}::{} panicked. Aborting!",
                context.com_ty_name, self.com_name,
            )
            .as_bytes(),
            Span::call_site(),
        )
    }

    // ----------------------------------------------------------------

    fn parse_all(item: &'a ItemImpl) -> Result<Vec<Self>, String> {
        let mut fns = Vec::new();

        for item in &item.items {
            let item = match item {
                ImplItem::Method(method) => method,
                _ => return Err("Only methods may be in a com_impl body".into()),
            };

            fns.push(Self::parse(item)?);
        }

        Ok(fns)
    }

    fn parse(item: &'a ImplItemMethod) -> Result<Self, String> {
        Self::validate_sig(item)?;

        let is_mut = Self::determine_mut(item)?;
        let is_unsafe = Self::determine_unsafe(item);
        let com_name = Self::determine_name(item)?;
        let panic_behavior = Self::determine_panic_behavior(item)?;
        let abi = Self::determine_abi(item);
        let args = Self::parse_args(item)?;
        let ret = &item.sig.decl.output;
        let body = &item.block;

        Ok(ComFunction {
            is_mut,
            is_unsafe,
            com_name,
            panic_behavior,
            abi,
            args,
            ret,
            body,
        })
    }

    fn determine_mut(item: &ImplItemMethod) -> Result<bool, String> {
        let first_arg = item.sig.decl.inputs.first().map(|p| *p.value());
        let arg = match first_arg {
            Some(FnArg::SelfRef(arg)) => arg,
            _ => {
                return Err(format!(
                    "A COM method must take `self` by ref. (fn {})",
                    item.sig.ident.to_string()
                ))
            }
        };

        Ok(arg.mutability.is_some())
    }

    fn determine_unsafe(item: &ImplItemMethod) -> bool {
        item.sig.unsafety.is_some()
    }

    fn determine_name(item: &ImplItemMethod) -> Result<Ident, String> {
        // First check for a #[com_name = "..."] attribute
        for attr in &item.attrs {
            if attr.path.segments.len() == 1 && attr.path.segments[0].ident == "com_name" {
                let meta = attr.parse_meta().map_err(|e| e.to_string())?;
                match &meta {
                    Meta::NameValue(MetaNameValue {
                        lit: Lit::Str(name),
                        ..
                    }) => return Ok(Ident::new(&name.value(), name.span())),
                    _ => return Err("Invalid syntax for #[com_name] attribute".into()),
                }
            } else if attr.path.segments.len() != 1 || attr.path.segments[0].ident != "panic" {
                return Err(format!(
                    "Invalid attribute `#[{}]` on COM method",
                    attr.path.clone().into_token_stream()
                ));
            }
        }

        // Now try to convert the name from the method name
        let orig_name = item.sig.ident.to_string();
        let mut is_start = true;
        let mut name = String::with_capacity(orig_name.len());
        for c in orig_name.chars() {
            match c {
                '0'...'9' => name.push(c),
                'A'...'Z' => name.push(c),
                'a'...'z' if !is_start => name.push(c),
                'a'...'z' if is_start => {
                    name.push(c.to_ascii_uppercase());
                    is_start = false;
                }
                '_' => is_start = true,
                _ => {
                    return Err(
                        "Identifier ({}) that wouldn't be used in a COM function name found. \
                         Please use #[com_name] to specify the function it maps to explicitly."
                            .into(),
                    )
                }
            }
        }

        Ok(Ident::new(&name, item.sig.ident.span()))
    }

    fn determine_panic_behavior(item: &ImplItemMethod) -> Result<OnPanic, String> {
        for attr in &item.attrs {
            if attr.path.segments.len() != 1 || attr.path.segments[0].ident != "panic" {
                continue;
            }

            let meta = attr.parse_meta().map_err(|e| e.to_string())?;
            let attr = match &meta {
                Meta::List(list) if list.nested.len() == 1 => &list.nested[0],
                _ => {
                    return Err("Incorrect syntax for #[panic]. \
                                See documentation for #[com_impl]"
                        .into())
                }
            };

            match attr {
                NestedMeta::Meta(Meta::Word(id)) if id == "abort" => {
                    return Ok(OnPanic::Abort);
                }
                NestedMeta::Meta(Meta::NameValue(MetaNameValue {
                    ident,
                    lit: Lit::Str(lit),
                    ..
                })) if ident == "result" => {
                    let expr: Expr = match syn::parse_str(&lit.value()) {
                        Ok(expr) => expr,
                        Err(e) => return Err(format!("Error parsing #[panic] attribute: {}", e)),
                    };

                    let expr = quote_spanned!{lit.span()=> { #expr }};
                    return Ok(OnPanic::Hresult(Box::new(expr)));
                }
                _ => {
                    return Err("Incorrect syntax for #[panic]. \
                                See documentation for #[com_impl]."
                        .into())
                }
            }
        }

        Ok(OnPanic::Nothing)
    }

    fn determine_abi(item: &ImplItemMethod) -> String {
        let abi = match &item.sig.abi {
            Some(abi) => abi,
            None => return "system".into(),
        };

        match &abi.name {
            Some(lit) => lit.value(),
            None => "system".into(),
        }
    }

    fn parse_args(item: &ImplItemMethod) -> Result<Vec<Arg>, String> {
        item.sig
            .decl
            .inputs
            .iter()
            .skip(1)
            .enumerate()
            .map(|(i, arg)| Arg::parse(i, arg))
            .collect()
    }

    fn validate_sig(item: &ImplItemMethod) -> Result<(), String> {
        if item.sig.decl.variadic.is_some() {
            return Err("Variadic methods are not allowed in COM".into());
        }
        if item.sig.decl.generics.params.len() > 0 {
            return Err(
                "Generic types and lifetime parameters are not allowed on COM methods.".into(),
            );
        }
        if item.sig.decl.generics.where_clause.is_some() {
            return Err("Where clauses are not allowed on COM methods.".into());
        }
        if item.sig.constness.is_some() {
            return Err("COM methods may not be const fns".into());
        }
        if item.sig.asyncness.is_some() {
            return Err("COM methods may not be async fns".into());
        }

        Ok(())
    }
}

struct Arg<'a> {
    ty: &'a Type,
    pat: Option<&'a Pat>,
    id: Ident,
}

impl<'a> Arg<'a> {
    fn quote_body_arg(&self) -> TokenStream {
        let ty = self.ty;
        match self.pat {
            Some(pat) => quote! { #pat : #ty },
            None => quote! { _ : #ty },
        }
    }

    fn quote_stub_arg(&self) -> TokenStream {
        let ty = self.ty;
        let id = &self.id;
        quote! { #id : #ty }
    }

    // ----------------------------------------------------------------

    fn parse(i: usize, arg: &'a FnArg) -> Result<Self, String> {
        match arg {
            FnArg::Captured(cap) => Ok(Arg {
                ty: &cap.ty,
                pat: Some(&cap.pat),
                id: Ident::new(&format!("__com_arg_{}", i), Span::call_site()),
            }),
            FnArg::Ignored(ty) => Ok(Arg {
                ty: ty,
                pat: None,
                id: Ident::new(&format!("__com_arg_{}", i), Span::call_site()),
            }),
            _ => return Err("Invalid argument syntax for COM function.".into()),
        }
    }
}
