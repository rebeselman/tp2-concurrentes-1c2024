use std::collections::HashMap;
use std::sync::Arc;

use actix::{Actor, AsyncContext, Context, Handler};
use tokio::net::UdpSocket;
use tokio::sync::Mutex;

use robots_simulation::{ContainerType, IceCreamFlavor, Item, Request, Response};

struct Coordinator {
    containers: HashMap<String, Arc<Mutex<bool>>>,
    socket: Arc<UdpSocket>,
}

impl Actor for Coordinator {
    type Context = Context<Self>;
}

impl Handler<Request> for Coordinator {
    type Result = ();

    fn handle(&mut self, msg: Request, ctx: &mut Self::Context) {
        println!("Received message: {:?}", msg);
        let socket = self.socket.clone();

        match msg {
            Request::SolicitarAcceso { robot_id, gusto, robot_addr } => {
                println!("Robot {} is requesting access to container {}", robot_id, gusto);
                let arc = self.containers.get(&gusto).unwrap().clone();
                let fut = async move {
                    let mut access = arc.lock().await;
                    if !*access {
                        *access = true;
                        Response::AccesoConcedido
                    } else {
                        Response::AccesoDenegado("Container already in use".into())
                    }
                };

                tokio::spawn(async move {
                    let response = fut.await;
                    let response = serde_json::to_vec(&response).unwrap();
                    println!("Sending response to robot {}", robot_addr);
                    socket.send_to(&response, robot_addr).await.unwrap();
                });
            }
            Request::LiberarAcceso { robot_id, gusto, robot_addr } => {
                let arc = self.containers.get(&gusto).unwrap().clone();
                tokio::spawn(async move {
                    let mut access = arc.lock().await;
                    *access = false;
                    println!("Access released for container {}", gusto);
                });
            }
        }
    }
}

#[actix_rt::main]
async fn main() {
    let socket = UdpSocket::bind("127.0.0.1:8080").await.unwrap();
    let socket = Arc::new(socket);

    let gustos = vec!["vainilla".to_string(), "chocolate".to_string(), "frutilla".to_string(), "limón".to_string(), "menta".to_string(), "dulce de leche".to_string(), "granizado".to_string(), "banana split".to_string(), "tramontana".to_string(), "chocolate amargo".to_string(), "menta granizada".to_string(), "americana".to_string()];
    let containers: HashMap<_, _> = gustos.into_iter().map(|gusto| (gusto, Arc::new(Mutex::new(false)))).collect();

    let coordinator = Coordinator {
        containers,
        socket: socket.clone(),
    }.start();

    let mut buf = [0; 1024];
    loop {
        let (len, addr) = socket.recv_from(&mut buf).await.unwrap();
        println!("Received message from {}", addr);
        let msg: Request = serde_json::from_slice(&buf[..len]).unwrap();

        coordinator.send(msg).await.unwrap();
    }
}
