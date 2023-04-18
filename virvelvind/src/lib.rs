use std::io::{BufRead, BufReader, StdinLock, StdoutLock, Write};
// rename
pub use requests as req;
use requests::MaelstromRequest;
pub use response as res;

use req::Initialize;
use res::{MaelstromResponse, ResponseBody};

pub use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub use String as NetworkEntityId;

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
  use crate::{
    res::{MaelstromResponse, ResponseBody},
    NetworkEntityId,
  };
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msg_id: Option<usize>,
  }

  impl<ServiceRequestType> RequestBody<ServiceRequestType> {
    pub fn into_response(self, msg_id: Option<usize>) -> ResponseBody<ServiceRequestType> {
      ResponseBody {
        in_reply_to: self.msg_id,
        msg_id: msg_id,
        response_type: self.data.into(),
      }
    }
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

  impl<ServiceRequestType> MaelstromRequest<ServiceRequestType> {
    pub fn into_reply(
      self,
      msg_id: Option<usize>,
    ) -> MaelstromResponse<ServiceRequestType> {
      MaelstromResponse {
        src: self.dest,
        dest: self.src,
        body: self.body.into_response(msg_id),
      }
    }
  }
}

pub mod response {
  use serde::{Deserialize, Serialize};

  #[derive(Debug, Serialize, Deserialize)]
  pub struct ResponseBody<ServiceResponseType> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_reply_to: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msg_id: Option<usize>,
    #[serde(flatten)]
    pub response_type: ServiceResponseType,
  }

  #[derive(Debug, Serialize, Deserialize)]
  pub struct MaelstromResponse<ServiceType> {
    pub src: crate::NetworkEntityId,
    pub dest: crate::NetworkEntityId,
    pub body: ResponseBody<ServiceType>,
  }

  impl<ServiceType> MaelstromResponse<ServiceType>
  where
    ServiceType: Serialize,
  {
    pub fn take_send<W: std::io::Write>(self, output: &mut W) -> Result<(), &'static str> {
      let contents = serde_json::to_string(&self).map_err(|_| "Couldn't serialize message")?;
      output
        .write(contents.as_bytes())
        .expect("Failed to write response");
      output.write_all(b"\n").expect("Failed to write newline");
      Ok(())
    }

    pub fn send_ref<W: std::io::Write>(&self, output: &mut W) -> Result<(), &'static str> {
      let contents = serde_json::to_string(&self).map_err(|_| "Couldn't serialize message")?;
      output
        .write(contents.as_bytes())
        .expect("Failed to write response");
      output.write_all(b"\n").expect("Failed to write newline");
      Ok(())
    }
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
    local_msg_id: usize,
  ) -> Result<res::MaelstromResponse<ServiceType>, String>;
}

pub enum Event<ServiceType: Serialize + DeserializeOwned + Send> {
  IOEvent(req::MaelstromRequest<ServiceType>),
  GossipEvent,
}

pub trait GossipMessagePump<ServiceType>
where
ServiceType: Serialize + DeserializeOwned + Send + 'static,
{
  fn get_duration(&self) -> std::time::Duration;
  fn tx(&self) -> std::sync::mpsc::Sender<Event<ServiceType>>;
}

pub trait CooperativeNode<ServiceType>: Node<ServiceType>
where
  ServiceType: DeserializeOwned + Serialize + Send,
{
  fn setup_sidechannel_thread(
    &mut self,
    _tx: std::sync::mpsc::Sender<Event<ServiceType>>,
  ) -> Option<std::thread::JoinHandle<()>> {
    None
  }

  fn process_event(&mut self, msg: Event<ServiceType>, local_msg_id: usize, comms: &mut StdoutLock);
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
      msg_id: Some(msg_id),
      response_type: MaelstromService::InitOk,
      in_reply_to: Some(in_reply_to),
    },
  }
}

