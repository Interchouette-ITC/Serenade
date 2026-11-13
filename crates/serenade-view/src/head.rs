//! Leptos head lifecycle helpers for client-rendered and hydrated documents.

#![allow(clippy::empty_enums)] // Leptos generates empty typed-builder error enums for optional props.

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use leptos::prelude::*;

#[derive(Clone)]
struct HeadRegistry {
    entries: RwSignal<Vec<HeadEntry>>,
    next_id: Arc<AtomicU64>,
}

impl HeadRegistry {
    fn new() -> Self {
        Self {
            entries: RwSignal::new(Vec::new()),
            next_id: Arc::new(AtomicU64::new(1)),
        }
    }

    fn register(&self, key: Option<String>, content: HeadEntryContent) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.entries
            .update(|entries| entries.push(HeadEntry { id, key, content }));
        id
    }

    fn remove(&self, id: u64) {
        self.entries
            .update(|entries| entries.retain(|entry| entry.id != id));
    }

    fn visible_entries(&self) -> Vec<HeadEntry> {
        let entries = self.entries.get();
        entries
            .iter()
            .enumerate()
            .filter(|(index, entry)| {
                entry.key.as_ref().is_none_or(|key| {
                    !entries[index + 1..]
                        .iter()
                        .any(|later| later.key.as_ref() == Some(key))
                })
            })
            .map(|(_, entry)| entry.clone())
            .collect()
    }
}

#[derive(Clone)]
struct HeadEntry {
    id: u64,
    key: Option<String>,
    content: HeadEntryContent,
}

#[derive(Clone)]
enum HeadEntryContent {
    Title {
        text: TextProp,
    },
    Meta {
        charset: Option<TextProp>,
        name: Option<TextProp>,
        property: Option<TextProp>,
        http_equiv: Option<TextProp>,
        itemprop: Option<TextProp>,
        content: Option<TextProp>,
    },
    Link {
        id: Option<TextProp>,
        as_: Option<TextProp>,
        crossorigin: Option<TextProp>,
        fetchpriority: Option<TextProp>,
        href: Option<TextProp>,
        hreflang: Option<TextProp>,
        imagesizes: Option<TextProp>,
        imagesrcset: Option<TextProp>,
        integrity: Option<TextProp>,
        media: Option<TextProp>,
        referrerpolicy: Option<TextProp>,
        rel: Option<TextProp>,
        sizes: Option<TextProp>,
        title: Option<TextProp>,
        type_: Option<TextProp>,
        blocking: Option<TextProp>,
    },
}

/// Provides a Leptos metadata context and a lifecycle-managed head insertion point.
///
/// Place this component inside the application's `<head>` and supply the active
/// head view as children. Entries with the same stable key replace earlier
/// entries while mounted; unkeyed repeatable entries remain.
#[component]
pub fn ManagedHead(children: Children) -> impl IntoView {
    leptos_meta::provide_meta_context();
    let registry = HeadRegistry::new();
    provide_context(registry.clone());
    let children = children();

    view! {
        // Render nested branches before enumerating entries so SSR includes their registrations.
        {children}
        <leptos_meta::MetaTags />
        <For
            each=move || registry.visible_entries()
            key=|entry| entry.id
            let:entry
        >
            <HeadEntryView entry=entry />
        </For>
    }
}

/// Registers a reactive metadata element and replaces an earlier entry with the same key.
#[component]
pub fn ManagedMeta(
    #[prop(optional, into)] head_key: Option<String>,
    #[prop(optional, into)] charset: Option<TextProp>,
    #[prop(optional, into)] name: Option<TextProp>,
    #[prop(optional, into)] property: Option<TextProp>,
    #[prop(optional, into)] http_equiv: Option<TextProp>,
    #[prop(optional, into)] itemprop: Option<TextProp>,
    #[prop(optional, into)] content: Option<TextProp>,
) -> impl IntoView {
    let entry = HeadEntryContent::Meta {
        charset,
        name,
        property,
        http_equiv,
        itemprop,
        content,
    };
    if let Some(registry) = use_context::<HeadRegistry>() {
        let id = registry.register(head_key, entry);
        on_cleanup(move || registry.remove(id));
        ().into_any()
    } else {
        head_entry_view(HeadEntry::from(entry))
    }
}

