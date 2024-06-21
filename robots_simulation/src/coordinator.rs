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

        let containers = flavors.into_iter()
            .map(|flavor| (flavor, Arc::new(Mutex::new(false))))
            .collect();

        Coordinator {
            containers,
            socket,
            queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    async fn process_queue(&self) {
        println!("Processing queue");
        let mut queue = self.queue.lock().await;
        while let Some((flavors, robot_id, robot_addr)) = queue.pop_front() {
            self.check_if_flavor_available(robot_id, &flavors, robot_addr).await;
        }
    }

    async fn check_if_flavor_available(&self, robot_id: usize, flavors: &Vec<IceCreamFlavor>, addr: SocketAddr) {
        for flavor in flavors {
            let container = self.containers.get(flavor).unwrap().clone();
            let mut access = container.lock().await;
            let response = if !*access {
                *access = true;
                println!("Robot {} is requesting access to container {:?}", robot_id, flavor);
                Response::AccesoConcedido(flavor.clone())
            } else {
                Response::AccesoDenegado("Container already in use".into())
            };
            send_response(&self.socket, &response, addr).await;
            if matches!(response, Response::AccesoConcedido(_)) {
                return;
            }
        }
        let response = Response::AccesoDenegado("All requested containers are in use".into());
        self.queue.lock().await.push_back((flavors.clone(), robot_id, addr));
        send_response(&self.socket, &response, addr).await;
    }
}

async fn send_response(socket: &Arc<UdpSocket>, response: &Response, addr: SocketAddr) {
    let response = serde_json::to_vec(response).unwrap();
    println!("Sending response to robot {}", addr);
    socket.send_to(&response, addr).await.unwrap();
}

impl Actor for Coordinator {
    type Context = Context<Self>;
}

impl Handler<AccessRequest> for Coordinator {
    type Result = ();

    fn handle(&mut self, msg: AccessRequest, _ctx: &mut Self::Context) {
        let AccessRequest { robot_id, flavors, addr } = msg;
        let flavors = flavors.clone();
        let this = self.clone();
        actix_rt::spawn(async move {
            this.check_if_flavor_available(robot_id, &flavors, addr).await;
        });    }
}

impl Handler<ReleaseRequest> for Coordinator {
    type Result = ();

    fn handle(&mut self, msg: ReleaseRequest, _ctx: &mut Self::Context) {
        let ReleaseRequest { robot_id, flavor, addr } = msg;
        let container = self.containers.get(&flavor).unwrap().clone();
        let socket = self.socket.clone();
        let this = self.clone();
        println!("Releasing access for container {:?} from {:?}", flavor, robot_id);
        actix_rt::spawn(async move {
            *container.lock().await = false;
            send_response(&socket, &Response::ACK, addr).await;
            println!("Access released for container {:?} from {:?}", flavor, robot_id);
        });
        actix_rt::spawn(async move {
            this.process_queue().await;
        });
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
            Request::SolicitarAcceso { robot_id, flavors } => {
                let access_request = AccessRequest {
                    robot_id,
                    flavors,
                    addr,
                };
                coordinator.send(access_request).await.unwrap();
            }
            Request::LiberarAcceso { robot_id, flavor } => {
                let release_request = ReleaseRequest {
                    robot_id,
                    flavor,
                    addr,
                };
                coordinator.send(release_request).await.unwrap();
            }
        }
    }
}
