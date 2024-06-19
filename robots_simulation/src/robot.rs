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
    (0..num_orders).map(|_| {
        (0..rng.gen_range(1..=3)).map(|_| {
            let container = container_types.choose_mut(&mut rng).unwrap().clone();
            let units = rng.gen_range(1..=3);
            let num_flavors = match container {
                ContainerType::Cup | ContainerType::Cone => 2,
                ContainerType::OneKilo => 3,
                ContainerType::HalfKilo | ContainerType::QuarterKilo => 2,
            };
            let flavors_item: Vec<IceCreamFlavor> = flavors.choose_multiple(&mut rng, num_flavors).cloned().collect();
            Item {
                container,
                units,
                flavors: flavors_item.clone(),
                flavor_status: flavors_item.iter().map(|flavor| (flavor.clone(), false)).collect(),
                is_completed: false,
            }
        }).collect()
    }).collect()
}

fn main() -> io::Result<()> {
    let robot_id = std::env::args().nth(1).unwrap().parse().unwrap();
    let addr = format!("0.0.0.0:809{}", robot_id);
    let socket = UdpSocket::bind(&addr)?;
    socket.set_read_timeout(Some(Duration::from_secs(1))).expect("Failed to set read timeout");
    let server_addr = "127.0.0.1:8080";
    loop {
        let mut orders = generate_ice_cream_orders(1);
        let mut order = orders.first_mut().expect("Order isn't valid");
        // let (len, addr) = socket.recv_from(&mut buf).unwrap();
        // println!("Received message from {}", addr);
        // Get flavors needed for all items in order
        make_order(robot_id, &socket, server_addr, order)?;
        println!("Order completed: {:?}", order);
    }
}

fn make_order(robot_id: usize, socket: &UdpSocket, server_addr: &str, order: &mut Vec<Item>) -> io::Result<()> {
    let flavors: HashSet<IceCreamFlavor> = order.iter().flat_map(|item| item.flavors.clone()).collect();
    let mut flavors_needed: HashMap<IceCreamFlavor, bool> = flavors.iter().map(|flavor| (flavor.clone(), false)).collect();

    // Se podría cambiar esto a que la orden esté completa
    while flavors_needed.values().any(|&completed| !completed) {
        println!("Flavors needed: {:?}", flavors_needed);
        let mut buf = [0; 1024];
        let flavors_to_request: Vec<IceCreamFlavor> = flavors_needed.iter().filter_map(|(flavor, &completed)| if !completed { Some(flavor.clone()) } else { None }).collect();
        make_request(socket, server_addr, &Request::SolicitarAcceso { robot_id, gustos: flavors_to_request })?;
        match socket.recv_from(&mut buf) {
            Ok((amt, _)) => read_coordinator_answer(robot_id, socket, server_addr, &mut buf, &amt, order, &mut flavors_needed)?,
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => println!("No response from coordinator, retrying..."),
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

fn build() -> usize {
    let args: Vec<String> = std::env::args().collect();
    let robot_id: usize = args[1].parse().expect("Invalid robot ID");
    robot_id
}

fn read_coordinator_answer(robot_id: usize, socket: &UdpSocket, server_addr: &str, buf: &mut [u8; 1024], amt: &usize, order: &mut Vec<Item>, flavors_needed: &mut HashMap<IceCreamFlavor, bool>) -> io::Result<()> {
    let response: Response = serde_json::from_slice(&buf[..*amt]).expect("Failed to parse response");
    match response {
        Response::AccesoConcedido(flavor) => {
            add_flavor_to_ice_cream(&flavor, robot_id);
            send_release_request(&flavor, robot_id, socket, server_addr)?;
            mark_flavor_as_completed(order, &flavor);
            if let Some(completed) = flavors_needed.get_mut(&flavor) {
                *completed = true;
            }
        },
        Response::AccesoDenegado(_) => loop {
            let mut buf = [0; 1024];
            match socket.recv_from(&mut buf) {
                Ok((amt, _)) => read_coordinator_answer(robot_id, socket, server_addr, &mut buf, &amt, order, flavors_needed)?,
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) => {
                    println!("Failed to receive response: {}", e);
                    break;
                }
            }
        },
        _ => {}
    }
    Ok(())
}

fn send_release_request(flavor: &IceCreamFlavor, robot_id: usize, socket: &UdpSocket, server_addr: &str) -> io::Result<()> {
    println!("[Robot {}] libero contenedor de {:?}", robot_id, flavor);
    let release_request = Request::LiberarAcceso { robot_id, gusto: flavor.clone() };
    socket.send_to(&serde_json::to_vec(&release_request)?, server_addr)?;
    let mut buf = [0; 1024];
    while let Ok((len, _)) = socket.recv_from(&mut buf) {
        if let Response::ACK = serde_json::from_slice(&buf[..len])? {
            println!("[Robot {}] ACK recibido", robot_id);
            break;
        }
        println!("[Robot {}] No se recibió ACK", robot_id);
    }
    println!("[Robot {}] Liberación de contenedor de {:?} completada", robot_id, flavor);
    Ok(())
}

fn add_flavor_to_ice_cream(flavor: &IceCreamFlavor, robot_id: usize) {
    println!("[Robot {}] accedo al contenedor de {:?}", robot_id, flavor);
    thread::sleep(Duration::from_secs(5));
}

fn mark_flavor_as_completed(order: &mut Vec<Item>, flavor: &IceCreamFlavor) {
    for item in order.iter_mut() {
        if let Some(completed) = item.flavor_status.get_mut(flavor) {
            *completed = true;
        }
        if item.flavor_status.values().all(|&completed| completed) {
            item.is_completed = true;
            println!("Item completed: {:?}", item);
        }
    }
}

fn make_request(socket: &UdpSocket, server_addr: &str, request: &Request) -> io::Result<()> {
    let request = serde_json::to_vec(request)?;
    socket.send_to(&request, server_addr)?;
    Ok(())
}