/// Registers a reactive link element and replaces an earlier entry with the same key.
#[component]
pub fn ManagedLink(
    #[prop(optional, into)] head_key: Option<String>,
    #[prop(optional, into)] id: Option<TextProp>,
    #[prop(optional, into)] as_: Option<TextProp>,
    #[prop(optional, into)] crossorigin: Option<TextProp>,
    #[prop(optional, into)] fetchpriority: Option<TextProp>,
    #[prop(optional, into)] href: Option<TextProp>,
    #[prop(optional, into)] hreflang: Option<TextProp>,
    #[prop(optional, into)] imagesizes: Option<TextProp>,
    #[prop(optional, into)] imagesrcset: Option<TextProp>,
    #[prop(optional, into)] integrity: Option<TextProp>,
    #[prop(optional, into)] media: Option<TextProp>,
    #[prop(optional, into)] referrerpolicy: Option<TextProp>,
    #[prop(optional, into)] rel: Option<TextProp>,
    #[prop(optional, into)] sizes: Option<TextProp>,
    #[prop(optional, into)] title: Option<TextProp>,
    #[prop(optional, into)] type_: Option<TextProp>,
    #[prop(optional, into)] blocking: Option<TextProp>,
) -> impl IntoView {
    let entry = HeadEntryContent::Link {
        id,
        as_,
        crossorigin,
        fetchpriority,
        href,
        hreflang,
        imagesizes,
        imagesrcset,
        integrity,
        media,
        referrerpolicy,
        rel,
        sizes,
        title,
        type_,
        blocking,
    };
    if let Some(registry) = use_context::<HeadRegistry>() {
        let id = registry.register(head_key, entry);
        on_cleanup(move || registry.remove(id));
        ().into_any()
    } else {
        head_entry_view(HeadEntry::from(entry))
    }
}

impl From<HeadEntryContent> for HeadEntry {
    fn from(content: HeadEntryContent) -> Self {
        Self {
            id: 0,
            key: None,
            content,
        }
    }
}

#[component]
fn HeadEntryView(entry: HeadEntry) -> impl IntoView {
    head_entry_view(entry)
}

fn head_entry_view(entry: HeadEntry) -> AnyView {
    match entry.content {
        HeadEntryContent::Title { text } => managed_title_view(text),
        HeadEntryContent::Meta {
            charset,
            name,
            property,
            http_equiv,
            itemprop,
            content,
        } => view! {
            <meta
                data-serenade-head=server_head_marker()
                charset=move || charset.as_ref().map(TextProp::get)
                name=move || name.as_ref().map(TextProp::get)
                http-equiv=move || http_equiv.as_ref().map(TextProp::get)
                itemprop=move || itemprop.as_ref().map(TextProp::get)
                content=move || content.as_ref().map(TextProp::get)
                {..leptos::tachys::html::attribute::custom::custom_attribute(
                    "property",
                    move || property.as_ref().map(TextProp::get)
                )}
            />
        }
        .into_any(),
        HeadEntryContent::Link {
            id,
            as_,
            crossorigin,
            fetchpriority,
            href,
            hreflang,
            imagesizes,
            imagesrcset,
            integrity,
            media,
            referrerpolicy,
            rel,
            sizes,
            title,
            type_,
            blocking,
        } => view! {
            <link
                data-serenade-head=server_head_marker()
                id=move || id.as_ref().map(TextProp::get)
                crossorigin=move || crossorigin.as_ref().map(TextProp::get)
                fetchpriority=move || fetchpriority.as_ref().map(TextProp::get)
                href=move || href.as_ref().map(TextProp::get)
                hreflang=move || hreflang.as_ref().map(TextProp::get)
                imagesizes=move || imagesizes.as_ref().map(TextProp::get)
                imagesrcset=move || imagesrcset.as_ref().map(TextProp::get)
                integrity=move || integrity.as_ref().map(TextProp::get)
                media=move || media.as_ref().map(TextProp::get)
                referrerpolicy=move || referrerpolicy.as_ref().map(TextProp::get)
                rel=move || rel.as_ref().map(TextProp::get)
                sizes=move || sizes.as_ref().map(TextProp::get)
                title=move || title.as_ref().map(TextProp::get)
                type=move || type_.as_ref().map(TextProp::get)
                blocking=move || blocking.as_ref().map(TextProp::get)
                {..leptos::tachys::html::attribute::custom::custom_attribute(
                    "as",
                    move || as_.as_ref().map(TextProp::get)
                )}
            />
        }
        .into_any(),
    }
}

