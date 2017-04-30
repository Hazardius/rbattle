use graph::{Graph, Node};
use map::Map;
use std::rc::Rc;

/// A player id number.
pub struct Player(usize);

/// The complete state of an RBattle game board.
pub struct State<G: Graph> {
    /// The map being played on.
    map: Rc<Map<G>>,

    /// The state of each node on the map.
    nodes: Vec<NodeState>
}

/// The state of a single node on a game board.
pub struct NodeState {
    /// The player who controls this node, if any.
    owner: Option<Player>,

    /// The amount of goop in this node. This ranges from 0 to MAX_GOOP,
    /// inclusive.
    goop: usize,

    /// Which neighbors of this node goop flows out to. It is a rule of the game
    /// that nodes with inflows from more than one player may not have any outflows.
    outflows: Vec<Node>
}

pub const MAX_GOOP: usize = 15;