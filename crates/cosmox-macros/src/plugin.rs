use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    Attribute, Ident, ItemMod, Visibility,
    parse::{Parse, ParseStream},
};

/// Parse helper for `#[on_event(Variant1, Variant2, ...)]` argument list.
struct EventVariants {
    variants: Vec<Ident>,
}

impl Parse for EventVariants {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut variants = Vec::new();
        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            variants.push(ident);
            if !input.is_empty() {
                let _: syn::Token![,] = input.parse()?;
            }
        }
        Ok(EventVariants { variants })
    }
}

#[derive(Debug, PartialEq, Clone)]
enum OnEventAttr {
    /// `#[on_event]` — no arguments, receive all events
    All,
    /// `#[on_event(Variant1, Variant2, ...)]` — register for specific event variants.
    /// Each entry carries the `EventVariantInfo` computed at parse time, so the
    /// generate phase can build the dispatch without re-querying the lookup table.
    Filtered(Vec<(Ident, EventVariantInfo)>),
}

/// Identifies the strongly-typed payload (`ctx`) type for an `Event` variant.
/// Each variant of this enum corresponds to a distinct `D` type in
/// `EventPayload<C, D>`. The macro uses this at parse time (never in generated
/// code) to ensure all variants in a single `#[on_event(...)]` share the same
/// handler signature — a single handler fn cannot receive different `ctx` types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum CtxTy {
    /// The payload type is `()` — no useful data, only the event kind matters.
    Unit,
    /// The payload type is `OnMetadataRawTreeReadyEventContext`.
    MetadataReady,
    /// The payload type is `OnMetadataLocalTreeReadyEventContext`.
    MetadataLocalReady,
    /// The payload type is `OnServerErrorEventContext`.
    ServerError,
}

impl std::fmt::Display for CtxTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CtxTy::Unit => f.write_str("()"),
            CtxTy::MetadataReady => f.write_str("OnMetadataRawTreeReadyEventContext"),
            CtxTy::MetadataLocalReady => f.write_str("OnMetadataLocalTreeReadyEventContext"),
            CtxTy::ServerError => f.write_str("OnServerErrorEventContext"),
        }
    }
}

/// Describes how a filtered `#[on_event(Variant)]` handler is invoked.
///
/// `context` is `Some((variant_pat, handle_idents))` when the host pairs this
/// event with a `EventContext` variant carrying resource handles. The generated
/// code only calls the handler after matching that `EventContext` variant and
/// unpacking its handles. `None` means the event carries no handles, so the
/// handler is called directly (the `event_context` parameter is ignored). The
/// strongly-typed business payload (the `D` of `EventPayload<C, D>`) is bound
/// as `ctx` and forwarded to the handler.
///
/// `ctx_ty` identifies the payload type. It is used only by the macro-internal
/// compatibility check (a single `#[on_event(...)]` shares one handler fn, so
/// every listed variant must share the same handler signature); it is never
/// emitted into the generated code.
#[derive(Debug, Clone)]
struct EventVariantInfo {
    context: Option<(TokenStream, Vec<Ident>)>,
    ctx_ty: CtxTy,
}

// `EventVariantInfo` equality (the `PartialEq` impl) decides whether two variants
// can share one handler fn. Two infos are equal iff all three hold: their
// `ctx_ty` matches (a `CtxTy` enum comparison), their context `TokenStream`
// patterns match (compared by normalized string via `same_context_pattern`,
// since string comparison ignores span/token-identity differences), and their
// handle-ident lists match in order (via `same_handles`). The lookup table in
// `event_variant_info` emits handle idents in a fixed order, so an ordered
// `Vec<Ident>` comparison suffices.

// Handle idents are compared in order (the lookup table emits them in a fixed
// order). Returns true iff both are `None` or both `Some` with equal ident lists.
fn same_handles(
    a: &Option<(TokenStream, Vec<Ident>)>,
    b: &Option<(TokenStream, Vec<Ident>)>,
) -> bool {
    match (a, b) {
        (Some((_, a)), Some((_, b))) => a == b,
        (None, None) => true,
        _ => false,
    }
}

// Compares the `context` `TokenStream` patterns by normalized string
// (ignoring span/token-identity differences). This catches cases where two
// variants share the same handle-ident list but differ in the `EventContext`
// variant being unpacked.
fn same_context_pattern(
    a: &Option<(TokenStream, Vec<Ident>)>,
    b: &Option<(TokenStream, Vec<Ident>)>,
) -> bool {
    match (a, b) {
        (Some((pa, _)), Some((pb, _))) => pa.to_string() == pb.to_string(),
        (None, None) => true,
        _ => false,
    }
}

// `PartialEq` is required because `OnEventAttr`/`AnnotatedFn` derive it (tests
// compare `Vec<AnnotatedFn>`). It compares `ctx_ty` via `CtxTy` enum equality,
// the context `TokenStream` pattern by string (via `same_context_pattern`),
// and the handle idents via `same_handles`.
impl PartialEq for EventVariantInfo {
    fn eq(&self, other: &Self) -> bool {
        self.ctx_ty == other.ctx_ty
            && same_handles(&self.context, &other.context)
            && same_context_pattern(&self.context, &other.context)
    }
}

/// Maps an `Event` variant name to its dispatch info. Returns `None` for unknown
/// variants so the macro can emit a compile error (the user referenced a
/// non-existent event variant).
fn event_variant_info(variant: &Ident) -> Option<EventVariantInfo> {
    let p = |s: &str| syn::Ident::new(s, variant.span());
    let ctx_mod = quote! {
        cosmox_api::api::bindings::cosmox::plugin::context::EventContext
    };
    let no_handles = || EventVariantInfo {
        context: None,
        ctx_ty: CtxTy::Unit,
    };
    match variant.to_string().as_str() {
        "OnMetadataRawTreeReady" => Some(EventVariantInfo {
            context: Some((
                quote! { #ctx_mod::MetadataReadyContext((ref metadata_handle, ref path_mapping_handle)) },
                vec![p("metadata_handle"), p("path_mapping_handle")],
            )),
            ctx_ty: CtxTy::MetadataReady,
        }),
        "OnMetadataLocalTreeReady" => Some(EventVariantInfo {
            context: Some((
                quote! { #ctx_mod::MetadataLocalReadyContext((
                    ref metadata_handle, ref path_mapping_handle, ref tag_handle
                )) },
                vec![
                    p("metadata_handle"),
                    p("path_mapping_handle"),
                    p("tag_handle"),
                ],
            )),
            ctx_ty: CtxTy::MetadataLocalReady,
        }),
        "OnServerError" => Some(EventVariantInfo {
            context: None,
            ctx_ty: CtxTy::ServerError,
        }),
        "OnScanComplete"
        | "OnNewFileDiscovered"
        | "OnUserLogin"
        | "OnLibraryCrate"
        | "OnPluginInstall"
        | "OnPluginUninstall"
        | "OnPluginEnable"
        | "OnPluginDisable"
        | "OnServerStart"
        | "OnServerStop" => Some(no_handles()),
        _ => None,
    }
}

pub struct PluginAttr {
    media_types: Vec<String>,
}

