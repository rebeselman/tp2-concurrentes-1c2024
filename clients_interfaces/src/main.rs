use actix::prelude::*;
/// Genera órdenes de pedido basadas en un archivo simulado.
/// Simula varias instancias de pantallas de pedidos.
const _NUMBER_SCREENS: u8 = 3;



struct MyActor {
    count: usize,
}

impl Actor for MyActor {
    type Context = Context<Self>;
}

#[derive(Message)]
#[rtype(result = "usize")]
struct Ping(usize);

impl Handler<Ping> for MyActor {
    type Result = usize;

    fn handle(&mut self, msg: Ping, _ctx: &mut Context<Self>) -> Self::Result {
        self.count += msg.0;

        self.count
    }
}

#[actix::main]
async fn main() {
    // start new actor
    let addr = MyActor { count: 10 }.start();

    // send message and get future for result
    let res = addr.send(Ping(10)).await;

    // handle() returns tokio handle
    println!("RESULT: {}", res.unwrap() == 20);
    println!("jejej");
    // stop system and exit
    System::current().stop();
   
}