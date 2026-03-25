//! Vue SFC template generation.
//!
//! Converts a [`ComponentDef`] AST into a Vue Single-File Component containing
//! `<template>` and `<script setup lang="ts">` blocks. No `<style>` section is
//! emitted — design-system CSS lives in the global `clutter.css`.

use clutter_runtime::{ComponentDef, ComponentNode, EachNode, EventBinding, IfNode, Node, PropValue, UnsafeNode};

// ---------------------------------------------------------------------------
// Built-in component → HTML element mapping
// ---------------------------------------------------------------------------

const BUILTIN: &[(&str, &str, bool)] = &[
    // (clutter name, html tag, self-closing)
    ("Column", "div",    false),
    ("Row",    "div",    false),
    ("Box",    "div",    false),
    ("Text",   "p",      false),
    ("Button", "button", false),
    ("Input",  "input",  true),
];

fn builtin_tag(name: &str) -> Option<(&'static str, bool)> {
    BUILTIN.iter().find(|(n, _, _)| *n == name).map(|(_, tag, self_closing)| (*tag, *self_closing))
}

// ---------------------------------------------------------------------------
// Prop generation
// ---------------------------------------------------------------------------

/// Processes a list of props and returns `(class_attr, bindings, text_content)`.
///
/// - `class_attr`: space-separated CSS classes from `StringValue` props
///   (excluding the special `value` prop).
/// Serialises a list of event bindings into a space-prefixed attribute string.
///
/// Each `EventBinding { name, handler }` becomes `@{name}="{handler}"`.
/// Returns an empty string when there are no events.
fn generate_events(events: &[EventBinding]) -> String {
    if events.is_empty() {
        return String::new();
    }
    let parts: Vec<String> = events
        .iter()
        .map(|ev| format!("@{}=\"{}\"", ev.name, ev.handler))
        .collect();
    format!(" {}", parts.join(" "))
}

/// - `bindings`: Vue `:prop="expr"` bindings from `ExpressionValue` props
///   (excluding the special `value` prop on Text/Input).
/// - `text_content`: the resolved text/interpolation for the `value` prop
///   (`None` if no `value` prop).
fn generate_props(
    component: &str,
    props: &[clutter_runtime::PropNode],
) -> (String, String, Option<String>) {
    let mut classes: Vec<String> = Vec::new();
    let mut bindings: Vec<String> = Vec::new();
    let mut text_content: Option<String> = None;

    for prop in props {
        let is_value_prop = prop.name == "value" && matches!(component, "Text" | "Input");

        match &prop.value {
            PropValue::StringValue(val) => {
                if is_value_prop {
                    text_content = Some(val.clone());
                } else {
                    classes.push(format!("clutter-{}-{}", prop.name, val));
                }
            }
            PropValue::ExpressionValue(expr) => {
                if is_value_prop {
                    text_content = Some(format!("{{{{ {expr} }}}}"));
                } else {
                    bindings.push(format!(":{}=\"{}\"", prop.name, expr));
                }
            }
            PropValue::UnsafeValue { value, .. } => {
                bindings.push(format!("{}=\"{}\"", prop.name, value));
            }
        }
    }

    (classes.join(" "), bindings.join(" "), text_content)
}

// ---------------------------------------------------------------------------
// Node generation
// ---------------------------------------------------------------------------

fn indent(depth: usize) -> String {
    "  ".repeat(depth)
}

fn generate_node(node: &Node, depth: usize) -> String {
    match node {
        Node::Component(c)  => generate_component_node(c, depth),
        Node::Text(t)       => format!("{}{}\n", indent(depth), t.value),
        Node::Expr(e)       => format!("{}{{{{ {} }}}}\n", indent(depth), e.value),
        Node::If(i)         => generate_if(i, depth),
        Node::Each(e)       => generate_each(e, depth),
        Node::Unsafe(u)     => generate_unsafe(u, depth),
    }
}