impl Parse for PluginAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut media_types: Vec<String> = Vec::new();

        while !input.is_empty() {
            let name: Ident = input.parse()?;
            if name == "media_types" {
                let _: syn::Token![=] = input.parse()?;
                let content;
                syn::bracketed!(content in input);
                while !content.is_empty() {
                    let s: syn::LitStr = content.parse()?;
                    media_types.push(s.value());
                    if !content.is_empty() {
                        let _: syn::Token![,] = content.parse()?;
                    }
                }
            }
            if !input.is_empty() {
                let _: syn::Token![,] = input.parse()?;
            }
        }

        Ok(PluginAttr { media_types })
    }
}

#[derive(Debug, PartialEq)]
enum AnnotatedFn {
    OnLoad(Ident),
    OnUnload(Ident),
    OnEnable(Ident),
    OnDisable(Ident),
    OnEvent(Ident, OnEventAttr),
    OnCommand(Ident),
    OnConfig(Ident),
    Health(Ident),
    MediaTypes(Ident),
}

/// Returns `(annotation_name, attribute_tokens, attribute_span)`.
/// `attribute_tokens` is the content inside the attribute brackets, e.g. for
/// `#[on_event(OnScanComplete)]` it returns `("on_event", OnScanComplete, <span>)`.
/// `attribute_span` covers the full `#[...]` so errors underline the whole attribute.
fn extract_annotation(attrs: &mut Vec<Attribute>) -> Option<(String, TokenStream, Span)> {
    let attr_names = [
        "on_load",
        "on_unload",
        "on_enable",
        "on_disable",
        "on_event",
        "on_command",
        "on_config",
        "health",
        "media_types",
    ];

    let idx = attrs.iter().position(|x| {
        x.path()
            .segments
            .last()
            .map(|s| attr_names.contains(&s.ident.to_string().as_str()))
            .unwrap_or(false)
    })?;

    let attr = attrs.remove(idx);
    let span = attr.bracket_token.span.join();
    let name = attr.path().segments.last().unwrap().ident.to_string();
    let tokens = match &attr.meta {
        syn::Meta::List(list) => list.tokens.clone(),
        _ => TokenStream::new(),
    };
    Some((name, tokens, span))
}

struct ParsedPlugin {
    module_tokens: TokenStream,
    annotated: Vec<AnnotatedFn>,
}

