#![allow(dead_code)]

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct SymbolContext {
    pub(super) package: Option<String>,
    pub(super) module: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum UserSymbolKind<'a> {
    Function { name: &'a str },
    Method { receiver: &'a str, method: &'a str },
    StaticMethod { owner: &'a str, method: &'a str },
    GenericInstance { base: &'a str, args: &'a [String] },
}

pub(super) fn user_function(name: &str) -> String {
    legacy_ax_symbol(name)
}

pub(super) fn user_method(receiver: &str, method: &str) -> String {
    legacy_ax_symbol(&format!("{receiver}.{method}"))
}

pub(super) fn static_method(owner: &str, method: &str) -> String {
    legacy_ax_symbol(&format!("{owner}.{method}"))
}

pub(super) fn generic_instance(base: &str, args: &[String]) -> String {
    if args.is_empty() {
        return legacy_ax_symbol(base);
    }
    legacy_ax_symbol(&format!("{base}<{}>", args.join(", ")))
}

pub(super) fn runtime_helper(name: &str) -> String {
    sanitize_prefixed(name, "")
}

pub(super) fn user_symbol(_context: &SymbolContext, kind: UserSymbolKind<'_>) -> String {
    match kind {
        UserSymbolKind::Function { name } => user_function(name),
        UserSymbolKind::Method { receiver, method } => user_method(receiver, method),
        UserSymbolKind::StaticMethod { owner, method } => static_method(owner, method),
        UserSymbolKind::GenericInstance { base, args } => generic_instance(base, args),
    }
}

pub(super) fn package_aware_user_symbol(
    context: &SymbolContext,
    kind: UserSymbolKind<'_>,
) -> String {
    let legacy = user_symbol(context, kind);
    if context.package.is_none() && context.module.is_none() {
        return legacy;
    }

    let mut parts = Vec::new();
    if let Some(package) = &context.package {
        parts.push(sanitize_component(package));
    }
    if let Some(module) = &context.module {
        parts.push(sanitize_component(module));
    }
    parts.push(legacy);
    parts.join("__")
}

pub(super) fn sanitize_component(value: &str) -> String {
    let mut sanitized = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            sanitized.push(ch);
        } else {
            sanitized.push('_');
        }
    }
    if sanitized.is_empty() {
        "_".to_string()
    } else {
        sanitized
    }
}

fn legacy_ax_symbol(name: &str) -> String {
    if name == "main" {
        return "main".to_string();
    }
    sanitize_prefixed(name, "ax_")
}

fn sanitize_prefixed(name: &str, prefix: &str) -> String {
    let mut symbol = String::from(prefix);
    symbol.push_str(&sanitize_component(name));
    symbol
}
