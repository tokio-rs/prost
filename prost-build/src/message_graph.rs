use std::collections::{HashMap, HashSet};

use petgraph::graph::NodeIndex;
use petgraph::visit::{EdgeRef, VisitMap};
use petgraph::{Direction, Graph};

use prost_types::{
    field_descriptor_proto::{Label, Type},
    DescriptorProto, FieldDescriptorProto, FileDescriptorProto,
};

use crate::path::PathMap;

/// `MessageGraph` builds a graph of messages whose edges correspond to nesting.
/// The goal is to recognize when message types are recursively nested, so
/// that fields can be boxed when necessary.
pub struct MessageGraph {
    /// Map<fq type name, graph node index>
    index: HashMap<String, NodeIndex>,
    /// Graph with fq type name as node, field name as edge
    graph: Graph<String, String>,
    /// Map<fq type name, DescriptorProto>
    messages: HashMap<String, DescriptorProto>,
    /// Manually boxed fields
    boxed: PathMap<()>,
}

impl MessageGraph {
    pub(crate) fn new<'a>(
        files: impl Iterator<Item = &'a FileDescriptorProto>,
        boxed: PathMap<()>,
    ) -> MessageGraph {
        let mut msg_graph = MessageGraph {
            index: HashMap::new(),
            graph: Graph::new(),
            messages: HashMap::new(),
            boxed,
        };

        for file in files {
            let package = format!(
                "{}{}",
                if file.package.is_some() { "." } else { "" },
                file.package.as_ref().map(String::as_str).unwrap_or("")
            );
            for msg in &file.message_type {
                msg_graph.add_message(&package, msg);
            }
        }

        msg_graph
    }

    fn get_or_insert_index(&mut self, msg_name: String) -> NodeIndex {
        let MessageGraph {
            ref mut index,
            ref mut graph,
            ..
        } = *self;
        assert_eq!(b'.', msg_name.as_bytes()[0]);
        *index
            .entry(msg_name.clone())
            .or_insert_with(|| graph.add_node(msg_name))
    }

    /// Adds message to graph IFF it contains a non-repeated field containing another message.
    /// The purpose of the message graph is detecting recursively nested messages and co-recursively nested messages.
    /// Because prost does not box message fields, recursively nested messages would not compile in Rust.
    /// To allow recursive messages, the message graph is used to detect recursion and automatically box the recursive field.
    /// Since repeated messages are already put in a Vec, boxing them isn’t necessary even if the reference is recursive.
    fn add_message(&mut self, package: &str, msg: &DescriptorProto) {
        let msg_name = format!("{}.{}", package, msg.name.as_ref().unwrap());
        let msg_index = self.get_or_insert_index(msg_name.clone());

        for field in &msg.field {
            if field.r#type() == Type::Message && field.label() != Label::Repeated {
                let field_index = self.get_or_insert_index(field.type_name.clone().unwrap());
                self.graph
                    .add_edge(msg_index, field_index, field.name.clone().unwrap());
            }
        }
        self.messages.insert(msg_name.clone(), msg.clone());

        for msg in &msg.nested_type {
            self.add_message(&msg_name, msg);
        }
    }

    /// Try get a message descriptor from current message graph
    pub fn get_message(&self, message: &str) -> Option<&DescriptorProto> {
        self.messages.get(message)
    }

    /// Returns true if message type `inner` is nested in message type `outer`,
    /// and no field edge in the chain of dependencies is manually boxed.
    pub fn is_directly_nested(&self, outer: &str, inner: &str) -> bool {
        let outer = match self.index.get(outer) {
            Some(outer) => *outer,
            None => return false,
        };
        let inner = match self.index.get(inner) {
            Some(inner) => *inner,
            None => return false,
        };

        // Check if `inner` is nested in `outer` and ensure that all edge fields are not boxed manually.
        is_connected_with_edge_filter(&self.graph, outer, inner, |node, field_name| {
            self.boxed
                .get_first_field(&self.graph[node], field_name)
                .is_none()
        })
    }

    /// Returns `true` if this message can automatically derive Copy trait.
    pub fn can_message_derive_copy(&self, fq_message_name: &str) -> bool {
        assert_eq!(".", &fq_message_name[..1]);
        self.get_message(fq_message_name)
            .unwrap()
            .field
            .iter()
            .all(|field| self.can_field_derive_copy(fq_message_name, field))
    }

    /// Returns `true` if the type of this field allows deriving the Copy trait.
    pub fn can_field_derive_copy(
        &self,
        fq_message_name: &str,
        field: &FieldDescriptorProto,
    ) -> bool {
        assert_eq!(".", &fq_message_name[..1]);

        // repeated field cannot derive Copy
        if field.label() == Label::Repeated {
            false
        } else if field.r#type() == Type::Message {
            // nested and boxed messages cannot derive Copy
            if self
                .boxed
                .get_first_field(fq_message_name, field.name())
                .is_some()
                || self.is_directly_nested(field.type_name(), fq_message_name)
            {
                false
            } else {
                self.can_message_derive_copy(field.type_name())
            }
        } else {
            matches!(
                field.r#type(),
                Type::Float
                    | Type::Double
                    | Type::Int32
                    | Type::Int64
                    | Type::Uint32
                    | Type::Uint64
                    | Type::Sint32
                    | Type::Sint64
                    | Type::Fixed32
                    | Type::Fixed64
                    | Type::Sfixed32
                    | Type::Sfixed64
                    | Type::Bool
                    | Type::Enum
            )
        }
    }
}

