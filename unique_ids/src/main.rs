use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use virvelvind as vv;

use vv::{
    req::Initialize,
    res::{MaelstromResponse, ResponseBody},
    Node,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Id<T> {
    id: T,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")] // make enum "internally tagged"
pub enum UniqueIdGenerationDefinition<T> {
    Generate,
    // GenerateOk { id: T },
    GenerateOk(Id<T>),
}

/// This is a very simple and stupid (and easy to break in production)
/// id generating service. What it does is, it takes current timestamp
/// and prepends it with the node ide name, found in `self.init.node_id`
/// Also, this service has a min-required time span of 1us - any requests that get served
/// in multiples shorter than that, will hand out duplicates. This is bad. But it's fine for this
#[derive(Default)]
pub struct UniqueIdServiceNode {
    init: Initialize,
}

impl UniqueIdServiceNode {
    pub fn new() -> UniqueIdServiceNode {
        UniqueIdServiceNode {
            init: Initialize::default(),
        }
    }

    // free standing 'static' member function
    pub fn generate_id(node_id: &str) -> Id<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros();
        Id {
            id: format!("{node_id}@{now:X}"),
        }
    }
}

impl Node<UniqueIdGenerationDefinition<String>> for UniqueIdServiceNode {
    fn init(&mut self, init: Initialize) {
        self.init = init;
    }

    fn get_init(&self) -> &Initialize {
        &self.init
    }

    fn process_message(
        &mut self,
        msg: vv::req::MaelstromRequest<UniqueIdGenerationDefinition<String>>,
        local_msg_id: usize,
    ) -> Result<vv::res::MaelstromResponse<UniqueIdGenerationDefinition<String>>, String> {
        match msg.body.data {
          UniqueIdGenerationDefinition::Generate => {
            Ok(MaelstromResponse {
              src: self.init.node_id.clone(),
              dest: msg.src,
              body: ResponseBody {
                  msg_id: local_msg_id,
                  // response_type: UniqueIdGenerationDefinition::GenerateOk { id: UniqueIdServiceNode::generate_id(&self.init.node_id) },
                  response_type: UniqueIdGenerationDefinition::GenerateOk(UniqueIdServiceNode::generate_id(&self.init.node_id)),
                  in_reply_to: msg.body.msg_id,
              },
          })
          },
          UniqueIdGenerationDefinition::GenerateOk(Id { id }) => Err(format!("We have been sent a GenerateOk response - we are not taking requests at this time {id}"))
      }
    }
}

fn main() -> Result<(), String> {
    vv::start_maelstrom_service_node(UniqueIdServiceNode::default())
}