fn server_head_marker() -> Option<&'static str> {
    #[cfg(feature = "leptos-ssr")]
    if use_context::<leptos_meta::ServerMetaContext>().is_some() {
        return Some("ssr");
    }
    None
}

/// Uses Leptos' title stack, escaping server-rendered text before metadata injection.
#[component]
pub fn ManagedTitle(
    #[prop(optional, into)] head_key: Option<String>,
    #[prop(into)] text: TextProp,
) -> impl IntoView {
    let entry = HeadEntryContent::Title { text };
    if let Some(registry) = use_context::<HeadRegistry>() {
        let id = registry.register(head_key, entry);
        on_cleanup(move || registry.remove(id));
        ().into_any()
    } else {
        head_entry_view(HeadEntry::from(entry))
    }
}

fn managed_title_view(text: TextProp) -> AnyView {
    #[cfg(feature = "leptos-ssr")]
    if use_context::<leptos_meta::ServerMetaContext>().is_some() {
        let title = text.get().to_string();
        let escaped = crate::escape_html(&title).into_owned();
        return view! { <leptos_meta::Title text=escaped /> }.into_any();
    }

    view! { <leptos_meta::Title text=text /> }.into_any()
}

/// Emits JSON-LD after JSON serialization and HTML raw-text-safe escaping.
#[component]
pub fn JsonLd(#[prop(into)] json: TextProp) -> impl IntoView {
    let json = move || {
        let value: serde_json::Value = serde_json::from_str(&json.get())
            .unwrap_or_else(|error| panic!("Rangular JSON-LD must be valid JSON: {error}"));
        let serialized = serde_json::to_string(&value)
            .expect("serializing a JSON value to a string cannot fail");
        serialized
            .replace('&', "\\u0026")
            .replace('<', "\\u003c")
            .replace('>', "\\u003e")
            .replace('\u{2028}', "\\u2028")
            .replace('\u{2029}', "\\u2029")
    };

    view! {
        <script type="application/ld+json" data-serenade-head=server_head_marker()>{json}</script>
    }
}

#[cfg(test)]
mod tests {
    use super::{HeadEntryContent, HeadRegistry};

    fn title_entry(text: &'static str) -> HeadEntryContent {
        HeadEntryContent::Title { text: text.into() }
    }

    fn meta_entry(content: &'static str) -> HeadEntryContent {
        HeadEntryContent::Meta {
            charset: None,
            name: Some("description".into()),
            property: None,
            http_equiv: None,
            itemprop: None,
            content: Some(content.into()),
        }
    }

    #[test]
    fn removing_route_metadata_restores_the_shared_singleton() {
        let registry = HeadRegistry::new();
        let shared = registry.register(
            Some("meta:name:description".to_owned()),
            meta_entry("Shared"),
        );
        let route = registry.register(
            Some("meta:name:description".to_owned()),
            meta_entry("Route"),
        );

        assert_eq!(
            registry
                .visible_entries()
                .iter()
                .map(|entry| entry.id)
                .collect::<Vec<_>>(),
            [route]
        );

        registry.remove(route);

        assert_eq!(
            registry
                .visible_entries()
                .iter()
                .map(|entry| entry.id)
                .collect::<Vec<_>>(),
            [shared]
        );
    }

    #[test]
    fn repeatable_metadata_entries_are_not_coalesced() {
        let registry = HeadRegistry::new();
        let first = registry.register(None, meta_entry("First"));
        let second = registry.register(None, meta_entry("Second"));

        assert_eq!(
            registry
                .visible_entries()
                .iter()
                .map(|entry| entry.id)
                .collect::<Vec<_>>(),
            [first, second]
        );
    }

    #[test]
    fn removing_route_title_restores_the_shared_singleton() {
        let registry = HeadRegistry::new();
        let shared = registry.register(Some("title".to_owned()), title_entry("Shared"));
        let route = registry.register(Some("title".to_owned()), title_entry("Route"));

        assert_eq!(
            registry
                .visible_entries()
                .iter()
                .map(|entry| entry.id)
                .collect::<Vec<_>>(),
            [route]
        );

        registry.remove(route);

        assert_eq!(
            registry
                .visible_entries()
                .iter()
                .map(|entry| entry.id)
                .collect::<Vec<_>>(),
            [shared]
        );
    }
}
