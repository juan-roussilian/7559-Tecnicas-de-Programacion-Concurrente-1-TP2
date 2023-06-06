#[derive(Debug, PartialEq, Eq)]
enum Status {
    Connected,
    Disconnected,
}

pub struct ConnectionStatus {
    next: Status,
    prev: Status,
}

impl ConnectionStatus {
    pub fn new() -> ConnectionStatus {
        ConnectionStatus {
            next: Status::Disconnected,
            prev: Status::Disconnected,
        }
    }

    pub fn is_online(&self) -> bool {
        self.next == Status::Connected && self.prev == Status::Connected
    }

    pub fn is_prev_online(&self) -> bool {
        self.prev == Status::Connected
    }

    pub fn set_prev_online(&mut self) {
        self.prev = Status::Connected;
    }
}
