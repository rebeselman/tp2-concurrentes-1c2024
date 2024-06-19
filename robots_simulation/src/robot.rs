use std::collections::{HashMap, HashSet};
use std::io;
use std::net::UdpSocket;
use std::thread;
use std::time::Duration;
use rand::prelude::{IndexedMutRandom, IndexedRandom};
use rand::Rng;
use robots_simulation::{ContainerType, IceCreamFlavor, Item, Request, Response};

pub fn generate_ice_cream_orders(num_orders: usize) -> Vec<Vec<Item>> {
    let mut rng = rand::thread_rng();

    let flavors = vec![
        IceCreamFlavor::Chocolate,
        IceCreamFlavor::Strawberry,
        IceCreamFlavor::Vanella,
        IceCreamFlavor::Mint,
        IceCreamFlavor::Lemon,
    ];

    let mut container_types:Vec<ContainerType> = vec![
        ContainerType::Cup,
        ContainerType::Cone,
        ContainerType::OneKilo,
        ContainerType::HalfKilo,
        ContainerType::QuarterKilo,
    ];

    let mut orders = Vec::new();

    for _ in 0..num_orders {
        let num_items = rng.gen_range(1..=3);
        let mut items = Vec::new();

        for _ in 0..num_items {
            let container: ContainerType = container_types.choose_mut(&mut rng).unwrap().clone();
            let units = rng.gen_range(1..=3);
            let num_flavors = match container {
                ContainerType::Cup | ContainerType::Cone => 2,
                ContainerType::OneKilo => 3,
                ContainerType::HalfKilo | ContainerType::QuarterKilo => 2,
            };

            let flavors_item: Vec<IceCreamFlavor> = flavors.choose_multiple(&mut rng, num_flavors).cloned().collect();
            let flavor_status = flavors_item.iter().map(|flavor| (flavor.clone(), false)).collect();
            items.push(Item {
                container,
                units,
                flavors: flavors_item,
                flavor_status, // Initialize flavor status
                is_completed: false,
            });
        }

        orders.push(items);
    }

    orders
}


fn main() {
    let robot_id= build();

    let addr = format!("0.0.0.0:809{robot_id}");
    let socket = UdpSocket::bind(addr.clone()).expect("Failed to bind socket");
    socket.set_read_timeout(Some(Duration::from_secs(1))).expect("Failed to set read timeout");

    let server_addr = "127.0.0.1:8080";
    // let mut buf = [0; 1024];
    loop {
        let mut orders = generate_ice_cream_orders(1);
        let mut order = orders.first_mut().expect("Order isn't valid");
        // let (len, addr) = socket.recv_from(&mut buf).unwrap();
        // println!("Received message from {}", addr);
        // Get flavors needed for all items in order
        make_order(robot_id, &socket, server_addr, order);
        println!("Order completed: {:?}", order);
    }
}

fn make_order(robot_id: usize, socket: &UdpSocket, server_addr: &str, order: &mut Vec<Item>) {
    let flavors: HashSet<IceCreamFlavor> = order.iter().flat_map(|item| item.flavors.clone()).collect();
    println!("Flavors needed: {:?}", flavors);
    let mut flavors_needed: HashMap<IceCreamFlavor, bool> = flavors.iter().map(|flavor| (flavor.clone(), false)).collect();

    // Se podría cambiar esto a que la orden esté completa
    while flavors_needed.iter().any(|(_, completed)| !completed) {
        // Filter flavors needed based on the values of the map
        let flavors_to_request: Vec<IceCreamFlavor> = flavors_needed.iter().filter(|(_, completed)| !**completed).map(|(flavor, _)| flavor.clone()).collect();
        println!("Requesting flavors: {:?}", flavors_to_request);
        let request = Request::SolicitarAcceso {
            robot_id,
            gustos: flavors_to_request
        };

        make_request(&socket, server_addr, &request);

        let mut buf = [0; 1024];
        match socket.recv_from(&mut buf) {
            Ok((amt, _)) => {
                read_coordinator_answer(robot_id, &socket, server_addr, &mut buf, &amt, order, &mut flavors_needed);
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                println!("No response from coordinator, retrying...");
            }
            Err(e) => {
                println!("Failed to receive response: {}", e);
            }
        }
    }
}

fn build() -> usize {
    let args: Vec<String> = std::env::args().collect();
    let robot_id: usize = args[1].parse().expect("Invalid robot ID");
    robot_id
}

fn read_coordinator_answer(robot_id: usize, socket: &UdpSocket, server_addr: &str, buf: &mut [u8; 1024], amt: &usize, order: &mut Vec<Item>, x: &mut HashMap<IceCreamFlavor, bool>) {
    let response: Response = serde_json::from_slice(&buf[..*amt]).expect("Failed to parse response");
    match response {
        Response::AccesoConcedido(flavor) => {
            add_flavor_to_ice_cream(&flavor, robot_id);
            send_release_request(&flavor, robot_id, socket, server_addr);
            mark_flavor_as_completed(order, &flavor);
            if let Some(completed) = x.get_mut(&flavor) {
                *completed = true;
            }
        },
        Response::AccesoDenegado(err) => {
            println!("[Robot {}] No pudé acceder a nigún contenedor {}", robot_id, err);
            // Use recv_from socket blocking
            loop {
                let mut buf = [0; 1024];
                match socket.recv_from(&mut buf) {
                    Ok((amt, _)) => {
                        read_coordinator_answer(robot_id, socket, server_addr, &mut buf, &amt, order, x);
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        continue;
                    }
                    Err(e) => {
                        println!("Failed to receive response: {}", e);
                        break;
                    }
                }
            }
        },
        _ => {}
    }
}

fn send_release_request(sabor: &IceCreamFlavor, robot_id: usize, socket: &UdpSocket, server_addr: &str) {
    println!("[Robot {}] libero contenedor de {:?}", robot_id, sabor);
    let release_request = Request::LiberarAcceso {
        robot_id,
        gusto: sabor.clone()
    };
    let release_request = serde_json::to_vec(&release_request).expect("Failed to serialize release request");
    socket.send_to(&release_request, server_addr).expect("Failed to send release request");
    let mut buf = [0; 1024];
    let mut received_ack = false;
    while !received_ack {
        let (len, _addr) = socket.recv_from(&mut buf).expect("Failed to receive response");
        let response: Response = serde_json::from_slice(&buf[..len]).expect("Failed to parse response");
        received_ack = match response {
            Response::ACK => {
                println!("[Robot {}] ACK recibido", robot_id);
                break
            },
            _ => {
                println!("[Robot {}] No se recibió ACK", robot_id);
                false
            }
        };
    }
    println!("[Robot {}] Liberación de contenedor de {:?} completada", robot_id, sabor);

}

fn add_flavor_to_ice_cream(sabor: &IceCreamFlavor, robot_id: usize) {
    println!("[Robot {}] accedo al contenedor de {:?}", robot_id, sabor);
    thread::sleep(Duration::from_secs(5)); // Simula tiempo de trabajo
}

fn mark_flavor_as_completed(order: &mut Vec<Item>, sabor: &IceCreamFlavor) {
    for item in order.iter_mut() {

        if let Some(completed) = item.flavor_status.get_mut(sabor) {
            *completed = true;
        }
        if item.flavor_status.values().all(|&completed| completed) {
            item.is_completed = true;
            println!("Item completed: {:?}", item);
        }
    }
}

fn make_request(socket: &UdpSocket, server_addr: &str, request: &Request) {
    let request = serde_json::to_vec(&request).expect("Failed to serialize request");
    socket.send_to(&request, server_addr).expect("Failed to send request");
}
