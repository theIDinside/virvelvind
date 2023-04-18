use std::{
  collections::{HashMap, HashSet},
  io::StdoutLock,
  time::UNIX_EPOCH,
};
use virvelvind as vv;
use vv::{
  requests::Initialize, res::MaelstromResponse, CooperativeNode, Deserialize, Event, Node,
  Serialize,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct RPCRead {
  messages: Vec<usize>,
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

  Gossip { news: Vec<GossipMessage> },
  GossipReceipt { receipt: Vec<BatchId> },
}

type GossipPayload = HashSet<usize>;
type BatchId = usize;
type OtherNodeBatchId = usize;

#[derive(Serialize, Deserialize, Debug)]
pub struct GossipMessage {
  id: BatchId,
  payload: GossipPayload,
}

// This design is meant to be made generic; or rather, reused with minor modifications
// so that if the data that's being sent
// is larger than a usize (let's say, it could be half a MB), then we don't
// want to send "acknowledged" gossip messages, containing the seen items (as it would gain in size fast), but instead a representation of it
// by using ID's, and in this case, we group messages together by batch ID:s.
#[derive(Default)]
pub struct BroadcastServiceNode {
  init: Initialize,
  // the current gossip batch that's being produced
  current_new_message_state: HashSet<usize>,
  // all gossip batches that has been produced (and sent), as a mapping of BatchId -> MessageBatch
  message_batches: HashMap<BatchId, HashSet<usize>>,
  // mapping of Node -> received batches ID's (these batches represent the batches produced on _source_ node, not this one)
  received_batches: HashMap<String, HashSet<OtherNodeBatchId>>,
  // mapping of node id -> ACK'd gossip batch ID's, so that we know what batche(s) node N has received
  acknowledged_sent_batches: HashMap<String, HashSet<BatchId>>,
  // neighboring nodes
  neighbors: Vec<vv::NetworkEntityId>,
}

impl BroadcastServiceNode {
  pub fn has_seen(&self, value: usize) -> bool {
    for v in self.message_batches.values().flat_map(|s| s.iter()) {
      if value == *v {
        return true;
      }
    }
    for v in self.current_new_message_state.iter() {
      if value == *v {
        return true;
      }
    }
    return false;
  }

  pub fn get_unknown(&mut self, node: &String) -> Vec<GossipMessage> {
    if !self.current_new_message_state.is_empty() {
      let new_id = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("")
        .as_micros() as usize;
      let mut swap = HashSet::default();
      std::mem::swap(&mut swap, &mut self.current_new_message_state);
      self.message_batches.insert(new_id, swap);
      assert!(self.current_new_message_state.is_empty(), "Should be empty here!");
    }
    let known_ids = self.acknowledged_sent_batches.get(node).expect("failed");
    let res = self
      .message_batches
      .keys()
      .filter_map(|id| {
        if !known_ids.contains(id) {
          Some(GossipMessage {
            id: *id,
            payload: self.message_batches.get(id)?.clone(),
          })
        } else {
          None
        }
      })
      .collect();

    res
  }

  /// Constructs all messages that this node has seen, by iterating over all produced batches (as well as the one being currently built in `current_new_message_state`)
  pub fn all_messages(&self) -> Vec<usize> {
    let total_msg_cnt = self
      .message_batches
      .values()
      .fold(0, |msg_count, set| set.len() + msg_count)
      + self.current_new_message_state.len();

    let mut preallocated = Vec::with_capacity(total_msg_cnt);
    for msg in self
      .message_batches
      .values()
      .flat_map(|set| set)
      .chain(self.current_new_message_state.iter())
    {
      preallocated.push(*msg);
    }
    preallocated
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
      std::thread::sleep(std::time::Duration::from_millis(12));
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
          self.current_new_message_state.insert(message);
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
          let seen = self.all_messages();
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
          let nbs = topology.remove(&self.init.node_id).expect("Did not find topology data for this node in this request");
          for nb in nbs.iter().cloned() {
            self.acknowledged_sent_batches.insert(nb.clone(), Default::default());
            self.received_batches.insert(nb, Default::default());
          }

          self.neighbors = nbs;

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
          let mut receipts = vec![];
          for batch in news {
            receipts.push(batch.id);
            if !self.received_batches.get(&msg.src).unwrap().contains(&batch.id) {
              for value in batch.payload {
                if !self.has_seen(value) {
                  self.current_new_message_state.insert(value);
                }
              }
              self.received_batches.get_mut(&msg.src).unwrap().insert(batch.id);
            }
          }

          MaelstromResponse {
            src: self.init.node_id.clone(),
            dest: msg.src,
            body: vv::res::ResponseBody::uni_dir(BroadcastServiceDefinition::GossipReceipt { receipt: receipts })
          }.take_send(stdout).expect("Failed to send receipt");
        },
        BroadcastServiceDefinition::GossipReceipt { receipt } => {
          for id in receipt {
            self.acknowledged_sent_batches.get_mut(&msg.src).unwrap().insert(id);
          }
        },
        BroadcastServiceDefinition::TopologyOk // these events should not be sent or received by nodes
        | BroadcastServiceDefinition::ReadOk(_)
        | BroadcastServiceDefinition::BroadcastOk => panic!("should never receive these messages"),

      }
      }
      Event::GossipEvent => {
        let nodes: Vec<_> = self.neighbors.iter().cloned().collect();
        for n in nodes {
          let news = self.get_unknown(&n);
          if !news.is_empty() {
            MaelstromResponse {
              src: self.init.node_id.clone(),
              dest: n,
              body: vv::res::ResponseBody::uni_dir(BroadcastServiceDefinition::Gossip { news: news })
            }.take_send(stdout).expect("failed to send gossip event");
          }
        }
      }
    }
  }
}

fn main() -> Result<(), String> {
  vv::start_service(BroadcastServiceNode::default())
}
