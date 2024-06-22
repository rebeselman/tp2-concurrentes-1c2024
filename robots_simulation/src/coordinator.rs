use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::Arc;
use actix::{Actor, Addr, Context, Handler};
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use robots_simulation::items::{AccessRequest, IceCreamFlavor, Order, OrderState, OrderRequest, ReleaseRequest, RequestToCoordinator, Response, OrderFinished, CommitReceived, OrderStatus};
use robots_simulation::items::OrderStatus::{Completed, CompletedButNotCommited, Pending};

#[derive(Clone)]
struct Coordinator {
    containers: HashMap<IceCreamFlavor, Arc<Mutex<bool>>>,
    socket: Arc<UdpSocket>,
    queue: Arc<Mutex<VecDeque<(Vec<IceCreamFlavor>, usize, SocketAddr)>>>,
    order_queue: Arc<Mutex<VecDeque<Order>>>,
    robot_states: Arc<Mutex<HashMap<usize, bool>>>, // Added to track robot states
    orders: HashMap<usize, OrderState>
}

const NUMBER_ROBOTS: usize = 2;

impl Coordinator {
    fn new(socket: Arc<UdpSocket>) -> Self {
        let flavors = vec![
            IceCreamFlavor::Chocolate,
            IceCreamFlavor::Strawberry,
            IceCreamFlavor::Vanilla,
            IceCreamFlavor::Mint,
            IceCreamFlavor::Lemon,
        ];

        let containers = flavors.into_iter()
            .map(|flavor| (flavor, Arc::new(Mutex::new(false))))
            .collect();

        let robot_states = (0..NUMBER_ROBOTS).map(|id| (id, false)).collect(); // Initialize robot states

        Coordinator {
            containers,
            socket,
            queue: Arc::new(Mutex::new(VecDeque::new())),
            order_queue: Arc::new(Default::default()),
            robot_states: Arc::new(Mutex::new(robot_states)), // Add robot_states
            orders: HashMap::new()
        }
    }

    async fn assign_order_to_robot(&self, order: Order) {
        let mut robot_states = self.robot_states.lock().await;
        if let Some((&robot_id, &available)) = robot_states.iter().find(|&(_, &available)| available == false) {
            robot_states.insert(robot_id, true); // Mark robot as busy
            let msg = serde_json::to_vec(&Response::AssignOrder { robot_id, order }).unwrap();
            self.socket.send_to(&msg, format!("0.0.0.0:809{}", robot_id)).await.unwrap();
            println!("[COORDINATOR] Order assigned to robot {}", robot_id);
        } else {
            // All robots are busy, handle accordingly (e.g., add to a queue)
            self.order_queue.lock().await.push_back(order);
        }
    }

    fn order_completed(&mut self, order_id: usize) {
        if let Some(order_state) = self.orders.get_mut(&order_id) {
            if order_state.status == OrderStatus::CommitReceived {
                order_state.status = OrderStatus::Completed;
            }
            else if order_state.status == Pending {
                order_state.status = CompletedButNotCommited;
            }
        }
    }

    async fn process_queue(&self) {
        println!("[COORDINATOR] Checking if there are access requests in the queue");
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
                println!("[COORDINATOR] Robot {} is requesting access to container {:?}", robot_id, flavor);
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
    fn send_finish_message(&self, order_id: usize, addr: &SocketAddr) {
        let socket = self.socket.clone();
        let addr = addr.clone();
        actix_rt::spawn(async move {
            let message = format!("{}\nfinished", order_id).into_bytes();
            socket.send_to(&message, &addr).await.unwrap();
        });
    }
}

async fn send_response(socket: &Arc<UdpSocket>, response: &Response, addr: SocketAddr) {
    let response = serde_json::to_vec(response).unwrap();
    socket.send_to(&response, addr).await.unwrap();
}

impl Actor for Coordinator {
    type Context = Context<Self>;
}

impl Handler<OrderRequest> for Coordinator {
    type Result = ();

    fn handle(&mut self, msg: OrderRequest, _ctx: &mut Self::Context) {
        let OrderRequest { order, screen_addr } = msg;
        let order = order.clone();
        let addr = screen_addr.clone();
        let this = self.clone();
        self.orders.insert(order.order_id, OrderState {
            order: order.clone(),
            status: Pending,
            screen_addr,
        });
        actix_rt::spawn(async move {
            let message = format!("{}\nready", order.order_id).into_bytes();
            this.socket.send_to(&message, &addr).await.unwrap();
            this.assign_order_to_robot(order).await;
        });
    }
}

impl Handler<CommitReceived> for Coordinator {
    type Result = ();

