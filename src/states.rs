pub enum IncomingConnectionState {
    Connect,
    ServerFull,
    Close(String),
    None,
}
