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
    fn get_name(&self) -> &'static str;
    async fn run(&self, state: &mut S);
}

pub struct StartFunNode;
pub struct EndFunNode;

#[async_trait]
impl<S> FunNode<S> for StartFunNode
where
    S: FunState,
{
    fn get_name(&self) -> &'static str {
        "Start"
    }
    async fn run(&self, _state: &mut S) {}
}

#[async_trait]
impl<S> FunNode<S> for EndFunNode
where
    S: FunState,
{
    fn get_name(&self) -> &'static str {
        "End"
    }
    async fn run(&self, _state: &mut S) {}
}

pub enum FunEdgeType<S: FunState> {
    Edge,
    ConditionalEdge(fn(&S) -> bool),
}

pub struct FunGraph<S: FunState> {
    graph: Graph<Box<dyn FunNode<S>>, FunEdgeType<S>>,
    start_node_index: NodeIndex,
    end_node_index: NodeIndex,
}

impl<S> FunGraph<S>
where
    S: FunState,
{
    pub fn new() -> Self {
        let mut graph: Graph<Box<dyn FunNode<S>>, FunEdgeType<S>> = Graph::new();
        let start_node_index = graph.add_node(Box::new(StartFunNode {}));
        let end_node_index = graph.add_node(Box::new(EndFunNode {}));
        FunGraph {
            graph,
            start_node_index,
            end_node_index,
        }
    }
    pub fn add_node<T: FunNode<S> + 'static>(&mut self, node: T) -> NodeIndex {
        let node = Box::new(node);
        self.graph.add_node(node)
    }

    pub fn add_edge(&mut self, from: NodeIndex, to: NodeIndex) {
        self.graph.add_edge(from, to, FunEdgeType::Edge);
    }

    pub fn add_start_edge(&mut self, to: NodeIndex) {
        self.graph
            .add_edge(self.start_node_index, to, FunEdgeType::Edge);
    }

    pub fn add_end_edge(&mut self, from: NodeIndex) {
        self.graph
            .add_edge(from, self.end_node_index, FunEdgeType::Edge);
    }

    pub fn add_conditional_end_edge(&mut self, from: NodeIndex, condition: fn(&S) -> bool) {
        self.graph.add_edge(
            from,
            self.end_node_index,
            FunEdgeType::ConditionalEdge(condition),
        );
    }

    pub fn add_conditional_edge(
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
        let mut current_node = self.get_start_node_index();
        let mut current_state = state;

        while !self.is_end_node(current_node) {
            let (next_node, new_state) = self.run_step(current_node, current_state).await;
            current_state = new_state;

            match next_node {
                Some(node) => {
                    current_node = node;
                }
                None => {
                    // 次のノードがない場合の処理
                    break;
                }
            }
        }

        current_state
    }

    pub fn get_start_node_index(&self) -> NodeIndex {
        self.start_node_index
    }

    pub fn is_end_node(&self, node_index: NodeIndex) -> bool {
        node_index == self.end_node_index
    }

    pub async fn run_step(&self, current_node: NodeIndex, mut state: S) -> (Option<NodeIndex>, S) {
        let node = self.graph.node_weight(current_node).unwrap();
        node.run(&mut state).await;

        match self.get_next_node_index(current_node, &state) {
            Some(next_node) => (Some(next_node), state),
            None => (None, state),
        }
    }

    pub fn get_next_node_index(&self, current_node: NodeIndex, state: &S) -> Option<NodeIndex> {
        let mut edges = self.graph.edges(current_node);

        while let Some(edge) = edges.next() {
            let target = edge.target();
            let weight = edge.weight();

            match weight {
                FunEdgeType::Edge => {
                    return Some(target);
                }
                FunEdgeType::ConditionalEdge(condition) => {
                    if condition(state) {
                        return Some(target);
                    }
                }
            }
        }

        None
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

#[cfg(test)]
mod tests {
    use super::*;
    fn init_logger() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[derive(Debug)]
    struct MyState {}

    impl FunState for MyState {}

    // cargo test node::node::tests::test_loop_graph
    #[tokio::test]
    async fn test_loop_graph() {
        init_logger();
        let node1 = StartFunNode {};
        let node2 = StartFunNode {};
        let mut graph: FunGraph<MyState> = FunGraph::new();
        let i_1 = graph.add_node(node1);
        let i_2 = graph.add_node(node2);
        graph.add_start_edge(i_1);
        graph.add_conditional_edge(i_1, i_2, |_: &MyState| {
            debug!("i_1 -> i_2 is false");
            false
        });
        graph.add_edge(i_2, i_1);
        graph.add_conditional_end_edge(i_1, |_: &MyState| {
            debug!("i_1 -> i_end is true");
            true
        });

        graph.run(MyState {}).await;
        assert_eq!(graph.graph.node_count(), 4);
    }
}