fn parse_plugin_module(mod_item: &mut ItemMod) -> syn::Result<ParsedPlugin> {
    let mut annotated = Vec::new();
    let mut seen_on_load = false;
    let mut seen_on_unload = false;
    let mut seen_on_enable = false;
    let mut seen_on_disable = false;
    let mut seen_on_event_all: Option<String> = None;
    // Tracks the specific event variants already claimed by a `#[on_event(...)]`
    // handler, so the same variant registered twice is an error while *different*
    // variants may each have their own handler.
    let mut seen_on_event_filtered: Vec<Ident> = Vec::new();
    let mut seen_on_command = false;
    let mut seen_on_config = false;
    let mut seen_health = false;
    let mut seen_media_types = false;

    if let Some((_, items)) = &mut mod_item.content {
        for item in items.iter_mut() {
            if let syn::Item::Fn(func) = item
                && let Some((name, tokens, attr_span)) = extract_annotation(&mut func.attrs)
            {
                let fn_name = func.sig.ident.clone();
                let span = func.sig.ident.span();

                let already_seen = match name.as_str() {
                    "on_load" => Some((&mut seen_on_load, "`#[on_load]`")),
                    "on_unload" => Some((&mut seen_on_unload, "`#[on_unload]`")),
                    "on_enable" => Some((&mut seen_on_enable, "`#[on_enable]`")),
                    "on_disable" => Some((&mut seen_on_disable, "`#[on_disable]`")),
                    "on_event" => None,
                    "on_command" => Some((&mut seen_on_command, "`#[on_command]`")),
                    "on_config" => Some((&mut seen_on_config, "`#[on_config]`")),
                    "health" => Some((&mut seen_health, "`#[health]`")),
                    "media_types" => Some((&mut seen_media_types, "`#[media_types]`")),
                    _ => None,
                };

                if let Some((flag, label)) = already_seen {
                    if *flag {
                        return Err(syn::Error::new(
                            span,
                            format_args!("duplicate {label} annotation"),
                        ));
                    }
                    *flag = true;
                }

                let kind = match name.as_str() {
                    "on_load" => Some(AnnotatedFn::OnLoad(fn_name)),
                    "on_unload" => Some(AnnotatedFn::OnUnload(fn_name)),
                    "on_enable" => Some(AnnotatedFn::OnEnable(fn_name)),
                    "on_disable" => Some(AnnotatedFn::OnDisable(fn_name)),
                    "on_event" => {
                        // Parse the optional argument list.
                        let on_event_attr = if tokens.is_empty() {
                            OnEventAttr::All
                        } else {
                            let variants: EventVariants =
                                syn::parse2(tokens).map_err(|e| syn::Error::new(attr_span, e))?;
                            for v in &variants.variants {
                                if event_variant_info(v).is_none() {
                                    return Err(syn::Error::new(
                                        attr_span,
                                        format_args!(
                                            "unknown event variant `{v}` in `#[on_event(...)]`; it is not a variant of `cosmox_api::event::Event`",
                                            v = v,
                                        ),
                                    ));
                                }
                            }
                            // All variants in one `#[on_event(...)]` share a single
                            // handler fn, so their handler signatures must match: same
                            // handle list AND same payload (`ctx`) type. Check each
                            // separately so the error names the actual mismatch.
                            let base = event_variant_info(&variants.variants[0])
                                .expect("variant already validated above");
                            for v in &variants.variants[1..] {
                                let info =
                                    event_variant_info(v).expect("variant already validated above");
                                if !same_handles(&base.context, &info.context) {
                                    return Err(syn::Error::new(
                                        attr_span,
                                        format_args!(
                                            "event variants `{first}` and `{second}` have incompatible handler signatures (different context handles) in a single `#[on_event(...)]`; declare them with separate `#[on_event({first})]` / `#[on_event({second})]` attributes instead",
                                            first = variants.variants[0],
                                            second = v,
                                        ),
                                    ));
                                }
                                if base.ctx_ty != info.ctx_ty {
                                    return Err(syn::Error::new(
                                        attr_span,
                                        format_args!(
                                            "event variants `{first}` and `{second}` have incompatible handler signatures (different payload type: expected `{expected}`, got `{actual}`) in a single `#[on_event(...)]`; declare them with separate `#[on_event({first})]` / `#[on_event({second})]` attributes instead",
                                            first = variants.variants[0],
                                            second = v,
                                            expected = base.ctx_ty,
                                            actual = info.ctx_ty,
                                        ),
                                    ));
                                }
                            }
                            let filtered: Vec<(Ident, EventVariantInfo)> = variants
                                .variants
                                .into_iter()
                                .map(|v| {
                                    let info = event_variant_info(&v)
                                        .expect("variant already validated above");
                                    (v, info)
                                })
                                .collect();
                            OnEventAttr::Filtered(filtered)
                        };

                        // Conflict / duplicate detection for #[on_event].
                        match &on_event_attr {
                            OnEventAttr::All => {
                                if !seen_on_event_filtered.is_empty() {
                                    let listed = seen_on_event_filtered
                                        .iter()
                                        .map(|v| v.to_string())
                                        .collect::<Vec<_>>()
                                        .join(", ");
                                    return Err(syn::Error::new(
                                        attr_span,
                                        format_args!(
                                            "cannot combine `#[on_event]` with `#[on_event(...)]`; `#[on_event(...)]` already declares the following variant(s): {listed}",
                                            listed = listed,
                                        ),
                                    ));
                                }
                                if let Some(first_fn) = &seen_on_event_all {
                                    return Err(syn::Error::new(
                                        attr_span,
                                        format_args!(
                                            "duplicate `#[on_event]` annotation (first on `{first_fn}`)",
                                            first_fn = first_fn,
                                        ),
                                    ));
                                }
                                seen_on_event_all = Some(fn_name.to_string());
                            }
                            OnEventAttr::Filtered(variants) => {
                                if let Some(first_fn) = &seen_on_event_all {
                                    return Err(syn::Error::new(
                                        attr_span,
                                        format_args!(
                                            "cannot combine `#[on_event(...)]` with `#[on_event]`; `#[on_event]` is already on `{first_fn}`",
                                            first_fn = first_fn,
                                        ),
                                    ));
                                }
                                // The same variant may be handled by multiple
                                // `#[on_event(...)]` functions, so we do not reject
                                // duplicates here. Record every variant so the
                                // All/Filtered mutual-exclusion check above still works.
                                for (v, _) in variants {
                                    seen_on_event_filtered.push(v.clone());
                                }
                            }
                        }

                        Some(AnnotatedFn::OnEvent(fn_name, on_event_attr))
                    }
                    "on_command" => Some(AnnotatedFn::OnCommand(fn_name)),
                    "on_config" => Some(AnnotatedFn::OnConfig(fn_name)),
                    "health" => Some(AnnotatedFn::Health(fn_name)),
                    "media_types" => Some(AnnotatedFn::MediaTypes(fn_name)),
                    _ => None,
                };
                if let Some(k) = kind {
                    // Make function accessible from generated code outside the module
                    func.vis = Visibility::Public(Default::default());
                    annotated.push(k);
                }
            }
        }
    }

    let module_tokens = quote! { #mod_item };
    Ok(ParsedPlugin {
        module_tokens,
        annotated,
    })
}

fn generate_plugin(
    attr: PluginAttr,
    module_name: &Ident,
    parsed: ParsedPlugin,
) -> syn::Result<TokenStream> {
    let module = &parsed.module_tokens;
    let mod_name = module_name;

    let on_load = parsed.annotated.iter().find_map(|a| match a {
        AnnotatedFn::OnLoad(n) => Some(n.clone()),
        _ => None,
    });
    let on_unload = parsed.annotated.iter().find_map(|a| match a {
        AnnotatedFn::OnUnload(n) => Some(n.clone()),
        _ => None,
    });
    let on_enable = parsed.annotated.iter().find_map(|a| match a {
        AnnotatedFn::OnEnable(n) => Some(n.clone()),
        _ => None,
    });
    let on_disable = parsed.annotated.iter().find_map(|a| match a {
        AnnotatedFn::OnDisable(n) => Some(n.clone()),
        _ => None,
    });
    // Collect *every* `#[on_event]` / `#[on_event(...)]` handler. A single
    // event variant may be handled by multiple handlers, so we no longer take
    // only the first one.
    let on_events: Vec<(Ident, OnEventAttr)> = parsed
        .annotated
        .iter()
        .filter_map(|a| match a {
            AnnotatedFn::OnEvent(n, attr) => Some((n.clone(), attr.clone())),
            _ => None,
        })
        .collect();
    // Convenience views:
    let on_event_all = on_events
        .iter()
        .find(|(_, attr)| matches!(attr, OnEventAttr::All));
    let on_event_filtered: Vec<(Ident, Vec<(Ident, EventVariantInfo)>)> = on_events
        .iter()
        .filter_map(|(n, attr)| match attr {
            OnEventAttr::Filtered(v) => Some((n.clone(), v.clone())),
            _ => None,
        })
        .collect();
    let on_command = parsed.annotated.iter().find_map(|a| match a {
        AnnotatedFn::OnCommand(n) => Some(n.clone()),
        _ => None,
    });
    let on_config = parsed.annotated.iter().find_map(|a| match a {
        AnnotatedFn::OnConfig(n) => Some(n.clone()),
        _ => None,
    });
    let health = parsed.annotated.iter().find_map(|a| match a {
        AnnotatedFn::Health(n) => Some(n.clone()),
        _ => None,
    });
    let media_types_fn = parsed.annotated.iter().find_map(|a| match a {
        AnnotatedFn::MediaTypes(n) => Some(n.clone()),
        _ => None,
    });

    // Error: both #[media_types] fn and #[plugin(media_types=[...])] specified
    if let Some(ref fn_ident) = media_types_fn
        && !attr.media_types.is_empty()
    {
        return Err(syn::Error::new(
            fn_ident.span(),
            format_args!(
                "conflict: `{mod_name}::{fn_name}` is annotated with `#[media_types]`, \
                     but `media_types` are also set in `#[plugin(media_types = {types:?})]`",
                mod_name = module_name,
                fn_name = fn_ident,
                types = attr.media_types,
            ),
        ));
    }

    let mod_path = quote! { super::#mod_name };

    let media_types_body = match media_types_fn {
        Some(ref fn_name) => quote! { #mod_path::#fn_name() },
        None if !attr.media_types.is_empty() => {
            let types: Vec<TokenStream> = attr
                .media_types
                .iter()
                .map(|t| quote! { ::std::convert::From::from(#t) })
                .collect();
            quote! { vec![ #(#types),* ] }
        }
        _ => quote! { ::std::vec::Vec::new() },
    };

    // Build event registration statements for #[on_event(Variant, ...)].
    // A variant may be declared by multiple handlers, so we register each
    // distinct variant only once.
    let registration_stmts = if on_event_filtered.is_empty() {
        quote! {}
    } else {
        let mut seen: Vec<Ident> = Vec::new();
        let reg_calls: Vec<TokenStream> = on_event_filtered
            .iter()
            .flat_map(|(_, variants)| variants)
            .filter(|(v, _)| {
                if seen.iter().any(|s| s == v) {
                    false
                } else {
                    seen.push(v.clone());
                    true
                }
            })
            .map(|(v, _)| {
                quote! {
                    cosmox_api::event::Event::#v(
                        cosmox_api::event::EventPayload::Cond(
                            ::std::default::Default::default(),
                        ),
                    ).register().expect(
                        concat!("failed to register ", stringify!(#v), " event listener"),
                    );
                }
            })
            .collect();
        quote! { #(#reg_calls)* }
    };

    let on_load_body = match on_load {
        Some(ref fn_name) => quote! {
            #registration_stmts
            #mod_path::#fn_name(config)
        },
        None => quote! {
            #registration_stmts
            cosmox_api::api::bindings::exports::cosmox::plugin::core_lifecycle::PluginResult::Ok
        },
    };

    let on_unload_body = match on_unload {
        Some(ref fn_name) => quote! { #mod_path::#fn_name() },
        None => quote! {
            cosmox_api::api::bindings::exports::cosmox::plugin::core_lifecycle::PluginResult::Ok
        },
    };

    let on_enable_body = match on_enable {
        Some(ref fn_name) => quote! { #mod_path::#fn_name() },
        None => quote! {
            cosmox_api::api::bindings::exports::cosmox::plugin::core_lifecycle::PluginResult::Ok
        },
    };

    let on_disable_body = match on_disable {
        Some(ref fn_name) => quote! { #mod_path::#fn_name() },
        None => quote! {
            cosmox_api::api::bindings::exports::cosmox::plugin::core_lifecycle::PluginResult::Ok
        },
    };

    // Generated `on_event` body.
    //
    // `#[on_event]` (All): forward the raw bytes + EventContext unchanged.
    //
    // `#[on_event(Variant, ...)]` (filtered): decode the payload, match the
    // variant, unpack the strongly-typed `EventPayload::Data(ctx)` (and, for
    // handle-bearing events, the `EventContext` handles), then call **every**
    // handler that declared this variant. The same variant may be handled by
    // multiple `#[on_event(...)]` functions, so all of them are invoked in
    // declaration order; the first non-`Ok` result short-circuits.
    // Non-matching events / decode failures / context mismatches fall through
    // to `PluginResult::Ok` with a log line.
    let on_event_body = if let Some((all_fn, _)) = on_event_all {
        quote! { #mod_path::#all_fn(event, event_context) }
    } else if !on_event_filtered.is_empty() {
        // Map each variant to the list of handler fns that declared it,
        // carrying the EventVariantInfo directly so the dispatch builder
        // doesn't need a second lookup.
        let mut variant_handlers: Vec<(Ident, Vec<Ident>, EventVariantInfo)> = Vec::new();
        for (fn_name, variants) in &on_event_filtered {
            for (v, info) in variants {
                match variant_handlers.iter_mut().find(|(vv, _, _)| vv == v) {
                    Some((_, fns, _)) => fns.push(fn_name.clone()),
                    None => variant_handlers.push((v.clone(), vec![fn_name.clone()], info.clone())),
                }
            }
        }

        let dispatch_arms: Vec<TokenStream> = variant_handlers
            .iter()
            .map(|(v, fns, info)| {
                // The context (handle) unpacking is identical for every handler
                // of this variant, so we do it ONCE per variant and reuse the
                // handles for all handler calls. Each handler receives its own
                // clone of `ctx` to avoid a use-after-move.
                let calls: Vec<TokenStream> = fns
                    .iter()
                    .map(|fn_name| match &info.context {
                        Some((_ctx_pat, handles)) => quote! {
                            let __r = #mod_path::#fn_name(ctx.clone(), #(#handles),*);
                            if let cosmox_api::api::bindings::exports::cosmox::plugin::host_notifier::PluginResult::Ok = __r {
                                // ok, continue to next handler
                            } else {
                                return __r;
                            }
                        },
                        None => quote! {
                            let __r = #mod_path::#fn_name(ctx.clone());
                            if let cosmox_api::api::bindings::exports::cosmox::plugin::host_notifier::PluginResult::Ok = __r {
                                // ok, continue to next handler
                            } else {
                                return __r;
                            }
                        },
                    })
                    .collect();
                // Wrap the calls in a single context-unpacking block (per variant).
                let arm_body = match &info.context {
                    Some((ctx_pat, _handles)) => quote! {
                        if let #ctx_pat = event_context {
                            #(#calls)*
                        } else {
                            log::warn!(
                                "event {} received an unexpected event_context; ignoring",
                                stringify!(#v),
                            );
                        }
                    },
                    // The `on_event` trait method has a fixed signature that always
                    // takes an `event_context` parameter, but handle-less variants
                    // never read it. Discard it here so generated plugins don't emit
                    // an `unused variable: event_context` warning.
                    None => quote! {
                        let _ = event_context;
                        #(#calls)*
                    },
                };
                quote! {
                    cosmox_api::event::Event::#v(
                        cosmox_api::event::EventPayload::Data(ctx),
                    ) => {
                        #arm_body
                        cosmox_api::api::bindings::exports::cosmox::plugin::host_notifier::PluginResult::Ok
                    }
                }
            })
            .collect();
        quote! {
            match cosmox_api::event::Event::decode(event) {
                ::std::result::Result::Ok(event) => match event {
                    #(#dispatch_arms)*
                    _ => {
                        log::debug!("on_event: unhandled event variant, ignoring");
                        cosmox_api::api::bindings::exports::cosmox::plugin::host_notifier::PluginResult::Ok
                    }
                },
                ::std::result::Result::Err(err) => {
                    log::error!("on_event: failed to decode event payload: {err:#?}");
                    cosmox_api::api::bindings::exports::cosmox::plugin::host_notifier::PluginResult::Ok
                }
            }
        }
    } else {
        quote! {
            cosmox_api::api::bindings::exports::cosmox::plugin::host_notifier::PluginResult::Ok
        }
    };

    let on_command_body = match on_command {
        Some(ref fn_name) => quote! { #mod_path::#fn_name(command_name, args) },
        None => quote! {
            cosmox_api::api::bindings::exports::cosmox::plugin::command_handler::PluginResult::Ok
        },
    };

    let on_config_body = match on_config {
        Some(ref fn_name) => quote! { #mod_path::#fn_name(config) },
        None => quote! {
            cosmox_api::api::bindings::exports::cosmox::plugin::configuration_manager::PluginResult::Ok
        },
    };

    let health_body = match health {
        Some(ref fn_name) => quote! { #mod_path::#fn_name() },
        None => quote! {
            cosmox_api::api::bindings::exports::cosmox::plugin::telemetry_reporter::PluginHealthStatus {
                status: cosmox_api::api::bindings::cosmox::plugin::cosmox_types::PluginStatus::Ok,
                message: ::std::option::Option::None,
                metrics: ::std::option::Option::None,
            }
        },
    };

    let init = quote! { let _ = cosmox_api::api::Cosmox::init(); };

    let output = quote! {
        #module

        mod __plugin {
            pub(crate) struct Plugin;

            impl cosmox_api::api::bindings::Guest for Plugin {
                fn run() {
                    #init
                }
            }

            impl cosmox_api::api::bindings::exports::cosmox::plugin::core_lifecycle::Guest for Plugin {
                fn on_load(
                    config: cosmox_api::api::bindings::exports::cosmox::plugin::core_lifecycle::ConfigData,
                ) -> cosmox_api::api::bindings::exports::cosmox::plugin::core_lifecycle::PluginResult {
                    #init
                    #on_load_body
                }

                fn on_unload(
                ) -> cosmox_api::api::bindings::exports::cosmox::plugin::core_lifecycle::PluginResult {
                    #init
                    #on_unload_body
                }

                fn on_enable(
                ) -> cosmox_api::api::bindings::exports::cosmox::plugin::core_lifecycle::PluginResult {
                    #init
                    #on_enable_body
                }

                fn on_disable(
                ) -> cosmox_api::api::bindings::exports::cosmox::plugin::core_lifecycle::PluginResult {
                    #init
                    #on_disable_body
                }
            }

            impl cosmox_api::api::bindings::exports::cosmox::plugin::configuration_manager::Guest for Plugin {
                fn get_current_config(
                ) -> cosmox_api::api::bindings::exports::cosmox::plugin::configuration_manager::ConfigData
                {
                    #init
                    cosmox_api::api::bindings::exports::cosmox::plugin::configuration_manager::ConfigData {
                        id: ::std::string::String::new(),
                        name: ::std::string::String::new(),
                        settings: ::std::string::String::from("{}"),
                    }
                }

                fn apply_new_config(
                    config: cosmox_api::api::bindings::exports::cosmox::plugin::configuration_manager::ConfigData,
                ) -> cosmox_api::api::bindings::exports::cosmox::plugin::configuration_manager::PluginResult
                {
                    #init
                    #on_config_body
                }

                fn supported_media_types() -> ::std::vec::Vec<::std::string::String> {
                    #init
                    #media_types_body
                }
            }

            impl cosmox_api::api::bindings::exports::cosmox::plugin::host_notifier::Guest for Plugin {
                fn on_event(
                    event: ::std::vec::Vec<u8>,
                    event_context: cosmox_api::api::bindings::exports::cosmox::plugin::host_notifier::EventContext,
                ) -> cosmox_api::api::bindings::exports::cosmox::plugin::host_notifier::PluginResult
                {
                    #init
                    #on_event_body
                }
            }

            impl cosmox_api::api::bindings::exports::cosmox::plugin::telemetry_reporter::Guest for Plugin {
                fn get_health_status(
                ) -> cosmox_api::api::bindings::exports::cosmox::plugin::telemetry_reporter::PluginHealthStatus
                {
                    #init
                    #health_body
                }
            }

            impl cosmox_api::api::bindings::exports::cosmox::plugin::command_handler::Guest for Plugin {
                fn execute_command(
                    command_name: ::std::string::String,
                    args: ::std::vec::Vec<::std::string::String>,
                ) -> cosmox_api::api::bindings::exports::cosmox::plugin::command_handler::PluginResult
                {
                    #init
                    #on_command_body
                }
            }

            cosmox_api::api::bindings::export!(Plugin);
        }

        pub(crate) use __plugin::Plugin;
    };
    Ok(output)
}

pub fn expand(attr: PluginAttr, mut input: ItemMod) -> syn::Result<TokenStream> {
    let module_name = input.ident.clone();

    let parsed = parse_plugin_module(&mut input)?;
    let expanded = generate_plugin(attr, &module_name, parsed)?;

    Ok(expanded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::format_ident;

    struct ExpectedBodies {
        on_load: TokenStream,
        on_unload: TokenStream,
        on_enable: TokenStream,
        on_disable: TokenStream,
        on_event: TokenStream,
        on_command: TokenStream,
        on_config: TokenStream,
        health: TokenStream,
        media_types: TokenStream,
    }

    impl Default for ExpectedBodies {
        fn default() -> Self {
            let core_ok = quote! {
                cosmox_api::api::bindings::exports::cosmox::plugin::core_lifecycle::PluginResult::Ok
            };
            let host_ok = quote! {
                cosmox_api::api::bindings::exports::cosmox::plugin::host_notifier::PluginResult::Ok
            };
            let cmd_ok = quote! {
                cosmox_api::api::bindings::exports::cosmox::plugin::command_handler::PluginResult::Ok
            };
            let cfg_ok = quote! {
                cosmox_api::api::bindings::exports::cosmox::plugin::configuration_manager::PluginResult::Ok
            };
            Self {
                on_load: core_ok.clone(),
                on_unload: core_ok.clone(),
                on_enable: core_ok.clone(),
                on_disable: core_ok.clone(),
                on_event: host_ok,
                on_command: cmd_ok,
                on_config: cfg_ok,
                health: quote! {
                    cosmox_api::api::bindings::exports::cosmox::plugin::telemetry_reporter::PluginHealthStatus {
                        status: cosmox_api::api::bindings::cosmox::plugin::cosmox_types::PluginStatus::Ok,
                        message: ::std::option::Option::None,
                        metrics: ::std::option::Option::None,
                    }
                },
                media_types: quote! { ::std::vec::Vec::new() },
            }
        }
    }

    fn expected_plugin_output(module: TokenStream, bodies: ExpectedBodies) -> TokenStream {
        let init = quote! { let _ = cosmox_api::api::Cosmox::init(); };
        let on_load_body = bodies.on_load;
        let on_unload_body = bodies.on_unload;
        let on_enable_body = bodies.on_enable;
        let on_disable_body = bodies.on_disable;
        let on_event_body = bodies.on_event;
        let on_command_body = bodies.on_command;
        let on_config_body = bodies.on_config;
        let health_body = bodies.health;
        let media_types_body = bodies.media_types;

        quote! {
            #module

            mod __plugin {
                pub(crate) struct Plugin;

                impl cosmox_api::api::bindings::Guest for Plugin {
                    fn run() { #init }
                }

                impl cosmox_api::api::bindings::exports::cosmox::plugin::core_lifecycle::Guest for Plugin {
                    fn on_load(
                        config: cosmox_api::api::bindings::exports::cosmox::plugin::core_lifecycle::ConfigData,
                    ) -> cosmox_api::api::bindings::exports::cosmox::plugin::core_lifecycle::PluginResult {
                        #init
                        #on_load_body
                    }

                    fn on_unload(
                    ) -> cosmox_api::api::bindings::exports::cosmox::plugin::core_lifecycle::PluginResult {
                        #init
                        #on_unload_body
                    }

                    fn on_enable(
                    ) -> cosmox_api::api::bindings::exports::cosmox::plugin::core_lifecycle::PluginResult {
                        #init
                        #on_enable_body
                    }

                    fn on_disable(
                    ) -> cosmox_api::api::bindings::exports::cosmox::plugin::core_lifecycle::PluginResult {
                        #init
                        #on_disable_body
                    }
                }

                impl cosmox_api::api::bindings::exports::cosmox::plugin::configuration_manager::Guest for Plugin {
                    fn get_current_config(
                    ) -> cosmox_api::api::bindings::exports::cosmox::plugin::configuration_manager::ConfigData
                    {
                        #init
                        cosmox_api::api::bindings::exports::cosmox::plugin::configuration_manager::ConfigData {
                            id: ::std::string::String::new(),
                            name: ::std::string::String::new(),
                            settings: ::std::string::String::from("{}"),
                        }
                    }

                    fn apply_new_config(
                        config: cosmox_api::api::bindings::exports::cosmox::plugin::configuration_manager::ConfigData,
                    ) -> cosmox_api::api::bindings::exports::cosmox::plugin::configuration_manager::PluginResult
                    {
                        #init
                        #on_config_body
                    }

                    fn supported_media_types() -> ::std::vec::Vec<::std::string::String> {
                        #init
                        #media_types_body
                    }
                }

                impl cosmox_api::api::bindings::exports::cosmox::plugin::host_notifier::Guest for Plugin {
                    fn on_event(
                        event: ::std::vec::Vec<u8>,
                        event_context: cosmox_api::api::bindings::exports::cosmox::plugin::host_notifier::EventContext,
                    ) -> cosmox_api::api::bindings::exports::cosmox::plugin::host_notifier::PluginResult
                    {
                        #init
                        #on_event_body
                    }
                }

                impl cosmox_api::api::bindings::exports::cosmox::plugin::telemetry_reporter::Guest for Plugin {
                    fn get_health_status(
                    ) -> cosmox_api::api::bindings::exports::cosmox::plugin::telemetry_reporter::PluginHealthStatus
                    {
                        #init
                        #health_body
                    }
                }

                impl cosmox_api::api::bindings::exports::cosmox::plugin::command_handler::Guest for Plugin {
                    fn execute_command(
                        command_name: ::std::string::String,
                        args: ::std::vec::Vec<::std::string::String>,
                    ) -> cosmox_api::api::bindings::exports::cosmox::plugin::command_handler::PluginResult
                    {
                        #init
                        #on_command_body
                    }
                }

                cosmox_api::api::bindings::export!(Plugin);
            }

            pub(crate) use __plugin::Plugin;
        }
    }

    /// Parse a `mod` block string into an ItemMod for testing.
    fn parse_mod(input: &str) -> ItemMod {
        let full = format!("mod test_mod {{ {input} }}");
        syn::parse_str(&full).expect("failed to parse test module")
    }

    #[test]
    fn extract_on_load() {
        let mut item: syn::Item = syn::parse_str("#[on_load]\nfn foo() {}").unwrap();
        let func = match &mut item {
            syn::Item::Fn(f) => f,
            _ => panic!("expected fn"),
        };
        assert_eq!(
            extract_annotation(&mut func.attrs).map(|(n, _, _)| n),
            Some("on_load".into())
        );
        assert!(func.attrs.is_empty(), "annotation should be consumed");
    }

    #[test]
    fn extract_unknown_annotation_returns_none() {
        let mut item: syn::Item = syn::parse_str("#[foobar]\nfn x() {}").unwrap();
        let func = match &mut item {
            syn::Item::Fn(f) => f,
            _ => panic!("expected fn"),
        };
        assert_eq!(extract_annotation(&mut func.attrs).map(|(n, _, _)| n), None);
        // unknown annotation should NOT be consumed
        assert_eq!(func.attrs.len(), 1);
    }

    #[test]
    fn parse_single_annotation() {
        let input = r#"#[on_load] fn my_load() {}"#;
        let mut mod_item = parse_mod(input);
        let parsed = parse_plugin_module(&mut mod_item).unwrap();
        assert_eq!(
            parsed.annotated,
            vec![AnnotatedFn::OnLoad(format_ident!("my_load"))]
        );
    }

    #[test]
    fn parse_multiple_annotations() {
        let input = r#"
            #[on_load]    fn load_fn() {}
            #[on_event]   fn event_fn() {}
            #[health]     fn health_fn() {}
        "#;
        let mut mod_item = parse_mod(input);
        let parsed = parse_plugin_module(&mut mod_item).unwrap();
        assert_eq!(
            parsed.annotated,
            vec![
                AnnotatedFn::OnLoad(format_ident!("load_fn")),
                AnnotatedFn::OnEvent(format_ident!("event_fn"), OnEventAttr::All),
                AnnotatedFn::Health(format_ident!("health_fn")),
            ]
        );
    }

    #[test]
    fn parse_all_annotation_kinds() {
        use AnnotatedFn::*;
        let input = r#"
            #[on_load]     fn a() {}
            #[on_unload]   fn b() {}
            #[on_enable]   fn c() {}
            #[on_disable]  fn d() {}
            #[on_event]    fn e() {}
            #[on_command]  fn f() {}
            #[on_config]   fn g() {}
            #[health]      fn h() {}
            #[media_types] fn i() {}
        "#;
        let mut mod_item = parse_mod(input);
        let parsed = parse_plugin_module(&mut mod_item).unwrap();
        assert_eq!(
            parsed.annotated,
            vec![
                OnLoad(format_ident!("a")),
                OnUnload(format_ident!("b")),
                OnEnable(format_ident!("c")),
                OnDisable(format_ident!("d")),
                OnEvent(format_ident!("e"), OnEventAttr::All),
                OnCommand(format_ident!("f")),
                OnConfig(format_ident!("g")),
                Health(format_ident!("h")),
                MediaTypes(format_ident!("i")),
            ]
        );
    }

    #[test]
    fn parse_makes_fns_pub() {
        let input = r#"
            #[on_load]
            fn private_fn() {}
        "#;
        let mut mod_item = parse_mod(input);
        let _parsed = parse_plugin_module(&mut mod_item).unwrap();
        // retrieve the fn back and check visibility
        if let Some((_, items)) = &mod_item.content {
            if let syn::Item::Fn(func) = &items[0] {
                assert!(
                    matches!(func.vis, Visibility::Public(_)),
                    "annotated fn should be made pub"
                );
            } else {
                panic!("expected fn item");
            }
        }
    }

    #[test]
    fn preserves_non_plugin_items_in_output() {
        let input = r#"
            const NAME: &str = "test";

            use std::collections::HashMap;

            fn helper() {}

            struct Dummy;

            #[on_load]
            fn init() {}
        "#;
        let mut mod_item = parse_mod(input);
        let parsed = parse_plugin_module(&mut mod_item).unwrap();
        let attr = PluginAttr {
            media_types: vec![],
        };
        let output = generate_plugin(attr, &format_ident!("test_mod"), parsed).unwrap();

        let module = quote! {
            mod test_mod {
                const NAME : & str = "test";
                use std :: collections :: HashMap ;
                fn helper () {}
                struct Dummy ;
                pub fn init () {}
            }
        };
        let expected = expected_plugin_output(
            module,
            ExpectedBodies {
                on_load: quote! { super :: test_mod :: init (config) },
                ..Default::default()
            },
        );
        assert_eq!(output.to_string(), expected.to_string());
    }

    #[test]
    fn parse_duplicate_on_load_is_err() {
        let input = r#"
            #[on_load]
            fn load_a() {}

            #[on_load]
            fn load_b() {}
        "#;
        let mut mod_item = parse_mod(input);
        assert!(
            parse_plugin_module(&mut mod_item).is_err(),
            "two #[on_load] should produce an error"
        );
    }

    #[test]
    fn parse_duplicate_on_event_is_err() {
        let input = r#"
            #[on_event]
            fn evt_a() {}

            #[on_event]
            fn evt_b() {}
        "#;
        let mut mod_item = parse_mod(input);
        assert!(
            parse_plugin_module(&mut mod_item).is_err(),
            "two #[on_event] should produce an error"
        );
    }

    #[test]
    fn parse_duplicate_media_types_is_err() {
        let input = r#"
            #[media_types]
            fn mt_a() {}

            #[media_types]
            fn mt_b() {}
        "#;
        let mut mod_item = parse_mod(input);
        assert!(
            parse_plugin_module(&mut mod_item).is_err(),
            "two #[media_types] should produce an error"
        );
    }

    #[test]
    fn media_types_conflict_is_err() {
        let input = r#"
            #[media_types]
            fn get_types() -> Vec<String> { vec![] }
        "#;
        let mut mod_item = parse_mod(input);
        let parsed = parse_plugin_module(&mut mod_item).unwrap();
        let attr = PluginAttr {
            media_types: vec!["Movie".into()],
        };
        assert!(
            generate_plugin(attr, &format_ident!("test_mod"), parsed).is_err(),
            "both func- and attr-based media_types should conflict"
        );
    }

    #[test]
    fn parse_plugin_attr_empty() {
        let attr: PluginAttr = syn::parse_str("").unwrap();
        assert!(attr.media_types.is_empty());
    }

    #[test]
    fn parse_plugin_attr_with_types() {
        let attr: PluginAttr = syn::parse_str(r#"media_types = ["Movie", "TV"]"#).unwrap();
        assert_eq!(attr.media_types, vec!["Movie", "TV"]);
    }

    #[test]
    fn parse_plugin_attr_single_type() {
        let attr: PluginAttr = syn::parse_str(r#"media_types = ["Audio"]"#).unwrap();
        assert_eq!(attr.media_types, vec!["Audio"]);
    }

    #[test]
    fn expand_round_trip() {
        let input: ItemMod = syn::parse_str("mod my_plugin { #[on_load] fn init() {} }").unwrap();
        let attr = PluginAttr {
            media_types: vec![],
        };
        let result = super::expand(attr, input);
        assert!(result.is_ok());
        let output = result.unwrap();

        let module = quote! {
            mod my_plugin { pub fn init() {} }
        };
        let expected = expected_plugin_output(
            module,
            ExpectedBodies {
                on_load: quote! { super :: my_plugin :: init (config) },
                ..Default::default()
            },
        );
        assert_eq!(output.to_string(), expected.to_string());
    }

    #[test]
    fn generates_init_in_each_trait_method() {
        let mod_item = parse_mod("fn _unused() {}");
        let mut mod_item = mod_item;
        let parsed = parse_plugin_module(&mut mod_item).unwrap();
        let attr = PluginAttr {
            media_types: vec![],
        };
        let output = generate_plugin(attr, &format_ident!("test_mod"), parsed).unwrap();

        let module = quote! { mod test_mod { fn _unused() {} } };
        let expected = expected_plugin_output(module, ExpectedBodies::default());
        assert_eq!(output.to_string(), expected.to_string());
    }

    #[test]
    fn media_types_from_annotated_fn() {
        let input = r#"
            #[media_types]
            fn get_types() -> Vec<String> { vec![] }
        "#;
        let mut mod_item = parse_mod(input);
        let parsed = parse_plugin_module(&mut mod_item).unwrap();
        let attr = PluginAttr {
            media_types: vec![],
        };
        let output = generate_plugin(attr, &format_ident!("test_mod"), parsed).unwrap();

        let module = quote! {
            mod test_mod { pub fn get_types() -> Vec<String> { vec![] } }
        };
        let expected = expected_plugin_output(
            module,
            ExpectedBodies {
                media_types: quote! { super :: test_mod :: get_types () },
                ..Default::default()
            },
        );
        assert_eq!(output.to_string(), expected.to_string());
    }

    #[test]
    fn full_output_on_load_and_event_with_media_types() {
        let input = r#"
            #[on_load]
            fn my_init() {}

            #[on_event]
            fn my_event() {}
        "#;
        let mut mod_item = parse_mod(input);
        let mod_name = mod_item.ident.clone();
        let parsed = parse_plugin_module(&mut mod_item).unwrap();
        let attr = PluginAttr {
            media_types: vec!["Movie".into()],
        };
        let output = generate_plugin(attr, &mod_name, parsed).unwrap();

        let module = quote! {
            mod test_mod {
                pub fn my_init() {}
                pub fn my_event() {}
            }
        };
        let expected = expected_plugin_output(
            module,
            ExpectedBodies {
                on_load: quote! { super :: test_mod :: my_init (config) },
                on_event: quote! { super :: test_mod :: my_event (event , event_context) },
                media_types: quote! { vec![::std::convert::From::from("Movie")] },
                ..Default::default()
            },
        );
        assert_eq!(output.to_string(), expected.to_string());
    }

    #[test]
    fn filtered_same_variant_multiple_handlers_coexist() {
        // The same variant may be handled by multiple #[on_event(...)] fns;
        // both handlers must be invoked when that event is dispatched.
        let input = r#"
            #[on_event(OnScanComplete)]
            fn a() {}

            #[on_event(OnScanComplete)]
            fn b() {}
        "#;
        let mut mod_item = parse_mod(input);
        let parsed = parse_plugin_module(&mut mod_item)
            .expect("same variant across handlers should be allowed");
        let attr = PluginAttr {
            media_types: vec![],
        };
        let output = generate_plugin(attr, &format_ident!("test_mod"), parsed).unwrap();
        let s = output.to_string();
        assert!(
            s.contains("super :: test_mod :: a (ctx . clone ())"),
            "first handler should be called: {s}"
        );
        assert!(
            s.contains("super :: test_mod :: b (ctx . clone ())"),
            "second handler should also be called: {s}"
        );
        // Both share the same OnScanComplete arm, so only one register call.
        let reg_count = s.matches("register ()").count();
        assert!(
            reg_count == 1,
            "OnScanComplete should be registered exactly once (found {reg_count}): {s}"
        );
    }

    #[test]
    fn filtered_different_variants_coexist_is_ok() {
        let input = r#"
            #[on_event(OnScanComplete)]
            fn a() {}

            #[on_event(OnUserLogin)]
            fn b() {}
        "#;
        let mut mod_item = parse_mod(input);
        assert!(
            parse_plugin_module(&mut mod_item).is_ok(),
            "different variants should each be allowed their own handler"
        );
    }

    #[test]
    fn on_event_all_and_filtered_conflict_is_err() {
        let input = r#"
            #[on_event]
            fn handle_all() {}

            #[on_event(OnScanComplete)]
            fn handle_one() {}
        "#;
        let mut mod_item = parse_mod(input);
        assert!(
            parse_plugin_module(&mut mod_item).is_err(),
            "mixing #[on_event] and #[[on_event(...)] should be an error"
        );
    }

    #[test]
    fn on_event_filtered_then_all_conflict_is_err() {
        // Reverse order of the all/filtered conflict: the filtered handler
        // appears first, then the bare `#[on_event]`. The All arm must reject
        // this too (it checks `seen_on_event_filtered`), which is a different
        // code path from `on_event_all_and_filtered_conflict_is_err`.
        let input = r#"
            #[on_event(OnScanComplete)]
            fn handle_one() {}

            #[on_event]
            fn handle_all() {}
        "#;
        let mut mod_item = parse_mod(input);
        assert!(
            parse_plugin_module(&mut mod_item).is_err(),
            "mixing #[on_event(...)] then #[on_event] should be an error"
        );
    }

    #[test]
    fn on_event_all_lists_all_filtered_variants_in_error() {
        // When `#[on_event]` (All) conflicts with multiple filtered handlers,
        // the error must list EVERY declared variant, not only the first one.
        let input = r#"
            #[on_event(OnScanComplete)]
            fn a() {}

            #[on_event(OnUserLogin, OnLibraryCrate)]
            fn b() {}

            #[on_event]
            fn handle_all() {}
        "#;
        let mut mod_item = parse_mod(input);
        let msg = match parse_plugin_module(&mut mod_item) {
            Ok(_) => panic!("mixing #[on_event] and #[on_event(...)] should be an error"),
            Err(e) => e.to_string(),
        };
        assert!(
            msg.contains("OnScanComplete"),
            "error should mention OnScanComplete: {msg}"
        );
        assert!(
            msg.contains("OnUserLogin"),
            "error should mention OnUserLogin: {msg}"
        );
        assert!(
            msg.contains("OnLibraryCrate"),
            "error should mention OnLibraryCrate: {msg}"
        );
    }

    #[test]
    fn on_event_unknown_variant_is_err() {
        let input = r#"
            #[on_event(NotARealEvent)]
            fn a() {}
        "#;
        let mut mod_item = parse_mod(input);
        assert!(
            parse_plugin_module(&mut mod_item).is_err(),
            "referencing a non-existent event variant should be an error"
        );
    }

    #[test]
    fn on_event_filtered_incompatible_variant_signatures_is_err() {
        // A single #[on_event(...)] shares one handler fn, so variants with
        // different handler signatures (here OnMetadataRawTreeReady carries
        // handles while OnScanComplete does not) must be rejected with a clear
        // message rather than producing call code that fails to compile.
        let input = r#"
            #[on_event(OnMetadataRawTreeReady, OnScanComplete)]
            fn a() {}
        "#;
        let mut mod_item = parse_mod(input);
        let msg = match parse_plugin_module(&mut mod_item) {
            Ok(_) => panic!(
                "mixing incompatible-signature variants in one #[on_event(...)] should be an error"
            ),
            Err(e) => e.to_string(),
        };
        assert!(
            msg.contains("OnMetadataRawTreeReady"),
            "error should name the first variant: {msg}"
        );
        assert!(
            msg.contains("OnScanComplete"),
            "error should name the conflicting variant: {msg}"
        );
    }

    #[test]
    fn on_event_filtered_incompatible_ctx_type_is_err() {
        // Both variants carry no handles, so a handle-only check would accept
        // them — but their payload (`ctx`) types differ (`()` vs
        // `OnServerErrorEventContext`), so one shared handler fn cannot serve
        // both. The compatibility check must reject this with a payload-type
        // specific message (not a handle-mismatch message).
        let input = r#"
            #[on_event(OnScanComplete, OnServerError)]
            fn a() {}
        "#;
        let mut mod_item = parse_mod(input);
        let msg = match parse_plugin_module(&mut mod_item) {
            Ok(_) => panic!(
                "mixing different-payload-type variants in one #[on_event(...)] should be an error"
            ),
            Err(e) => e.to_string(),
        };
        assert!(
            msg.contains("OnScanComplete"),
            "error should name the first variant: {msg}"
        );
        assert!(
            msg.contains("OnServerError"),
            "error should name the conflicting variant: {msg}"
        );
        assert!(
            msg.contains("different payload type"),
            "error should pinpoint the payload-type mismatch: {msg}"
        );
        assert!(
            !msg.contains("different context handles"),
            "error should NOT report a handle mismatch for this case: {msg}"
        );
    }

    #[test]
    fn on_event_filtered_generates_decode_and_dispatch() {
        let input = r#"
            #[on_event(OnMetadataRawTreeReady)]
            fn a() {}
        "#;
        let mut mod_item = parse_mod(input);
        let parsed = parse_plugin_module(&mut mod_item).unwrap();
        let attr = PluginAttr {
            media_types: vec![],
        };
        let output = generate_plugin(attr, &format_ident!("test_mod"), parsed).unwrap();
        let s = output.to_string();

        assert!(s.contains("Event :: decode"), "should decode the payload");
        assert!(
            s.contains("EventPayload :: Data (ctx)"),
            "should unpack EventPayload::Data into ctx: {s}"
        );
        assert!(
            s.contains(
                "EventContext :: MetadataReadyContext ((ref metadata_handle , ref path_mapping_handle))"
            ),
            "should unpack the MetadataReadyContext handles by reference: {s}"
        );
        assert!(
            s.contains(
                "super :: test_mod :: a (ctx . clone () , metadata_handle , path_mapping_handle)"
            ),
            "should call handler with ctx + &handles: {s}"
        );
        assert!(
            s.contains("log :: warn !"),
            "should warn on context mismatch: {s}"
        );
        assert!(
            s.contains("log :: error !"),
            "should log decode errors: {s}"
        );
    }

    #[test]
    fn on_event_multi_variant_generates_multiple_arms() {
        let input = r#"
            #[on_event(OnScanComplete, OnUserLogin)]
            fn a() {}
        "#;
        let mut mod_item = parse_mod(input);
        let parsed = parse_plugin_module(&mut mod_item).unwrap();
        let attr = PluginAttr {
            media_types: vec![],
        };
        let output = generate_plugin(attr, &format_ident!("test_mod"), parsed).unwrap();
        let s = output.to_string();

        assert!(
            s.contains("cosmox_api :: event :: Event :: decode"),
            "should decode the payload: {s}"
        );
        assert!(
            s.contains(
                "cosmox_api :: event :: Event :: OnScanComplete (cosmox_api :: event :: EventPayload :: Data (ctx) ,)"
            ),
            "should match OnScanComplete: {s}"
        );
        assert!(
            s.contains(
                "cosmox_api :: event :: Event :: OnUserLogin (cosmox_api :: event :: EventPayload :: Data (ctx) ,)"
            ),
            "should match OnUserLogin: {s}"
        );
        assert!(
            s.contains("super :: test_mod :: a (ctx . clone ())"),
            "both arms should call the same handler with ctx: {s}"
        );
        let reg_count = s.matches("register ()").count();
        assert!(
            reg_count >= 2,
            "should register each variant (found {reg_count}): {s}"
        );
    }

    #[test]
    fn on_event_unit_variant_generates_ctx_ignore() {
        // `OnScanComplete` is a handle-less variant, so the generated arm must:
        //  - discard the fixed `event_context` parameter (otherwise the plugin
        //    would warn `unused variable: event_context`), and
        //  - call the handler with `ctx` only, with no `EventContext` variant match.
        let input = r#"
            #[on_event(OnScanComplete)]
            fn a() {}
        "#;
        let mut mod_item = parse_mod(input);
        let parsed = parse_plugin_module(&mut mod_item).unwrap();
        let attr = PluginAttr {
            media_types: vec![],
        };
        let output = generate_plugin(attr, &format_ident!("test_mod"), parsed).unwrap();
        let s = output.to_string();

        assert!(
            s.contains("let _ = event_context"),
            "handle-less arm must discard the event_context param to avoid an unused-variable warning: {s}"
        );
        assert!(
            s.contains("super :: test_mod :: a (ctx . clone ())"),
            "unit event arm should call handler with ctx only: {s}"
        );
        assert!(
            !s.contains("EventContext ::"),
            "unit event arm should not reference any EventContext variant: {s}"
        );
    }
}
