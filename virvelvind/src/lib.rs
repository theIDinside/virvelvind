use std::io::{BufRead, BufReader, StdoutLock, Write};
// rename
pub use requests as req;
pub use response as res;

use req::Initialize;
use res::{MaelstromResponse, ResponseBody};

pub use serde::{de::DeserializeOwned, Deserialize, Serialize};

type NetworkEntityId = String;

// Re-export it all to consumer
pub mod prelude {
  pub use crate::req::*;
  pub use crate::res::*;
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")] // make enum "internally tagged"
enum MaelstromService {
    InitOk,
}

pub mod requests {
    use crate::NetworkEntityId;
    use serde::{de::DeserializeOwned, Deserialize, Serialize};

    #[derive(Deserialize, Serialize, Default)]
    pub struct Initialize {
        pub node_id: NetworkEntityId,
        pub node_ids: Vec<NetworkEntityId>,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct RequestBody<ServiceRequestType> {
        #[serde(flatten)]
        pub data: ServiceRequestType,
        pub msg_id: usize,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct MaelstromRequest<ServiceRequestType> {
        pub src: NetworkEntityId,
        pub dest: NetworkEntityId,
        pub body: RequestBody<ServiceRequestType>,
    }

    pub fn parse_request<'a, S: DeserializeOwned>(
        input: &'a str,
    ) -> Result<MaelstromRequest<S>, serde_json::Error> {
        serde_json::from_str(input.trim()).map_err(|e| {
            eprintln!("errored on input: '{input}'");
            e
        })
    }
}

pub mod response {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    pub struct ResponseBody<ServiceResponseType> {
        pub msg_id: usize,
        #[serde(flatten)]
        pub response_type: ServiceResponseType,
        pub in_reply_to: usize,
    }

    #[derive(Debug, Serialize)]
    pub struct MaelstromResponse<ServiceType> {
        pub src: crate::NetworkEntityId,
        pub dest: crate::NetworkEntityId,
        pub body: ResponseBody<ServiceType>,
    }
}

pub trait Node<ServiceType>
where
    ServiceType: DeserializeOwned + Serialize,
{
    fn init(&mut self, init: Initialize);
    fn get_init(&self) -> &Initialize;

    fn is_initialized(&self) -> bool {
      let init = self.get_init();
      let default_init = Initialize::default();
      default_init.node_id != init.node_id
    }
    fn process_message(
        &mut self,
        msg: req::MaelstromRequest<ServiceType>,
    ) -> Result<res::MaelstromResponse<ServiceType>, String>;
}

pub fn prepare_response<ServiceType: serde::Serialize>(
    msg: &res::MaelstromResponse<ServiceType>,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(msg)
}

fn init_response(
    in_reply_to: usize,
    msg_id: usize,
    src: String,
    dest: String,
) -> MaelstromResponse<MaelstromService> {
    MaelstromResponse {
        src: src,
        dest: dest,
        body: ResponseBody {
            msg_id,
            response_type: MaelstromService::InitOk,
            in_reply_to,
        },
    }
}

pub fn start_maelstrom_service_node<N, ServiceType>(mut node: N) -> Result<(), String>
where
    N: Node<ServiceType>,
    ServiceType: Serialize + DeserializeOwned,
{
    let mut stdin = std::io::stdin().lock();
    let mut buf = String::with_capacity(512);
    stdin
        .read_line(&mut buf)
        .map_err(|_| "Failed to read init packet")?;
    eprintln!("first packet: {}", &buf);
    let init: req::MaelstromRequest<Initialize> = serde_json::from_str(&buf).map_err(|e| {
        format!("Init request always required but failed to parse: {e}. Contents: {buf}")
    })?;
    node.init(init.body.data);

    if node.is_initialized() {
      panic!("Node initialized with faulty settings");
    }

    let init_respose_ = init_response(init.body.msg_id, 1, init.dest, init.src);
    let msg =
        serde_json::to_string(&init_respose_).map_err(|_| "Failed to serialize init resposne")?;
    let mut stdout: StdoutLock = std::io::stdout().lock();
    stdout
        .write(msg.as_bytes())
        .expect("Failed to send init response");
    stdout.write_all(b"\n").expect("newline");

    let mut reader = BufReader::new(stdin);
    loop {
        buf.clear();
        reader.read_line(&mut buf).expect("Failed to read input");
        let req: req::MaelstromRequest<ServiceType> =
            req::parse_request(&buf).map_err(|e| format!("Failed to parse request: {e:?}"))?;
        match node.process_message(req) {
            Err(err) => {
                eprintln!("failed to process message {err}")
            }
            Ok(response) => {
                let contents =
                    serde_json::to_string(&response).map_err(|_| "Couldn't serialize message")?;
                stdout
                    .write(contents.as_bytes())
                    .expect("Failed to write response");
                stdout.write_all(b"\n").expect("Failed to write newline");
            }
        }
    }
}