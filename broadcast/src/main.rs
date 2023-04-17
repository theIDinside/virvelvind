use virvelvind::{
    requests::{Initialize, MaelstromRequest},
    response::{MaelstromResponse, ResponseBody},
    Deserialize, Node, Serialize,
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")] // make enum "internally tagged"
pub enum BroadcastServiceDefinition {}

#[derive(Default)]
pub struct BroadcastServiceNode {
    init: Initialize,
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
