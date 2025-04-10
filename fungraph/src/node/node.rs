// node trait

use async_trait::async_trait;
use log::debug;
use petgraph::{Direction, Graph, graph::NodeIndex, visit::EdgeRef};

#[derive(Debug, Clone)]
pub struct State {
    pub name: String,
    pub value: String,
}

pub trait FunState: Send + Sync {}

#[async_trait]
pub trait FunNode<S: FunState>: Send + Sync {
    fn get_name(&self) -> String;
    async fn run(&self, state: &mut S);
}

pub struct StartFunNode;
pub struct EndFunNode;

#[async_trait]
impl<S> FunNode<S> for StartFunNode
where
    S: FunState + 'static,
{
    fn get_name(&self) -> String {
        "Start".to_string()
    }
    async fn run(&self, state: &mut S) {}
}

#[async_trait]
impl<S> FunNode<S> for EndFunNode
where
    S: FunState + 'static,
{
    fn get_name(&self) -> String {
        "End".to_string()
    }
    async fn run(&self, state: &mut S) {}
}

pub enum FunEdgeType<S: FunState> {
    Edge,
    ConditionalEdge(fn(&S) -> bool),
}

pub struct FunGraph<S: FunState> {
    graph: Graph<Box<dyn FunNode<S>>, FunEdgeType<S>>,
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

    pub fn add_edge(&mut self, from: NodeIndex, to: NodeIndex) {
        self.graph.add_edge(from, to, FunEdgeType::Edge);
    }

    pub fn add_edge_with_condition(
        &mut self,
        from: NodeIndex,
        to: NodeIndex,
        condition: fn(&S) -> bool,
    ) {
        self.graph
            .add_edge(from, to, FunEdgeType::ConditionalEdge(condition));
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
            debug!("End node indices: {:?}", indices);
            panic!("End node is not found");
        }
        indices.first().unwrap().clone()
    }

    pub async fn run(&self, state: S) -> S {
        let begin_node = self.get_begin_node();
        let _end_node = self.get_end_node();
        let mut current_node = begin_node;
        let mut current_state = state;
        loop {
            let node = self.graph.node_weight(current_node).unwrap();
            node.run(&mut current_state).await;
            let next_nodes: Vec<NodeIndex> = self
                .graph
                .neighbors_directed(current_node, Direction::Outgoing)
                .collect();

            let mut edges = self.graph.edges(current_node);

            // update current_node
            // Prirority: Edge > ConditionalEdge
            let mut next_node = current_node.clone();
            while let Some(edge) = edges.next() {
                let source = edge.source();
                let target = edge.target();
                let weight = edge.weight();
                match weight {
                    FunEdgeType::Edge => {
                        debug!("Edge from {:?} to {:?}", source, target);
                        next_node = target;
                        break;
                    }
                    FunEdgeType::ConditionalEdge(condition) => {
                        if condition(&current_state) {
                            debug!("Conditional edge from {:?} to {:?}", source, target);
                            next_node = target;
                            break;
                        } else {
                            debug!(
                                "Conditional edge from {:?} to {:?} is not taken",
                                source, target
                            );
                        }
                    }
                }
            }
            if next_node == current_node {
                debug!("End node reached: {:?}", current_node);
                break;
            }
            debug!("Next node: {:?}", next_node);
            current_node = next_node;
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

trait ConditionalEdge {
    type Input;
    fn check(&self, state: &Self::Input) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    fn init_logger() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[derive(Debug)]
    struct MyState {}

    impl FunState for MyState {}

    struct AlwaysTrueNode;
    impl ConditionalEdge for AlwaysTrueNode {
        type Input = MyState;
        fn check(&self, state: &Self::Input) -> bool {
            debug!("AlwaysTrueNode");
            true
        }
    }

    struct AlwaysFalseNode;
    impl ConditionalEdge for AlwaysFalseNode {
        type Input = MyState;
        fn check(&self, state: &Self::Input) -> bool {
            debug!("AlwaysFalseNode");
            false
        }
    }

    // cargo test node::node::tests::test_loop_graph
    #[tokio::test]
    async fn test_loop_graph() {
        init_logger();
        let node0 = StartFunNode {};
        let node1 = StartFunNode {};
        let node2 = StartFunNode {};
        let node3 = EndFunNode {};
        let mut graph: FunGraph<MyState> = FunGraph::new();
        let start = graph.add_node(node0);
        let i_1 = graph.add_node(node1);
        let i_2 = graph.add_node(node2);
        let end = graph.add_node(node3);
        graph.add_edge(start, i_1);
        graph.add_edge_with_condition(i_1, i_2, |state: &MyState| {
            debug!("i_1 -> i_2 is false");
            false
        });
        graph.add_edge(i_2, i_1);
        graph.add_edge_with_condition(i_1, end, |state: &MyState| {
            debug!("i_1 -> i_end is true");
            true
        });

        graph.run(MyState {}).await;
        assert_eq!(graph.graph.node_count(), 4);
    }
}
