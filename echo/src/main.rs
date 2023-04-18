use virvelvind::{
  requests::{Initialize, MaelstromRequest},
  response::{MaelstromResponse, ResponseBody},
  Deserialize, Node, Serialize,
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")] // make enum "internally tagged"
pub enum EchoServiceDefinition {
  Echo { echo: String },
  EchoOk { echo: String },
}

#[derive(Default)]
pub struct EchoServiceNode {
  init: Initialize,
}

impl EchoServiceNode {
  fn handle_request(
    &mut self,
    msg: MaelstromRequest<EchoServiceDefinition>,
    msg_id: usize,
  ) -> Result<MaelstromResponse<EchoServiceDefinition>, String> {
    match msg.body.data {
      EchoServiceDefinition::Echo { echo } => Ok(MaelstromResponse {
        src: self.init.node_id.clone(),
        dest: msg.src,
        body: ResponseBody {
          msg_id: Some(msg_id),
          response_type: EchoServiceDefinition::EchoOk { echo: echo },
          in_reply_to: msg.body.msg_id,
        },
      }),
      unexpected @ _ => Err(format!("Should not receive {unexpected:?}")),
    }
  }
}
//
impl Node<EchoServiceDefinition> for EchoServiceNode {
  fn init(&mut self, init: Initialize) {
    self.init = init;
  }

  fn get_init(&self) -> &Initialize {
    &self.init
  }

  fn process_message(
    &mut self,
    msg: virvelvind::req::MaelstromRequest<EchoServiceDefinition>,
    local_msg_id: usize,
  ) -> Result<virvelvind::res::MaelstromResponse<EchoServiceDefinition>, String> {
    self.handle_request(msg, local_msg_id)
  }
}

fn main() -> Result<(), String> {
  virvelvind::start_maelstrom_service_node(EchoServiceNode::default())
}
