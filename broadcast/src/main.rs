use std::collections::{HashSet, HashMap};
use virvelvind as vv;
use vv::{requests::Initialize, Deserialize, Node, Serialize};


#[derive(Debug, Serialize, Deserialize)]
pub struct RPCRead {
  messages: HashSet<usize>
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Topology {
  topology: HashMap<vv::NetworkEntityId, Vec<vv::NetworkEntityId>>
}

impl Topology {
  pub fn neighborhood_of(&self, node_id: &vv::NetworkEntityId) -> Vec<vv::NetworkEntityId> {
    self.topology.get(node_id).unwrap().clone()
  }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")] // make enum "internally tagged"
/// https://fly.io/dist-sys/3a/ defines the broadcast service
pub enum BroadcastServiceDefinition {
    Broadcast { message: usize },
    BroadcastOk,

    Read,
    ReadOk(RPCRead),

    Topology(Topology),
    TopologyOk,

    Gossip { news: HashSet<usize> }
}

#[derive(Default)]
pub struct BroadcastServiceNode {
    init: Initialize,
    gossiped: HashSet<usize>,
    news: HashSet<usize>,
    neighbors: Topology
}

impl BroadcastServiceNode {
  pub fn gossip_news(&mut self) -> HashSet<usize> {
    let news_to_gossip_about: HashSet<usize> = self.gossiped.difference(&self.news).map(|x| *x).collect();
    for item in &news_to_gossip_about {
      self.gossiped.insert(*item);
    }
    news_to_gossip_about
  }
}

impl Node<BroadcastServiceDefinition> for BroadcastServiceNode {
    fn init(&mut self, init: Initialize) {
        self.init = init;
    }

    fn get_init(&self) -> &Initialize {
        &self.init
    }

    fn process_message(
        &mut self,
        msg: virvelvind::req::MaelstromRequest<BroadcastServiceDefinition>,
        local_msg_id: usize,
    ) -> Result<virvelvind::res::MaelstromResponse<BroadcastServiceDefinition>, String> {
        todo!()
    }
}

fn main() -> Result<(), String> {
    virvelvind::start_maelstrom_service_node(BroadcastServiceNode::default())
}
