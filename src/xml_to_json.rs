use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::io::Read;

use anyhow::anyhow;
use serde_json::{Map, Value};
use xmltree::{Element, XMLNode};

/// Configuration for parsing XML to JSON.
#[derive(Debug, Default)]
pub struct Config {
    /// Exclude attributes such as `schemaLocation` from outputted JSON.
    pub ignore_attributes: Vec<String>,
    /// XML such as <groups><group /></groups> is converted to { "groups": { "group": [...] } }
    /// even when there is a a single child <group /> in the XML, when `try_detect_lists: true` we
    /// use the plural form, e.g. "groups" and the singular form "group" and assume that although
    /// we have a single child, the correct conversion would be to a list.
    pub try_detect_lists: bool,
}

pub fn parse_xml<T: Read>(input: &mut T, config: &Config) -> anyhow::Result<Value> {
    let element = Element::parse(input)?;

    let (name, value) = convert_node(XMLNode::Element(element), &config)
        .ok_or_else(|| anyhow!("No XML node found"))?;

    let mut map = Map::new();
    map.insert(name, value);

    Ok(Value::Object(map))
}

// Converts an XML Node into a JSON property, all attributes and text are interpreted as strings.
fn convert_node(node: XMLNode, config: &Config) -> Option<(String, Value)> {
    match node {
        XMLNode::Element(el) => {
            // We explicitly ignore <child> in the case of:
            // <parent>
            //   <child>
            //     ..
            //   </child>
            //   Text
            // </parent>
            if el.attributes.is_empty() {
                if let Some(XMLNode::Text(text)) =
                    el.children.iter().find(|child| child.as_text().is_some())
                {
                    return Some((el.name, Value::String(text.clone())));
                }
            }

            let map = el
                .attributes
                .into_iter()
                .filter(|(k, _)| !config.ignore_attributes.contains(k))
                .map(|(k, v)| (k, Value::String(v)))
                .chain(convert_children(&el.name, el.children, config))
                .collect();

            Some((el.name, Value::Object(map)))
        }
        XMLNode::Text(text) => Some(("value".to_owned(), Value::String(text))),
        _ => None, // All other node types are ignored.
    }
}

fn convert_children(
    parent_name: &str,
    children: Vec<XMLNode>,
    config: &Config,
) -> impl Iterator<Item = (String, Value)> {
    let mut map: HashMap<String, Value> = HashMap::new();

    for child in children {
        if let Some((name, value)) = convert_node(child, config) {
            let maybe_list_element = parent_name.ends_with('s')
                && &parent_name[..parent_name.len() - 1] == name.as_str();

            match map.entry(name) {
                Entry::Occupied(mut e) => {
                    let current_value = e.get_mut();
                    match current_value {
                        Value::String(_) | Value::Object(_) => {
                            // Convert value to list if we find multiple children with the same
                            // name.
                            let single_value = std::mem::replace(current_value, Value::Null);
                            let list = Value::Array(vec![single_value, value]);
                            *current_value = list;
                        }
                        Value::Array(ref mut list) => {
                            list.push(value);
                        }
                        _ => unreachable!(),
                    }
                }
                Entry::Vacant(e) => {
                    if config.try_detect_lists && maybe_list_element {
                        e.insert(Value::Array(vec![value]));
                    } else {
                        e.insert(value);
                    }
                }
            }
        }
    }

    map.into_iter()
}
