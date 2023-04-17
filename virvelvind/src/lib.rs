use serde::{Deserialize, Serialize};

type ClientId = String;
type NodeId = String;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")] // make enum "internally tagged"
pub enum RequestType {
    #[serde(rename = "init")]
    Init {
        node_id: NodeId,
        node_ids: Vec<NodeId>,
    },

    #[serde(rename = "echo")]
    Echo { echo: String },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")] // make enum "internally tagged"
pub enum ResponseType {
    #[serde(rename = "init_ok")]
    InitOk,
    #[serde(rename = "echo_ok")]
    EchoOk { echo: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestBody {
    #[serde(flatten)]
    pub request_type: RequestType,
    pub msg_id: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseBody {
    pub msg_id: usize,
    #[serde(flatten)]
    pub response_type: ResponseType,
    pub in_reply_to: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestMessage {
    pub id: usize,
    pub src: ClientId,
    pub dest: NodeId,
    pub body: RequestBody,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseMessage {
    pub src: NodeId,
    pub dest: ClientId,
    pub body: ResponseBody,
}

#[derive(Debug)]
pub struct RequestError<'a> {
    pub serde_erron: serde_json::Error,
    pub contents: &'a str,
}

pub trait Node {
    type ProcessingResult;
    type RegisterResult;
    fn register_as(&mut self, node_id: &str) -> Self::RegisterResult;
    fn process(&mut self, msg: RequestMessage) -> Self::ProcessingResult;
}

pub fn parse_request<'a>(input: &'a str) -> Result<RequestMessage, RequestError<'a>> {
    serde_json::from_str(input.trim()).map_err(|e| {
        eprintln!("errored on input: '{input}'");
        RequestError {
            serde_erron: e,
            contents: input,
        }
    })
}

pub fn prepare_response(msg: &ResponseMessage) -> Result<String, serde_json::Error> {
    serde_json::to_string(msg)
}

#[cfg(test)]
mod tests {
    use crate::{RequestBody, RequestMessage};

    #[test]
    fn serialize() -> Result<(), serde_json::Error> {
        let request = RequestMessage {
            body: RequestBody {
                msg_id: 1,
                request_type: crate::RequestType::Echo { echo: "foo".into() },
            },
            dest: "n1".into(),
            src: "c1".into(),
        };
        let ser = serde_json::to_string(&request)?;
        println!("ser: {ser}");
        Ok(())
    }
}
