use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Attribute, Ident, ItemMod, Visibility,
    parse::{Parse, ParseStream},
};

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
    OnEvent(Ident),
    OnCommand(Ident),
    OnConfig(Ident),
    Health(Ident),
    MediaTypes(Ident),
}

fn extract_annotation_name(attrs: &mut Vec<Attribute>) -> Option<String> {
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
    Some(attr.path().segments.last().unwrap().ident.to_string())
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
    let mut seen_on_event = false;
    let mut seen_on_command = false;
    let mut seen_on_config = false;
    let mut seen_health = false;
    let mut seen_media_types = false;

    if let Some((_, items)) = &mut mod_item.content {
        for item in items.iter_mut() {
            if let syn::Item::Fn(func) = item
                && let Some(name) = extract_annotation_name(&mut func.attrs)
            {
                let fn_name = func.sig.ident.clone();
                let span = func.sig.ident.span();

                let already_seen = match name.as_str() {
                    "on_load" => Some((&mut seen_on_load, "`#[on_load]`")),
                    "on_unload" => Some((&mut seen_on_unload, "`#[on_unload]`")),
                    "on_enable" => Some((&mut seen_on_enable, "`#[on_enable]`")),
                    "on_disable" => Some((&mut seen_on_disable, "`#[on_disable]`")),
                    "on_event" => Some((&mut seen_on_event, "`#[on_event]`")),
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
                    "on_event" => Some(AnnotatedFn::OnEvent(fn_name)),
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
    let on_event = parsed.annotated.iter().find_map(|a| match a {
        AnnotatedFn::OnEvent(n) => Some(n.clone()),
        _ => None,
    });
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

    let on_load_body = match on_load {
        Some(ref fn_name) => quote! { #mod_path::#fn_name(config) },
        None => quote! {
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

    let on_event_body = match on_event {
        Some(ref fn_name) => quote! { #mod_path::#fn_name(event, event_context) },
        None => quote! {
            cosmox_api::api::bindings::exports::cosmox::plugin::host_notifier::PluginResult::Ok
        },
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
            extract_annotation_name(&mut func.attrs),
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
        assert_eq!(extract_annotation_name(&mut func.attrs), None);
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
                AnnotatedFn::OnEvent(format_ident!("event_fn")),
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
                OnEvent(format_ident!("e")),
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
}
