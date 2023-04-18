use std::{
  collections::{HashMap, HashSet},
  io::StdoutLock,
};
use virvelvind as vv;
use vv::{
  requests::Initialize, res::MaelstromResponse, CooperativeNode, Deserialize, Event, Node,
  Serialize,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct RPCRead {
  messages: HashSet<usize>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Topology {
  topology: HashMap<vv::NetworkEntityId, Vec<vv::NetworkEntityId>>,
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

  Gossip { news: HashSet<usize> },
}

#[derive(Default)]
pub struct BroadcastServiceNode {
  init: Initialize,
  gossiped: HashSet<usize>,
  news: HashSet<usize>,
  _neighbors: Vec<vv::NetworkEntityId>,
}

impl BroadcastServiceNode {
  /// Returns a set of previously not sent values
  /// This way, at least this node can assume that all it's neighbors knows _this_ node's values
  pub fn gossip_news(&mut self) -> HashSet<usize> {
    // news_to_gossip_about = values in that are in `news` _AND NOT_ in `gossiped`
    let news_to_gossip_about: HashSet<usize> =
      self.news.difference(&self.gossiped).map(|x| *x).collect();
    self.gossiped.extend(&news_to_gossip_about);
    news_to_gossip_about
  }

  pub fn received_broadcast_messages(&self) -> HashSet<usize> {
    self
      .gossiped
      .iter()
      .chain(self.news.iter())
      .copied()
      .collect()
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
    _msg: virvelvind::req::MaelstromRequest<BroadcastServiceDefinition>,
    _local_msg_id: usize,
  ) -> Result<virvelvind::res::MaelstromResponse<BroadcastServiceDefinition>, String> {
    todo!()
  }
}

impl CooperativeNode<BroadcastServiceDefinition> for BroadcastServiceNode {
  fn setup_sidechannel_thread(
    &mut self,
    tx: std::sync::mpsc::Sender<vv::Event<BroadcastServiceDefinition>>,
  ) -> Option<std::thread::JoinHandle<()>> {
    Some(std::thread::spawn(move || loop {
      std::thread::sleep(std::time::Duration::from_millis(500));
      match tx.send(Event::GossipEvent) {
        Ok(_) => {}
        Err(err) => {
          eprintln!("Gossip event failed: {err}");
          std::process::exit(-1)
        }
      }
    }))
  }

  fn process_event(
    &mut self,
    evt: Event<BroadcastServiceDefinition>,
    local_msg_id: usize,
    stdout: &mut StdoutLock,
  ) {
    match evt {
      Event::IOEvent(msg) => {
        match msg.body.data {
        BroadcastServiceDefinition::Broadcast { message } => {
          self.news.insert(message);
          MaelstromResponse {
            src: self.init.node_id.clone(),
            dest: msg.src,
            body: vv::res::ResponseBody {
              in_reply_to: msg.body.msg_id,
              msg_id: Some(local_msg_id),
              response_type: BroadcastServiceDefinition::BroadcastOk,
            },
          }.take_send(stdout).expect("could not send broadcast ok ok");
        },
        BroadcastServiceDefinition::Read => {
          let seen = self.received_broadcast_messages();
          MaelstromResponse {
            src: self.init.node_id.clone(),
            dest: msg.src,
            body: vv::res::ResponseBody {
              in_reply_to: msg.body.msg_id,
              msg_id: Some(local_msg_id),
              response_type: BroadcastServiceDefinition::ReadOk(RPCRead { messages: seen }),
            },
          }.take_send(stdout).expect("could not send read ok");
        },
        BroadcastServiceDefinition::Topology(Topology { mut topology }) => {
          self._neighbors = topology.remove(&self.init.node_id).expect("Did not find topology data for this node in this request");
          MaelstromResponse {
            src: self.init.node_id.clone(),
            dest: msg.src,
            body: vv::res::ResponseBody {
              in_reply_to: msg.body.msg_id,
              msg_id: Some(local_msg_id),
              response_type: BroadcastServiceDefinition::TopologyOk,
            },
          }.take_send(stdout).expect("could not send topology ok");
        },
        BroadcastServiceDefinition::Gossip { news } => {
          self.news.extend(news);
        },
        BroadcastServiceDefinition::TopologyOk // these events should not be sent or received by nodes
        | BroadcastServiceDefinition::ReadOk(_)
        | BroadcastServiceDefinition::BroadcastOk => panic!("should never receive these messages"),
      }
      }
      Event::GossipEvent => {
        let news = self.gossip_news();
        if !news.is_empty() {
          let mut msg = MaelstromResponse {
            src: self.init.node_id.clone(),
            dest: "".into(),
            body: vv::res::ResponseBody {
              in_reply_to: None,
              msg_id: None,
              response_type: BroadcastServiceDefinition::Gossip { news: news.clone() },
            },
          };
          for n in self._neighbors.iter().cloned() {
            msg.dest = n;
            msg.send_ref(stdout).expect("Couldn't send message");
          }
        }
      }
    }
  }
}

fn main() -> Result<(), String> {
  vv::start_service(BroadcastServiceNode::default())
}
