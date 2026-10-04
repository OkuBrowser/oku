use irpc::rpc_requests;

#[rpc_requests(Service, Msg)]
enum Enum {
    #[rpc(fnord = Bla)]
    A(u8),
}

fn main() {}