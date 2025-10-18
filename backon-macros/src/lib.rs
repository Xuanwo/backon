#![forbid(unsafe_code)]

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Error, FnArg, Ident, ImplItemFn, ItemFn, LitBool, Pat, Path, Signature, Token};

/// Attribute for turning a function into a retried one using backon retry APIs.
#[proc_macro_attribute]
pub fn backon(args: TokenStream, input: TokenStream) -> TokenStream {
    match expand_backon(args, input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error().into(),
    }
}

fn expand_backon(args: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    let args = syn::parse2::<BackonArgs>(proc_macro2::TokenStream::from(args))?;
    let input_tokens = proc_macro2::TokenStream::from(input);

    if let Ok(mut item_fn) = syn::parse2::<ItemFn>(input_tokens.clone()) {
        if item_fn.sig.receiver().is_some() {
            let method = syn::parse2::<ImplItemFn>(input_tokens)?;
            return expand_method(&args, method);
        }
        let original_block = (*item_fn.block).clone();
        let body_tokens = quote!(#original_block);
        let body_span = original_block.span();
        let block = build_function_body(
            &args,
            &item_fn.sig,
            body_tokens,
            body_span,
            None,
            false,
            false,
        )?;
        item_fn.block = Box::new(block);
        return Ok(TokenStream::from(quote!(#item_fn)));
    }

    if let Ok(method) = syn::parse2::<ImplItemFn>(input_tokens) {
        return expand_method(&args, method);
    }

    Err(Error::new(
        proc_macro2::Span::call_site(),
        "#[backon] may only be applied to free functions or inherent methods",
    ))
}

fn expand_method(args: &BackonArgs, method: ImplItemFn) -> syn::Result<TokenStream> {
    let has_receiver = matches!(method.sig.inputs.first(), Some(FnArg::Receiver(_)));

    if !has_receiver {
        let mut wrapper = method;
        wrapper.attrs.retain(|attr| !attr.path().is_ident("backon"));
        let original_block = wrapper.block.clone();
        let body_tokens = quote!(#original_block);
        let body_span = original_block.span();
        let block = build_function_body(
            args,
            &wrapper.sig,
            body_tokens,
            body_span,
            None,
            false,
            false,
        )?;
        wrapper.block = block;
        return Ok(TokenStream::from(quote!(#wrapper)));
    }

    let mut helper = method.clone();
    helper.attrs.retain(|attr| !attr.path().is_ident("backon"));
    let helper_ident = format_ident!("__backon_{}_inner", helper.sig.ident);
    helper.sig.ident = helper_ident.clone();

    let mut wrapper = method;
    wrapper.attrs.retain(|attr| !attr.path().is_ident("backon"));

    if let Some(FnArg::Receiver(receiver)) = wrapper.sig.inputs.first() {
        if receiver.mutability.is_some() {
            return Err(Error::new(
                receiver.span(),
                "`#[backon]` does not yet support methods taking `&mut self`; please fall back to manual `RetryableWithContext` usage",
            ));
        }
    }

    let context = prepare_context(&wrapper.sig, true)?;
    let receiver_ident = context
        .receiver_ident
        .clone()
        .ok_or_else(|| Error::new(wrapper.sig.span(), "failed to capture receiver"))?;
    let arg_idents = collect_arg_idents(&wrapper.sig)?;

    let helper_args = if arg_idents.is_empty() {
        quote!(#receiver_ident)
    } else {
        quote!(#receiver_ident, #(#arg_idents),*)
    };

    let helper_call = if wrapper.sig.asyncness.is_some() {
        quote!(Self::#helper_ident(#helper_args).await)
    } else {
        quote!(Self::#helper_ident(#helper_args))
    };

    let body_tokens = quote!({ #helper_call });
    let body_span = helper.block.span();

    let block = build_function_body(
        args,
        &wrapper.sig,
        body_tokens,
        body_span,
        Some(context.clone()),
        true,
        true,
    )?;
    wrapper.block = block;

    Ok(TokenStream::from(quote!(#helper #wrapper)))
}

#[derive(Clone, Default)]
struct BackonArgs {
    backoff: Option<Path>,
    sleep: Option<Path>,
    when: Option<Path>,
    notify: Option<Path>,
    adjust: Option<Path>,
    context: bool,
}

impl Parse for BackonArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(Self::default());
        }

        let mut args = BackonArgs::default();

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            let key = ident.to_string();
            input.parse::<Token![=]>()?;

            match key.as_str() {
                "backoff" => {
                    ensure_path_unset(args.backoff.is_some(), ident.span())?;
                    args.backoff = Some(input.parse()?);
                }
                "sleep" => {
                    ensure_path_unset(args.sleep.is_some(), ident.span())?;
                    args.sleep = Some(input.parse()?);
                }
                "when" => {
                    ensure_path_unset(args.when.is_some(), ident.span())?;
                    args.when = Some(input.parse()?);
                }
                "notify" => {
                    ensure_path_unset(args.notify.is_some(), ident.span())?;
                    args.notify = Some(input.parse()?);
                }
                "adjust" => {
                    ensure_path_unset(args.adjust.is_some(), ident.span())?;
                    args.adjust = Some(input.parse()?);
                }
                "context" => {
                    if args.context {
                        return Err(Error::new(
                            ident.span(),
                            "`context` cannot be specified more than once",
                        ));
                    }
                    let value: LitBool = input.parse()?;
                    args.context = value.value;
                }
                other => {
                    return Err(Error::new(
                        ident.span(),
                        format!("unknown parameter `{other}`"),
                    ));
                }
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(args)
    }
}

fn ensure_path_unset(already: bool, span: proc_macro2::Span) -> syn::Result<()> {
    if already {
        Err(Error::new(span, "parameter already specified"))
    } else {
        Ok(())
    }
}

fn collect_arg_idents(sig: &Signature) -> syn::Result<Vec<Ident>> {
    let mut out = Vec::new();
    for input in sig.inputs.iter() {
        if let FnArg::Typed(pat_type) = input {
            match &*pat_type.pat {
                Pat::Ident(pat_ident) => out.push(pat_ident.ident.clone()),
                _ => {
                    return Err(Error::new(
                        pat_type.span(),
                        "parameters must bind to identifiers",
                    ));
                }
            }
        }
    }
    Ok(out)
}

fn build_function_body(
    args: &BackonArgs,
    sig: &Signature,
    body: proc_macro2::TokenStream,
    body_span: proc_macro2::Span,
    precomputed_context: Option<ContextInfo>,
    force_context: bool,
    include_receiver: bool,
) -> syn::Result<syn::Block> {
    let backoff_path = args
        .backoff
        .clone()
        .unwrap_or_else(|| syn::parse_str("::backon::ExponentialBuilder::default").unwrap());

    let sleep_path = args.sleep.clone();
    let when_path = args.when.clone();
    let notify_path = args.notify.clone();
    let adjust_path = args.adjust.clone();

    let is_async = sig.asyncness.is_some();

    if adjust_path.is_some() && !is_async {
        return Err(Error::new(
            sig.span(),
            "`adjust` is only available for async functions",
        ));
    }

    let context_data = if let Some(context) = precomputed_context {
        Some(context)
    } else if force_context || args.context {
        Some(prepare_context(sig, include_receiver)?)
    } else {
        None
    };

    let chain_tokens = if let Some(context) = context_data {
        build_with_context_chain(
            is_async,
            &backoff_path,
            sleep_path,
            when_path,
            notify_path,
            adjust_path,
            body,
            body_span,
            context,
        )
    } else {
        build_simple_chain(
            is_async,
            &backoff_path,
            sleep_path,
            when_path,
            notify_path,
            adjust_path,
            body,
        )
    }?;

    Ok(syn::parse2(chain_tokens)?)
}

#[derive(Clone)]
struct ContextInfo {
    pattern: proc_macro2::TokenStream,
    initial_expr: proc_macro2::TokenStream,
    return_expr: proc_macro2::TokenStream,
    ty: proc_macro2::TokenStream,
    receiver_ident: Option<Ident>,
}

fn prepare_context(sig: &Signature, include_receiver: bool) -> syn::Result<ContextInfo> {
    let mut patterns = Vec::new();
    let mut exprs = Vec::new();
    let mut return_exprs = Vec::new();
    let mut types = Vec::new();
    let mut receiver_ident = None;
    for input in sig.inputs.iter() {
        match input {
            FnArg::Receiver(receiver) => {
                if !include_receiver {
                    continue;
                }

                if receiver.reference.is_none() {
                    return Err(Error::new(
                        receiver.span(),
                        "`context = true` does not support methods that take ownership of `self`",
                    ));
                }

                if receiver.colon_token.is_some() {
                    return Err(Error::new(
                        receiver.span(),
                        "`#[backon]` currently supports only `&self` and `&mut self` receivers",
                    ));
                }

                let binding = format_ident!("__backon_self");
                let lifetime = receiver
                    .reference
                    .as_ref()
                    .and_then(|(_, lifetime)| lifetime.as_ref());
                let ty_tokens = if receiver.mutability.is_some() {
                    if let Some(lifetime) = lifetime {
                        quote!(& #lifetime mut Self)
                    } else {
                        quote!(&mut Self)
                    }
                } else if let Some(lifetime) = lifetime {
                    quote!(& #lifetime Self)
                } else {
                    quote!(&Self)
                };

                patterns.push(quote!(#binding));
                exprs.push(quote!(self));
                return_exprs.push(quote!(#binding));
                types.push(ty_tokens);
                receiver_ident = Some(binding);
            }
            FnArg::Typed(pat_type) => match &*pat_type.pat {
                Pat::Ident(pat_ident) => {
                    let ident = &pat_ident.ident;
                    patterns.push(quote!(#pat_ident));
                    exprs.push(quote!(#ident));
                    return_exprs.push(quote!(#ident));
                    let ty = &pat_type.ty;
                    types.push(quote!(#ty));
                }
                _ => {
                    return Err(Error::new(
                        pat_type.span(),
                        "`context = true` requires arguments to bind to identifiers",
                    ));
                }
            },
        }
    }

    let pattern = if patterns.is_empty() {
        quote!(())
    } else {
        quote!((#(#patterns),*))
    };

    let initial_expr = if exprs.is_empty() {
        quote!(())
    } else {
        quote!((#(#exprs),*))
    };

    let return_expr = if return_exprs.is_empty() {
        quote!(())
    } else {
        quote!((#(#return_exprs),*))
    };

    let ty = if types.is_empty() {
        quote!(())
    } else {
        quote!((#(#types),*))
    };

    Ok(ContextInfo {
        pattern,
        initial_expr,
        return_expr,
        ty,
        receiver_ident,
    })
}

fn build_simple_chain(
    is_async: bool,
    backoff_path: &Path,
    sleep: Option<Path>,
    when: Option<Path>,
    notify: Option<Path>,
    adjust: Option<Path>,
    body: proc_macro2::TokenStream,
) -> syn::Result<proc_macro2::TokenStream> {
    let mut chain = if is_async {
        quote! {
            (|| async move #body)
                .retry(__backon_builder)
        }
    } else {
        quote! {
            (|| #body)
                .retry(__backon_builder)
        }
    };

    if let Some(path) = sleep {
        chain = quote!(#chain.sleep(#path));
    }

    if let Some(path) = when {
        chain = quote!(#chain.when(#path));
    }

    if let Some(path) = notify {
        chain = quote!(#chain.notify(#path));
    }

    if let Some(path) = adjust {
        chain = quote!(#chain.adjust(#path));
    }

    let executed = if is_async {
        quote!(#chain.await)
    } else {
        quote!(#chain.call())
    };

    let trait_use = if is_async {
        quote!(
            use ::backon::Retryable as _;
        )
    } else {
        quote!(
            use ::backon::BlockingRetryable as _;
        )
    };

    Ok(quote!({
        #trait_use
        let __backon_builder = (#backoff_path)();
        #executed
    }))
}

fn build_with_context_chain(
    is_async: bool,
    backoff_path: &Path,
    sleep: Option<Path>,
    when: Option<Path>,
    notify: Option<Path>,
    adjust: Option<Path>,
    body: proc_macro2::TokenStream,
    _body_span: proc_macro2::Span,
    context: ContextInfo,
) -> syn::Result<proc_macro2::TokenStream> {
    let initial_context = &context.initial_expr;
    let return_context = &context.return_expr;
    let context_ty = &context.ty;
    let pattern = &context.pattern;

    let mut chain = if is_async {
        quote! {
            (|__backon_ctx: #context_ty| async move {
                let #pattern = __backon_ctx;
                let __backon_result = #body;
                (#return_context, __backon_result)
            })
            .retry(__backon_builder)
        }
    } else {
        quote! {
            (|__backon_ctx: #context_ty| {
                let #pattern = __backon_ctx;
                let __backon_result = #body;
                (#return_context, __backon_result)
            })
            .retry(__backon_builder)
        }
    };

    if let Some(path) = sleep {
        chain = quote!(#chain.sleep(#path));
    }

    if let Some(path) = when {
        chain = quote!(#chain.when(#path));
    }

    if let Some(path) = notify {
        chain = quote!(#chain.notify(#path));
    }

    if let Some(path) = adjust {
        chain = quote!(#chain.adjust(#path));
    }

    let trait_use = if is_async {
        quote!(
            use ::backon::RetryableWithContext as _;
        )
    } else {
        quote!(
            use ::backon::BlockingRetryableWithContext as _;
        )
    };

    let tail = if is_async {
        quote!({
            let (__backon_context, __backon_result) = #chain
                .context(__backon_initial_context)
                .await;
            let _ = __backon_context;
            __backon_result
        })
    } else {
        quote!({
            let (__backon_context, __backon_result) = #chain
                .context(__backon_initial_context)
                .call();
            let _ = __backon_context;
            __backon_result
        })
    };

    Ok(quote!({
        #trait_use
        let __backon_builder = (#backoff_path)();
        let __backon_initial_context: #context_ty = #initial_context;
        #tail
    }))
}
