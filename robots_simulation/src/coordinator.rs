use std::collections::HashMap;
use std::sync::Arc;

use actix::{Actor, AsyncContext, Context, Handler};
use tokio::net::UdpSocket;
use tokio::sync::Mutex;

use robots_simulation::{ IceCreamFlavor, Request, Response, AccessRequest, ReleaseRequest};

struct Coordinator {
    containers: HashMap<IceCreamFlavor, Arc<Mutex<bool>>>,
    socket: Arc<UdpSocket>,
}

impl Actor for Coordinator {
    type Context = Context<Self>;
}

impl Handler<AccessRequest> for Coordinator {
    type Result = ();

    fn handle(&mut self, msg: AccessRequest, _ctx: &mut Self::Context) {
        let socket = self.socket.clone();
        let AccessRequest { robot_id, gusto, addr } = msg;

        println!("Robot {} is requesting access to container {:?}", robot_id, gusto);
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
            println!("Sending response to robot {}", addr);
            socket.send_to(&response, addr).await.unwrap();
        });
    }
}

impl Handler<ReleaseRequest> for Coordinator {
    type Result = ();

    fn handle(&mut self, msg: ReleaseRequest, _ctx: &mut Self::Context) {
        let ReleaseRequest { robot_id: _robot_id, gusto, addr: _addr } = msg;

        println!("Releasing access for container {:?}", gusto);
        let arc = self.containers.get(&gusto).unwrap().clone();
        tokio::spawn(async move {
            let mut access = arc.lock().await;
            *access = false;
            println!("Access released for container {:?}", gusto);
        });
    }
}

#[actix_rt::main]
async fn main() {
    let socket = UdpSocket::bind("127.0.0.1:8080").await.unwrap();
    let socket = Arc::new(socket);
    let flavors = vec![
        IceCreamFlavor::Chocolate,
        IceCreamFlavor::Strawberry,
        IceCreamFlavor::Vanella,
        IceCreamFlavor::Mint,
        IceCreamFlavor::Lemon,
    ];

    let containers: HashMap<_, _> = flavors.into_iter().map(|gusto| (gusto, Arc::new(Mutex::new(false)))).collect();

    let coordinator = Coordinator {
        containers,
        socket: socket.clone(),
    }.start();

    let mut buf = [0; 1024];
    loop {
        let (len, addr) = socket.recv_from(&mut buf).await.unwrap();
        println!("Received message from {}", addr);
        let msg: Request = serde_json::from_slice(&buf[..len]).unwrap();

        match msg {
            Request::SolicitarAcceso { robot_id, gusto } => {
                let access_request = AccessRequest {
                    robot_id,
                    gusto,
                    addr,
                };
                coordinator.send(access_request).await.unwrap();
            }
            Request::LiberarAcceso { robot_id, gusto } => {
                let release_request = ReleaseRequest {
                    robot_id,
                    gusto,
                    addr,
                };
                coordinator.send(release_request).await.unwrap();
            }
        }
    }
}
