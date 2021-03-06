pub mod biblio;
pub mod dfn;
pub mod reference;

use kuchiki::NodeRef;
use std::collections::HashMap;

use crate::config::DFN_SELECTOR;
use crate::html::{self, Attr};
use crate::spec::Spec;

pub fn process_auto_links(doc: &mut Spec) {
    for auto_link_el in html::select(doc.dom(), "a:not([href]):not([data-link-type='biblio'])") {
        let link_type = determine_link_type(&auto_link_el);
        html::insert_attr(&auto_link_el, "data-link-type", &link_type);

        let link_text = html::get_text_content(&auto_link_el);

        let reference = doc.reference_manager.get_reference(&link_type, &link_text);

        if let Some(ref reference_spec) = reference.spec {
            if let Some(ref doc_spec) = doc.reference_manager.spec {
                if reference_spec.to_lowercase() != doc_spec.to_lowercase() {
                    doc.external_references_used
                        .entry(reference_spec.to_owned())
                        .or_default()
                        .insert(link_text, reference.to_owned());
                }
            }

            if let Some(biblio_entry) = doc.biblio_entry_manager.get_biblio_entry(&reference_spec) {
                doc.normative_biblio_entries
                    .insert(biblio_entry.link_text.to_owned(), biblio_entry);
            }
        }

        // Decorate auto-link.
        html::insert_attr(&auto_link_el, "href", &reference.url);
        let name = reference.url.rsplitn(2, '#').next().unwrap();
        html::insert_attr(&auto_link_el, "id", format!("ref-for-{}", name));
    }

    html::dedup_ids(doc.dom());
}

fn determine_link_type(link_el: &NodeRef) -> String {
    match html::get_attr(link_el, "data-link-type") {
        Some(link_type) => link_type,
        None => "dfn".to_owned(),
    }
}

pub fn add_self_links(doc: &mut Spec) {
    let dfn_els = html::select(doc.dom(), &DFN_SELECTOR).collect::<Vec<NodeRef>>();

    let mut found_first_numbered_section = false;

    for heading_el in html::select(doc.dom(), "h2, h3, h4, h5, h6") {
        found_first_numbered_section |= html::get_attr(&heading_el, "data-level").is_some();

        if dfn_els.contains(&heading_el) || !found_first_numbered_section {
            continue;
        }

        // Append self-link.
        if let Some(id) = html::get_attr(&heading_el, "id") {
            let a_el = html::new_a(
                btreemap! {
                    "class" => "self-link".to_owned(),
                    "href" => format!("#{}", id),
                },
                "",
            );
            heading_el.append(a_el);
        }
    }

    add_dfn_panels(doc, &dfn_els);
}

fn add_dfn_panels(doc: &mut Spec, dfn_els: &[NodeRef]) {
    // id => <a> elements with this id
    let mut all_links_els: HashMap<String, Vec<NodeRef>> = HashMap::new();

    for a_el in html::select(doc.dom(), "a") {
        let href = match html::get_attr(&a_el, "href") {
            Some(href) => href,
            None => continue,
        };

        if !href.starts_with('#') {
            continue;
        }

        all_links_els
            .entry(href[1..].to_owned())
            .or_default()
            .push(a_el.to_owned());
    }

    let mut at_least_one_panel = false;

    for dfn_el in dfn_els {
        let id = match html::get_attr(dfn_el, "id") {
            Some(id) => id,
            None => continue,
        };

        // section name => <a> elements
        let mut section_els: HashMap<String, Vec<NodeRef>> = HashMap::new();

        if let Some(links_els) = all_links_els.get(&id) {
            for link_el in links_els {
                if let Some(section) = html::get_section(link_el) {
                    section_els
                        .entry(section)
                        .or_default()
                        .push(link_el.to_owned());
                }
            }
        }

        if section_els.is_empty() {
            // Insert a self-link.
            let a_el = html::new_a(
                btreemap! {
                    "class" => "self-link".to_owned(),
                    "href" => format!("#{}", id),
                },
                "",
            );
            dfn_el.append(a_el);

            continue;
        }

        at_least_one_panel = true;

        html::add_class(dfn_el, "dfn-paneled");

        let aside_el = html::new_element(
            "aside",
            btreemap! {
                "class" => "dfn-panel",
                "data-for" => &id,
            },
        );

        aside_el.append({
            let b_el = html::new_element("b", None::<Attr>);
            b_el.append(html::new_a(
                btreemap! {
                    "href" => format!("#{}", id)
                },
                format!("#{}", id),
            ));
            b_el
        });

        aside_el.append({
            let b_el = html::new_element("b", None::<Attr>);
            b_el.append(html::new_text("Referenced in:"));
            b_el
        });

        let ul_el = html::new_element("ul", None::<Attr>);

        for (section, section_els) in section_els {
            let li_el = html::new_element("li", None::<Attr>);

            for section_el in section_els {
                let section_id = match html::get_attr(&section_el, "id") {
                    Some(section_id) => section_id,
                    None => format!("ref-for-{}", id),
                };

                let a_el = html::new_a(
                    btreemap! {
                        "href" => format!("#{}", section_id)
                    },
                    &section,
                );
                li_el.append(a_el);
            }

            ul_el.append(li_el);
        }

        aside_el.append(ul_el);

        doc.body().append(aside_el);
    }

    if at_least_one_panel {
        doc.extra_styles
            .insert("dfn-panel", include_str!("../style/dfn-panel.css"));
        doc.extra_scripts
            .insert("dfn-panel", include_str!("../script/dfn-panel.js"));
    }
}
