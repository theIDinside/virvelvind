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

pub struct EchoServiceNode {
    init: Initialize,
    local_txn_id: usize,
}

impl EchoServiceNode {
    pub fn new() -> Self {
        Self {
            local_txn_id: 1,
            init: Initialize::default(),
        }
    }

    fn handle_request(
        &mut self,
        msg: MaelstromRequest<EchoServiceDefinition>,
    ) -> Result<MaelstromResponse<EchoServiceDefinition>, String> {
        self.local_txn_id += 1;
        match msg.body.data {
            EchoServiceDefinition::Echo { echo } => Ok(MaelstromResponse {
                src: self.init.node_id.clone(),
                dest: msg.src,
                body: ResponseBody {
                    msg_id: self.local_txn_id,
                    response_type: EchoServiceDefinition::EchoOk { echo: echo },
                    in_reply_to: msg.body.msg_id,
                },
            }),
            unexpected @ _ => Err(format!("Should not receive {unexpected:?}")),
        }
    }
}

impl Node<EchoServiceDefinition> for EchoServiceNode {
    fn process(
        &mut self,
        msg: MaelstromRequest<EchoServiceDefinition>,
    ) -> Result<MaelstromResponse<EchoServiceDefinition>, String> {
        self.handle_request(msg)
    }

    fn init(&mut self, init: Initialize) {
        self.init = init;
    }

    fn get_init(&self) -> &Initialize {
        &self.init
    }
}

fn main() -> Result<(), String> {
    virvelvind::start_maelstrom_service_node(EchoServiceNode::new())
}
