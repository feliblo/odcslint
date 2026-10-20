use serde::Deserialize;

use super::node::{as_bool, as_i64, as_seq, as_str, get, scalar_text};
use super::Node;
use crate::constants::keys;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Contract,
    Schema,
    Property,
}

#[derive(Debug, Clone, Copy)]
pub struct Contract<'a> {
    root: &'a Node<'a>,
}

impl<'a> Contract<'a> {
    pub const fn new(root: &'a Node<'a>) -> Self {
        Self { root }
    }

    pub const fn root(&self) -> &'a Node<'a> {
        self.root
    }

    pub fn field(&self, key: &str) -> Option<&'a Node<'a>> {
        get(self.root, key)
    }

    pub fn team_name(&self) -> Option<&'a Node<'a>> {
        self.field(keys::TEAM)
            .and_then(|team| get(team, keys::NAME))
    }

    /// `team` is a list before ODCS v3.1 and an object with `members` from v3.1.
    pub fn team_members(&self) -> Vec<&'a Node<'a>> {
        let members = self
            .field(keys::TEAM)
            .and_then(|team| as_seq(team).or_else(|| get(team, keys::MEMBERS).and_then(as_seq)));
        members
            .map(|items| items.iter().collect())
            .unwrap_or_default()
    }

    pub fn custom_property_values(&self, key: &str) -> Vec<&'a Node<'a>> {
        self.field(keys::CUSTOM_PROPERTIES)
            .and_then(as_seq)
            .map(|items| {
                items
                    .iter()
                    .filter(|item| {
                        get(item, keys::PROPERTY).and_then(scalar_text).as_deref() == Some(key)
                    })
                    .filter_map(|item| get(item, keys::VALUE))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn schema_objects(&self) -> Vec<SchemaObject<'a>> {
        self.field(keys::SCHEMA)
            .and_then(as_seq)
            .map(|items| {
                items
                    .iter()
                    .enumerate()
                    .map(|(index, node)| SchemaObject { node, index })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn property_lists(&self, recursive: bool) -> Vec<PropertyList<'a>> {
        self.schema_objects()
            .iter()
            .filter_map(SchemaObject::properties)
            .flat_map(|list| {
                if recursive {
                    list.flatten()
                } else {
                    vec![list]
                }
            })
            .collect()
    }

    pub fn properties(&self) -> Vec<Property<'a>> {
        self.property_lists(true)
            .iter()
            .flat_map(PropertyList::properties)
            .collect()
    }

    pub fn nodes_in(&self, scope: Scope) -> Vec<&'a Node<'a>> {
        match scope {
            Scope::Contract => vec![self.root],
            Scope::Schema => self
                .schema_objects()
                .iter()
                .map(|object| object.node)
                .collect(),
            Scope::Property => self
                .properties()
                .iter()
                .map(|property| property.node)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SchemaObject<'a> {
    pub node: &'a Node<'a>,
    pub index: usize,
}

impl<'a> SchemaObject<'a> {
    pub fn name(&self) -> Option<&'a str> {
        get(self.node, keys::NAME).and_then(as_str)
    }

    pub fn properties(&self) -> Option<PropertyList<'a>> {
        let node = get(self.node, keys::PROPERTIES)?;
        as_seq(node)?;
        Some(PropertyList {
            node,
            owner: self.name(),
            pointer: format!("/{}/{}/{}", keys::SCHEMA, self.index, keys::PROPERTIES),
        })
    }
}

#[derive(Debug, Clone)]
pub struct PropertyList<'a> {
    pub node: &'a Node<'a>,
    pub owner: Option<&'a str>,
    pub pointer: String,
}

impl<'a> PropertyList<'a> {
    pub fn properties(&self) -> Vec<Property<'a>> {
        as_seq(self.node)
            .map(|items| items.iter().map(|node| Property { node }).collect())
            .unwrap_or_default()
    }

    pub fn children(&self) -> Vec<Self> {
        self.properties()
            .iter()
            .enumerate()
            .filter_map(|(index, property)| {
                let item_pointer = format!("{}/{index}", self.pointer);
                get(property.node, keys::PROPERTIES)
                    .map(|node| (node, format!("{item_pointer}/{}", keys::PROPERTIES)))
                    .or_else(|| {
                        get(property.node, keys::ITEMS)
                            .and_then(|items| get(items, keys::PROPERTIES))
                            .map(|node| {
                                (
                                    node,
                                    format!("{item_pointer}/{}/{}", keys::ITEMS, keys::PROPERTIES),
                                )
                            })
                    })
                    .filter(|(node, _)| as_seq(node).is_some())
                    .map(|(node, pointer)| Self {
                        node,
                        owner: property.name(),
                        pointer,
                    })
            })
            .collect()
    }

    pub fn flatten(self) -> Vec<Self> {
        let children = self.children();
        std::iter::once(self)
            .chain(children.into_iter().flat_map(Self::flatten))
            .collect()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Property<'a> {
    pub node: &'a Node<'a>,
}

impl<'a> Property<'a> {
    pub fn name(&self) -> Option<&'a str> {
        get(self.node, keys::NAME).and_then(as_str)
    }

    pub fn is_primary_key(&self) -> bool {
        get(self.node, keys::PRIMARY_KEY)
            .and_then(as_bool)
            .unwrap_or(false)
    }

    /// `-1` is the schema default for "not a primary key".
    pub fn primary_key_position(&self) -> Option<i64> {
        get(self.node, keys::PRIMARY_KEY_POSITION)
            .and_then(as_i64)
            .filter(|position| *position >= 1)
    }

    pub fn has_primary_key_position(&self) -> bool {
        get(self.node, keys::PRIMARY_KEY_POSITION)
            .and_then(as_i64)
            .is_some_and(|position| position != -1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;

    const CONTRACT: &str = "\
team:
  name: finance
  members:
    - username: alice
      role: owner
    - username: bob
customProperties:
  - property: owner
    value: carol
schema:
  - name: orders
    properties:
      - name: id
        primaryKey: true
        primaryKeyPosition: 1
      - name: address
        logicalType: object
        properties:
          - name: street
      - name: lines
        logicalType: array
        items:
          logicalType: object
          properties:
            - name: sku
";

    #[test]
    fn reads_team_members_from_object_form() {
        let document = Document::parse(CONTRACT).unwrap();
        let contract = Contract::new(document.root());
        assert_eq!(contract.team_members().len(), 2);
        assert_eq!(contract.team_name().and_then(as_str), Some("finance"));
    }

    #[test]
    fn reads_team_members_from_deprecated_list_form() {
        let document = Document::parse("team:\n  - username: alice\n").unwrap();
        let contract = Contract::new(document.root());
        assert_eq!(contract.team_members().len(), 1);
        assert!(contract.team_name().is_none());
    }

    #[test]
    fn reads_custom_property_values() {
        let document = Document::parse(CONTRACT).unwrap();
        let contract = Contract::new(document.root());
        let values = contract.custom_property_values("owner");
        assert_eq!(
            values.iter().map(|v| as_str(v)).collect::<Vec<_>>(),
            vec![Some("carol")]
        );
    }

    #[test]
    fn flattens_nested_property_lists_with_pointers() {
        let document = Document::parse(CONTRACT).unwrap();
        let contract = Contract::new(document.root());
        let pointers: Vec<String> = contract
            .property_lists(true)
            .into_iter()
            .map(|list| list.pointer)
            .collect();
        assert_eq!(
            pointers,
            vec![
                "/schema/0/properties",
                "/schema/0/properties/1/properties",
                "/schema/0/properties/2/items/properties",
            ]
        );
    }

    #[test]
    fn non_recursive_lists_only_top_level() {
        let document = Document::parse(CONTRACT).unwrap();
        let contract = Contract::new(document.root());
        assert_eq!(contract.property_lists(false).len(), 1);
    }

    #[test]
    fn collects_all_properties() {
        let document = Document::parse(CONTRACT).unwrap();
        let contract = Contract::new(document.root());
        let names: Vec<_> = contract.properties().iter().map(Property::name).collect();
        assert_eq!(
            names,
            vec![
                Some("id"),
                Some("address"),
                Some("lines"),
                Some("street"),
                Some("sku")
            ]
        );
    }

    #[test]
    fn reads_primary_key_fields() {
        let document = Document::parse(CONTRACT).unwrap();
        let contract = Contract::new(document.root());
        let id = contract.properties()[0];
        assert!(id.is_primary_key());
        assert_eq!(id.primary_key_position(), Some(1));
    }

    #[test]
    fn nodes_in_scope() {
        let document = Document::parse(CONTRACT).unwrap();
        let contract = Contract::new(document.root());
        assert_eq!(contract.nodes_in(Scope::Contract).len(), 1);
        assert_eq!(contract.nodes_in(Scope::Schema).len(), 1);
        assert_eq!(contract.nodes_in(Scope::Property).len(), 5);
    }
}
