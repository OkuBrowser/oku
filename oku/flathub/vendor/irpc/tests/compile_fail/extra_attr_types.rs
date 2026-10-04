use irpc::rpc_requests;

#[rpc_requests(Service, Msg)]
enum Enum {
    #[rpc(tx = NoSender, rx = NoReceiver, fnord = Foo)]
    A(u8),
}

fn main() {}