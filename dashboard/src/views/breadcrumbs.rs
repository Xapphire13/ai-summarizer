use maud::{Markup, html};

use crate::views::ClassName;

pub struct Breadcrumb<'a> {
    pub label: &'a str,
    pub href: Option<&'a str>,
}

pub fn breadcrumbs(items: &[Breadcrumb]) -> Markup {
    html! {
        nav.(ClassName::BREADCRUMBS) {
            ol.(ClassName::BREADCRUMBS_LIST) {
                @for item in items {
                    li.(ClassName::BREADCRUMB) {
                        @if let Some(href) = item.href {
                            a href=(href) { (item.label) }
                        } @else {
                            (item.label)
                        }
                    }
                }
            }
        }
    }
}
