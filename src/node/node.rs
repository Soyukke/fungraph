// node trait

use async_trait::async_trait;
use petgraph::{Direction, Graph, graph::NodeIndex};

#[derive(Debug, Clone)]
pub struct State {
    pub name: String,
    pub value: String,
}

pub trait FunState {}

#[async_trait]
pub trait FunNode<S: FunState> {
    fn get_name(&self) -> String;
    async fn run(&self, state: S) -> S;
}

pub enum FunEdgeType {
    Edge,
    ConditionalEdge,
}

pub trait ConditionalEdge {
    fn check(&self) -> bool;
}

pub struct FunGraph<S: FunState> {
    graph: Graph<Box<dyn FunNode<S>>, String>,
}

impl<S> FunGraph<S>
where
    S: FunState,
{
    pub fn new() -> Self {
        FunGraph {
            graph: Graph::new(),
        }
    }
    pub fn add_node<T: FunNode<S> + 'static>(&mut self, node: T) -> NodeIndex {
        let node = Box::new(node);
        self.graph.add_node(node)
    }

    pub fn add_edge(&mut self, from: NodeIndex, to: NodeIndex, edge: String) {
        self.graph.add_edge(from, to, edge);
    }

    fn get_begin_node(&self) -> NodeIndex {
        let indices: Vec<NodeIndex> = self
            .graph
            .node_indices()
            .filter(|node| {
                self.graph
                    .neighbors_directed(*node, Direction::Incoming)
                    .count()
                    == 0
            })
            .collect();

        if indices.len() != 1 {
            panic!("Begin node is not found");
        }

        indices.first().unwrap().clone()
    }

    fn get_end_node(&self) -> NodeIndex {
        let indices: Vec<NodeIndex> = self
            .graph
            .node_indices()
            .filter(|node| {
                self.graph
                    .neighbors_directed(*node, Direction::Outgoing)
                    .count()
                    == 0
            })
            .collect();
        if indices.len() != 1 {
            panic!("End node is not found");
        }
        indices.first().unwrap().clone()
    }

    pub async fn run(&self, state: S) -> S {
        let begin_node = self.get_begin_node();
        let end_node = self.get_end_node();
        let mut current_node = begin_node;
        let mut current_state = state;
        loop {
            let node = self.graph.node_weight(current_node).unwrap();
            current_state = node.run(current_state).await;
            let next_nodes: Vec<NodeIndex> = self
                .graph
                .neighbors_directed(current_node, Direction::Outgoing)
                .collect();
            if next_nodes.len() == 0 {
                break;
            }
            if next_nodes.len() > 1 {
                panic!("Multiple next nodes are not supported");
            }
            current_node = next_nodes.first().unwrap().clone();
        }
        current_state
    }
}

pub struct FunGraphBuilder<S: FunState> {
    graph: Graph<Box<dyn FunNode<S>>, String>,
}

impl<S> FunGraphBuilder<S>
where
    S: FunState,
{
    pub fn new() -> Self {
        FunGraphBuilder {
            graph: Graph::new(),
        }
    }

    pub fn add_node<T: FunNode<S> + 'static>(&mut self, node: T) -> NodeIndex {
        let node = Box::new(node);
        self.graph.add_node(node)
    }

    pub fn add_edge(&mut self, from: NodeIndex, to: NodeIndex, edge: String) {
        self.graph.add_edge(from, to, edge);
    }
}