/// Check two nodes is connected with edge filter
fn is_connected_with_edge_filter<F, N, E>(
    graph: &Graph<N, E>,
    start: NodeIndex,
    end: NodeIndex,
    mut is_good_edge: F,
) -> bool
where
    F: FnMut(NodeIndex, &E) -> bool,
{
    fn visitor<F, N, E>(
        graph: &Graph<N, E>,
        start: NodeIndex,
        end: NodeIndex,
        is_good_edge: &mut F,
        visited: &mut HashSet<NodeIndex>,
    ) -> bool
    where
        F: FnMut(NodeIndex, &E) -> bool,
    {
        if start == end {
            return true;
        }
        visited.visit(start);
        for edge in graph.edges_directed(start, Direction::Outgoing) {
            // if the edge doesn't pass the filter, skip it
            if !is_good_edge(start, edge.weight()) {
                continue;
            }
            let target = edge.target();
            if visited.is_visited(&target) {
                continue;
            }
            if visitor(graph, target, end, is_good_edge, visited) {
                return true;
            }
        }
        false
    }
    let mut visited = HashSet::new();
    visitor(graph, start, end, &mut is_good_edge, &mut visited)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connected() {
        let mut graph = Graph::new();
        let n1 = graph.add_node(1);
        let n2 = graph.add_node(2);
        let n3 = graph.add_node(3);
        let n4 = graph.add_node(4);
        let n5 = graph.add_node(5);
        let n6 = graph.add_node(6);
        let n7 = graph.add_node(7);
        let n8 = graph.add_node(8);
        graph.add_edge(n1, n2, 1.);
        graph.add_edge(n2, n3, 2.);
        graph.add_edge(n3, n4, 3.);
        graph.add_edge(n4, n5, 4.);
        graph.add_edge(n5, n6, 5.);
        graph.add_edge(n6, n7, 6.);
        graph.add_edge(n7, n8, 7.);
        graph.add_edge(n8, n1, 8.);
        assert!(is_connected_with_edge_filter(&graph, n2, n1, |_, edge| {
            dbg!(edge);
            true
        }),);
        assert!(is_connected_with_edge_filter(&graph, n2, n1, |_, edge| {
            dbg!(edge);
            edge < &8.5
        }),);
        assert!(!is_connected_with_edge_filter(&graph, n2, n1, |_, edge| {
            dbg!(edge);
            edge < &7.5
        }),);
    }

    #[test]
    fn test_connected_multi_circle() {
        let mut graph = Graph::new();
        let n0 = graph.add_node(0);
        let n1 = graph.add_node(1);
        let n2 = graph.add_node(2);
        let n3 = graph.add_node(3);
        let n4 = graph.add_node(4);
        graph.add_edge(n0, n1, 0.);
        graph.add_edge(n1, n2, 1.);
        graph.add_edge(n2, n3, 2.);
        graph.add_edge(n3, n0, 3.);
        graph.add_edge(n1, n4, 1.5);
        graph.add_edge(n4, n0, 2.5);
        assert!(is_connected_with_edge_filter(&graph, n1, n0, |_, edge| {
            dbg!(edge);
            edge < &2.8
        }),);
        assert!(!is_connected_with_edge_filter(&graph, n1, n0, |_, edge| {
            dbg!(edge);
            edge < &2.1
        }),);
    }
}