fn generate_component_node(node: &ComponentNode, depth: usize) -> String {
    let ind = indent(depth);

    match builtin_tag(&node.name) {
        Some((tag, self_closing)) => {
            let base_class = format!("clutter-{}", node.name.to_lowercase());
            let (extra_classes, bindings, text_content) = generate_props(&node.name, &node.props);

            let class_attr = if extra_classes.is_empty() {
                format!("class=\"{base_class}\"")
            } else {
                format!("class=\"{base_class} {extra_classes}\"")
            };

            let bindings_str = if bindings.is_empty() {
                String::new()
            } else {
                format!(" {bindings}")
            };

            let events_str = generate_events(&node.events);

            if self_closing {
                return format!("{ind}<{tag} {class_attr}{bindings_str}{events_str} />\n");
            }

            let children_str = generate_template(&node.children, depth + 1);

            match text_content {
                Some(text) => format!("{ind}<{tag} {class_attr}{bindings_str}{events_str}>{text}</{tag}>\n"),
                None => {
                    if children_str.is_empty() {
                        format!("{ind}<{tag} {class_attr}{bindings_str}{events_str}></{tag}>\n")
                    } else {
                        format!("{ind}<{tag} {class_attr}{bindings_str}{events_str}>\n{children_str}{ind}</{tag}>\n")
                    }
                }
            }
        }
        None => {
            // Custom component: pass through as-is
            let mut attr_parts: Vec<String> = Vec::new();
            for prop in &node.props {
                match &prop.value {
                    PropValue::StringValue(v) => attr_parts.push(format!("{}=\"{}\"", prop.name, v)),
                    PropValue::ExpressionValue(e) => attr_parts.push(format!(":{}=\"{}\"", prop.name, e)),
                    PropValue::UnsafeValue { value, .. } => attr_parts.push(format!("{}=\"{}\"", prop.name, value)),
                }
            }
            let attrs = if attr_parts.is_empty() {
                String::new()
            } else {
                format!(" {}", attr_parts.join(" "))
            };
            format!("{ind}<{}{} />\n", node.name, attrs)
        }
    }
}

fn generate_if(node: &IfNode, depth: usize) -> String {
    let ind = indent(depth);

    let use_template = node.then_children.len() > 1;

    if use_template {
        let then_body = generate_template(&node.then_children, depth + 1);
        let else_part = match &node.else_children {
            Some(children) if !children.is_empty() => {
                let else_body = generate_template(children, depth + 1);
                format!("{ind}<template v-else>\n{else_body}{ind}</template>\n")
            }
            _ => String::new(),
        };
        format!(
            "{ind}<template v-if=\"{cond}\">\n{then_body}{ind}</template>\n{else_part}",
            cond = node.condition,
        )
    } else {
        // Single then-child: place v-if directly on it
        let then_str = generate_node_with_directive(
            &node.then_children[0],
            depth,
            &format!("v-if=\"{}\"", node.condition),
        );
        let else_str = match &node.else_children {
            Some(children) if !children.is_empty() => {
                generate_node_with_directive(&children[0], depth, "v-else")
            }
            _ => String::new(),
        };
        format!("{then_str}{else_str}")
    }
}

fn generate_each(node: &EachNode, depth: usize) -> String {
    let ind = indent(depth);
    let vfor = format!("v-for=\"{} in {}\" :key=\"{}\"", node.alias, node.collection, node.alias);

    let use_template = node.children.len() > 1;

    if use_template {
        let body = generate_template(&node.children, depth + 1);
        format!("{ind}<template {vfor}>\n{body}{ind}</template>\n")
    } else {
        generate_node_with_directive(&node.children[0], depth, &vfor)
    }
}

fn generate_unsafe(node: &UnsafeNode, depth: usize) -> String {
    // <unsafe> is a compiler concept only — render children directly, no wrapper.
    generate_template(&node.children, depth)
}

/// Re-renders a node but injects an extra HTML attribute into the opening tag.
///
/// Used to attach `v-if`, `v-else`, or `v-for` directly onto a single child
/// element instead of wrapping it in a `<template>`.
fn generate_node_with_directive(node: &Node, depth: usize, directive: &str) -> String {
    let rendered = generate_node(node, depth);
    // Inject the directive after the first `<tag` or `<Tag`.
    // We find the first `<` followed by a word character and insert after it.
    if let Some(pos) = rendered.find('<') {
        let after_lt = &rendered[pos + 1..];
        if let Some(space_or_end) = after_lt.find(|c: char| c == ' ' || c == '/' || c == '>') {
            let tag_end = pos + 1 + space_or_end;
            let (before, after) = rendered.split_at(tag_end);
            return format!("{before} {directive}{after}");
        }
    }
    rendered
}

// ---------------------------------------------------------------------------
// Template + SFC assembly
// ---------------------------------------------------------------------------

pub fn generate_template(nodes: &[Node], depth: usize) -> String {
    nodes.iter().map(|n| generate_node(n, depth)).collect()
}

/// Generates a complete Vue SFC string for a single [`ComponentDef`].
///
/// The SFC contains only `<template>` and `<script setup>` — no `<style>`.
/// Design-system CSS lives in the global `clutter.css` emitted by the CLI.
pub fn generate_sfc(comp: &ComponentDef) -> String {
    let template_body = generate_template(&comp.template, 0);
    format!(
        "<template>\n{template_body}</template>\n\n<script setup lang=\"ts\">\n{logic}</script>\n",
        logic = comp.logic_block,
    )
}