    fn handle(&mut self, msg: CommitReceived, _ctx: &mut Self::Context) {
        let CommitReceived { robot_id, order } = msg;
        let mut send_finished = false;
        let mut addr: SocketAddr = SocketAddr::new([0, 0, 0, 0].into(), 0);
        if let Some(order_state) = self.orders.get_mut(&order.order_id) {
            if order_state.status == Pending {
                order_state.status = OrderStatus::CommitReceived;
            }
            else if order_state.status == CompletedButNotCommited {
                order_state.status = Completed;
                send_finished = true;
                addr = order_state.screen_addr.clone();
            }
        }
        // If address is not null
        if send_finished && addr != SocketAddr::new([0, 0, 0, 0].into(), 0) {
            self.send_finish_message(order.order_id, &addr);
        }
    }
}

impl Handler<AccessRequest> for Coordinator {
    type Result = ();

    fn handle(&mut self, msg: AccessRequest, _ctx: &mut Self::Context) {
        let AccessRequest { robot_id, flavors, addr } = msg;
        let flavors = flavors.clone();
        let this = self.clone();
        actix_rt::spawn(async move {
            this.check_if_flavor_available(robot_id, &flavors, addr).await;
        });
    }
}

impl Handler<ReleaseRequest> for Coordinator {
    type Result = ();

    fn handle(&mut self, msg: ReleaseRequest, _ctx: &mut Self::Context) {
        let ReleaseRequest { robot_id, flavor, addr } = msg;
        let container = self.containers.get(&flavor).unwrap().clone();
        let socket = self.socket.clone();
        let this = self.clone();
        println!("[COORDINATOR] Releasing access for container {:?} from {:?}", flavor, robot_id);
        actix_rt::spawn(async move {
            *container.lock().await = false;
            send_response(&socket, &Response::ACK, addr).await;
        });
        actix_rt::spawn(async move {
            this.process_queue().await;
        });
    }
}

impl Handler<OrderFinished> for Coordinator {
    type Result = ();

    fn handle(&mut self, msg: OrderFinished, _ctx: &mut Self::Context) {
        let OrderFinished { robot_id, order } = msg;
        let order_id = order.order_id;
        let mut this = self.clone();
        actix_rt::spawn(async move {
            this.order_completed(order_id);
            if let Some(order_state) = this.orders.get(&order_id) {
                if order_state.status == Completed {
                    this.send_finish_message(order_id, &order_state.screen_addr);
                }
            }
            let mut robot_states = this.robot_states.lock().await;
            robot_states.insert(robot_id, false);
        });
    }
}

async fn process_access_message(coordinator: &Addr<Coordinator>, msg: RequestToCoordinator, addr: SocketAddr) {
    match msg {
        RequestToCoordinator::SolicitarAcceso { robot_id, flavors } => {
            let access_request = AccessRequest {
                robot_id,
                flavors,
                addr,
            };
            coordinator.send(access_request).await.unwrap();
        }
        RequestToCoordinator::LiberarAcceso { robot_id, flavor } => {
            let release_request = ReleaseRequest {
                robot_id,
                flavor,
                addr,
            };
            coordinator.send(release_request).await.unwrap();
        }
        RequestToCoordinator::OrdenTerminada { robot_id, order } => {
            println!("[COORDINATOR] Order completed by robot {}", robot_id);
            let finished_message = OrderFinished {
                robot_id,
                order,
            };
            coordinator.send(finished_message).await.unwrap();
        }
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
        let received_message = String::from_utf8_lossy(&buf[..len]);
        let mut parts = received_message.split('\n');
        let message_type = parts.next().unwrap();
        println!("Received message: {}", message_type);

        match message_type {
            "prepare" => {
                let order: Order = serde_json::from_str(&parts.next().unwrap()).unwrap();
                println!("[COORDINATOR] Received prepare message for order: {}", order.order_id);
                let order_request = OrderRequest {
                    order,
                    screen_addr: addr,
                };
                coordinator.send(order_request).await.unwrap();
            }
            "commit" => {
                let order: Order = serde_json::from_str(&parts.next().unwrap()).unwrap();
                println!("[COORDINATOR] Received commit message for order: {}", order.order_id);
                let commit_received = CommitReceived {
                    robot_id: 0,
                    order,
                };
                coordinator.send(commit_received).await.unwrap();
            }
            "access" => {
                let msg: RequestToCoordinator = serde_json::from_str(&parts.next().unwrap()).unwrap();
                process_access_message(&coordinator, msg, addr).await;
            },
            _ => {}
        };
    }
}
