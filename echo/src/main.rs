use std::io::{BufRead, BufReader, StdoutLock, Write};

use virvelvind::{
    parse_request, prepare_response, Node, RequestMessage, RequestType, ResponseBody,
    ResponseMessage, ResponseType,
};

pub struct Echo<'a> {
    node_id: Option<String>,
    comm: StdoutLock<'a>,
    local_txn_id: usize,
}

impl<'a> Echo<'a> {
    pub fn new(stdout: StdoutLock<'a>) -> Self {
        Self {
            comm: stdout,
            local_txn_id: 1,
            node_id: None,
        }
    }

    fn handle_request(&mut self, msg: RequestMessage) -> ResponseMessage {
        self.local_txn_id += 1;
        match msg.body.request_type {
            // TODO(simon): we're not handling multiple nodes at this point. We just grab the target node we're being
            // provided and get that as a name. This should really be placed _behind_ this application gateway
            // so that maelstrom connects to this, tells us how many nodes to spin up and _this_ then delegates to creating new nodes
            RequestType::Init { node_id, .. } => {
                self.register_as(&node_id).expect("Failed to register node id");
                ResponseMessage {
                    src: node_id,
                    dest: msg.src,
                    body: ResponseBody {
                        msg_id: self.local_txn_id,
                        response_type: ResponseType::InitOk,
                        in_reply_to: msg.body.msg_id,
                    },
                }
            }

            RequestType::Echo { echo } => ResponseMessage {
                src: self.node_id.as_ref().expect("NODE ID NOT SET").clone(),
                dest: msg.src,
                body: ResponseBody {
                    msg_id: self.local_txn_id,
                    response_type: ResponseType::EchoOk { echo: echo },
                    in_reply_to: msg.body.msg_id,
                },
            },
        }
    }
}

impl<'a> Node for Echo<'a> {
    type ProcessingResult = Result<(), &'static str>;
    type RegisterResult = Result<(), ()>;

    fn process(&mut self, msg: RequestMessage) -> Self::ProcessingResult {
        let response = self.handle_request(msg);
        let prepared_response =
            prepare_response(&response).map_err(|_| "Failed to prepare response")?;
        self.comm
            .write(prepared_response.as_bytes())
            .map_err(|_| "failed to write to stdout")?;
        self.comm
            .write_all(b"\n")
            .map_err(|_| "Failed to write to stdout")?;
        Ok(())
    }

    fn register_as(&mut self, node_id: &str) -> Self::RegisterResult {
        self.node_id = Some(node_id.into());
        Ok(())
    }
}

fn main() -> Result<(), String> {
    let stdin = std::io::stdin().lock();

    let mut reader = BufReader::new(stdin);
    let stdout: StdoutLock = std::io::stdout().lock();
    let mut buf = String::with_capacity(512);
    let mut echo_node = Echo::new(stdout);

    loop {
        let bytes_read = reader.read_line(&mut buf).expect("Failed to read input");
        let req = parse_request(&buf[0..bytes_read])
            .map_err(|e| format!("Failed to parse request: {e:?}"))?;
        buf.clear();
        match echo_node.process(req) {
            Err(err) => {
                eprintln!("failed to process message {err}")
            }
            _ => {}
        }
    }
}