fn exit_threads<T1, T2>(
  io: std::thread::JoinHandle<T1>,
  gossip: Option<std::thread::JoinHandle<T2>>,
) -> Result<(), String> {
  io.join()
    .map_err(|e| format!("IO Thread join failed. Cause:\n\t {e:#?}"))?;
  let Some(gossip) = gossip else { return Ok(()) };
  gossip
    .join()
    .map_err(|e| format!("Gossip thread join failed. Cause:\n\t {e:#?}"))?;
  Ok(())
}

fn wait_for_init_and_respond(mut stdin: StdinLock) -> Result<Initialize, String> {
  let mut buf = String::with_capacity(512);
  stdin
    .read_line(&mut buf)
    .map_err(|_| "Failed to read init packet")?;
  eprintln!("first packet: {}", &buf);
  let init: req::MaelstromRequest<Initialize> = serde_json::from_str(&buf).map_err(|e| {
    format!("Init request always required but failed to parse: {e}. Contents: {buf}")
  })?;

  let init_respose_ = init_response(
    init.body.msg_id.expect("Init request ill-formed"),
    1,
    init.dest,
    init.src,
  );
  let msg =
    serde_json::to_string(&init_respose_).map_err(|_| "Failed to serialize init resposne")?;
  let mut stdout: StdoutLock = std::io::stdout().lock();
  stdout
    .write(msg.as_bytes())
    .expect("Failed to send init response");
  stdout.write_all(b"\n").expect("");

  Ok(init.body.data)
}

pub fn start_service<N, ServiceType>(mut node: N) -> Result<(), String>
where
  N: CooperativeNode<ServiceType>,
  ServiceType: Serialize + DeserializeOwned + Send + 'static,
{
  let (tx, rx) = std::sync::mpsc::channel::<Event<ServiceType>>();

  let init = wait_for_init_and_respond(std::io::stdin().lock())?;
  node.init(init);

  if !node.is_initialized() {
    panic!("Node initialized with faulty settings");
  }

  let node_tx_ = tx.clone();
  let gossip_thread = node.setup_sidechannel_thread(node_tx_);

  let io_tx = tx.clone();
  let input_notifier_thread = std::thread::spawn(move || -> Result<(), String> {
    let stdin = std::io::stdin().lock();
    let mut reader = BufReader::new(stdin);
    let mut buf = String::with_capacity(512);
    loop {
      reader.read_line(&mut buf).expect("Failed to read input");
      let req: req::MaelstromRequest<ServiceType> =
        req::parse_request(&buf).map_err(|e| format!("Failed to parse request: {e:?}"))?;
      io_tx
        .send(Event::IOEvent(req))
        .map_err(|e| format!("Failed to send IO Event {e:#}"))?;
      buf.clear();
    }
  });

  let mut stdout: StdoutLock = std::io::stdout().lock();
  let mut msg_id = 2..usize::MAX;
  loop {
    match rx.recv() {
      Ok(evt) => {
        node.process_event(
          evt,
          msg_id.next().expect("failed to get new msg_id"),
          &mut stdout,
        );
      }
      Err(e) => {
        exit_threads(input_notifier_thread, gossip_thread)?;
        return Err(format!("Application Level Error: {e:#}"));
      }
    }
  }
}

pub fn start_maelstrom_service_node<N, ServiceType>(mut node: N) -> Result<(), String>
where
  N: Node<ServiceType>,
  ServiceType: Serialize + DeserializeOwned,
{
  let init = wait_for_init_and_respond(std::io::stdin().lock())?;
  node.init(init);

  if !node.is_initialized() {
    panic!("Node initialized with faulty settings");
  }

  let mut stdout: StdoutLock = std::io::stdout().lock();
  let mut reader = BufReader::new(std::io::stdin().lock());
  let mut msg_id = 2..usize::MAX;
  let mut buf = String::with_capacity(512);
  loop {
    buf.clear();
    reader.read_line(&mut buf).expect("Failed to read input");
    let req: req::MaelstromRequest<ServiceType> =
      req::parse_request(&buf).map_err(|e| format!("Failed to parse request: {e:?}"))?;
    node
      .process_message(req, msg_id.next().expect("Ran out of message id's"))?
      .take_send(&mut stdout)?;
  }
}
