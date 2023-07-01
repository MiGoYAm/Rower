pub const ADDRESS: &str = "127.0.0.1:25565";

pub const THRESHOLD: i32 = 256;
pub const ONLINE: bool = false;

pub const SERVERS: [Server; 1] = [
    Server {
        name: "main",
        ip: "127.0.0.1:25566"
    }
];

pub struct Server {
    pub name: &'static str,
    pub ip: &'static str
}
