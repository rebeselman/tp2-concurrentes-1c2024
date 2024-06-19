use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::Arc;

use actix::{Actor, Context, Handler};
use tokio::net::UdpSocket;
use tokio::sync::Mutex;

use robots_simulation::{ IceCreamFlavor, Request, Response, AccessRequest, ReleaseRequest};

#[derive(Clone)]
struct Coordinator {
    containers: HashMap<IceCreamFlavor, Arc<Mutex<bool>>>,
    socket: Arc<UdpSocket>,
    queue: Arc<Mutex<VecDeque<(Vec<IceCreamFlavor>, usize, SocketAddr)>>>,
}

impl Coordinator {
    fn new(socket: Arc<UdpSocket>) -> Self {
        let flavors = vec![
            IceCreamFlavor::Chocolate,
            IceCreamFlavor::Strawberry,
            IceCreamFlavor::Vanella,
            IceCreamFlavor::Mint,
            IceCreamFlavor::Lemon,
        ];

        let containers = flavors.iter()
            .map(|gusto| (gusto.clone(), Arc::new(Mutex::new(false))))
            .collect();

        Coordinator {
            containers,
            socket,
            queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    fn process_queue(&mut self) {
        println!("Processing queue");
        let queue = self.queue.clone();
        let mut _self = self.clone();
        tokio::spawn(async move {
            let mut queue = queue.lock().await;
            while let Some((gustos, robot_id, robot_addr)) = queue.pop_front() {
                _self.check_if_flavor_available(robot_id, &gustos, robot_addr);
            }
        });
    }

    fn check_if_flavor_available(&mut self, robot_id: usize, gustos: &Vec<IceCreamFlavor>, addr: SocketAddr) {
        let socket = self.socket.clone();
        let containers = self.containers.clone();
        let queue = self.queue.clone();
        let gustos = gustos.clone(); // Clone gustos here
        tokio::spawn(async move {
            for gusto in &gustos {
                println!("Robot {} is requesting access to container {:?}", robot_id, gusto);
                let arc = containers.get(&gusto).unwrap().clone();
                let fut = async move {
                    let mut access = arc.lock().await;
                    if !*access {
                        *access = true;
                        Response::AccesoConcedido(gusto.clone())
                    } else {
                        Response::AccesoDenegado("Container already in use".into())
                    }
                };
                let response = fut.await;
                if let Response::AccesoConcedido(_) = response {
                    send_response(&socket, response, addr).await;
                    return true;
                }
            }
            let response = Response::AccesoDenegado("All requested containers are in use".into());
            let mut queue = queue.lock().await;
            queue.push_back((gustos.to_vec(), robot_id, addr));
            send_response(&socket, response, addr).await;
            false
        });
    }
}

async fn send_response(socket: &Arc<UdpSocket>, response: Response, addr: SocketAddr) {
    let response = serde_json::to_vec(&response).unwrap();
    println!("Sending response to robot {}", addr);
    socket.send_to(&response, addr).await.unwrap();
}

impl Actor for Coordinator {
    type Context = Context<Self>;
}

impl Handler<AccessRequest> for Coordinator {
    type Result = ();

    fn handle(&mut self, msg: AccessRequest, _ctx: &mut Self::Context) {
        let AccessRequest { robot_id, gustos, addr } = msg;
        self.check_if_flavor_available(robot_id, &gustos, addr);
    }
}

impl Handler<ReleaseRequest> for Coordinator {
    type Result = ();

    fn handle(&mut self, msg: ReleaseRequest, _ctx: &mut Self::Context) {
        let ReleaseRequest { robot_id, gusto, addr } = msg;
        let socket = self.socket.clone();

        println!("Releasing access for container {:?} from {:?}", gusto, robot_id);
        let arc = self.containers.get(&gusto).unwrap().clone();
        tokio::spawn(async move {
            let mut access = arc.lock().await;
            *access = false;
            println!("Access released for container {:?} from {:?}", gusto, robot_id);
            let response = Response::ACK;
            send_response(&socket, response, addr).await;
        });
        self.process_queue();
    }
}

#[actix_rt::main]
async fn main() {
    let socket = UdpSocket::bind("127.0.0.1:8080").await.unwrap();
    let socket = Arc::new(socket);
    let coordinator = Coordinator::new(socket.clone()).start();

    let mut buf = [0; 1024];
    loop {
        let (len, addr) = socket.recv_from(&mut buf).await.unwrap();
        println!("Received message from {}", addr);
        let msg: Request = serde_json::from_slice(&buf[..len]).unwrap();

        match msg {
            Request::SolicitarAcceso { robot_id, gustos } => {
                let access_request = AccessRequest {
                    robot_id,
                    gustos,
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
