pub fn id_to_coffee_port(id: usize) -> String {
    let port = id + 20000;
    port.to_string()
}

pub fn id_to_server_port(id: usize) -> String {
    let port = id + 10000;
    port.to_string()
}

pub fn id_to_address(id: usize) -> String {
    let port = id + 10000;
    "127.0.0.1:".to_owned() + &*port.to_string()
}
