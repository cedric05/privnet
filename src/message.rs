use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerResponse {
    Ok { end_user_port: u16 },
    Pong { seq: u32 },
    NewConnection { client_connect_port: u16 },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientRequest {
    Ping { seq: u32 },
    ClientConnect,
}
