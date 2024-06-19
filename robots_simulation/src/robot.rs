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
            items.push(Item {
                container,
                units,
                flavors: flavors_item,
            });
        }

        orders.push(items);
    }

    orders
}


fn main() {
    let (sabores_helado, robot_id) = build();

    let addr = format!("0.0.0.0:809{robot_id}");
    let socket = UdpSocket::bind(addr.clone()).expect("Failed to bind socket");
    socket.set_read_timeout(Some(Duration::from_secs(1))).expect("Failed to set read timeout");

    let server_addr = "127.0.0.1:8080";
    // let mut buf = [0; 1024];
    loop {
        // Elegir sabor según número random
        // let (len, addr) = socket.recv_from(&mut buf).unwrap();
        // println!("Received message from {}", addr);
        let mut rng = rand::thread_rng();
        let num_sabor = rng.gen_range(0..sabores_helado.clone().len());
        let sabor:String = sabores_helado[num_sabor].clone();

        let request = Request::SolicitarAcceso {
            robot_id,
            gusto: sabores_helado[num_sabor].to_string(),
            robot_addr: addr.clone().to_string(),
        };

        make_request(&socket, server_addr, &request);

        let mut buf = [0; 1024];
        match socket.recv_from(&mut buf) {
            Ok((amt, _)) => {
                read_coordinator_answer(&sabor, robot_id, addr.clone(), &socket, server_addr, &mut buf, &amt);
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                println!("No response from coordinator, retrying...");
                continue; // Volver a enviar la solicitud
            }
            Err(e) => {
                println!("Failed to receive response: {}", e);
                break;
            }
        }
    }
}

fn build() -> (Vec<String>, usize) {
    let args: Vec<String> = std::env::args().collect();
    let sabores_helado = vec!["chocolate".to_string(), "frutilla".to_string(), "vainilla".to_string(), "menta".to_string(), "limón".to_string()];
    let robot_id: usize = args[1].parse().expect("Invalid robot ID");
    (sabores_helado, robot_id)
}

fn read_coordinator_answer(sabor: &String, robot_id: usize, addr: String, socket: &UdpSocket, server_addr: &str, buf: &mut [u8; 1024], amt: &usize) {
    let response: Response = serde_json::from_slice(&buf[..*amt]).expect("Failed to parse response");
    match response {
        Response::AccesoConcedido => {
            add_flavor_to_ice_cream(sabor, robot_id);
            send_release_request(sabor, robot_id, addr, socket, server_addr);
        },
        Response::AccesoDenegado(err) => println!("[Robot {}] No pudé acceder al contenedor de {}: {}", robot_id, sabor, err),
    }
}

fn send_release_request(sabor: &String, robot_id: usize, addr: String, socket: &UdpSocket, server_addr: &str) {
    let release_request = Request::LiberarAcceso {
        robot_id,
        gusto: sabor.clone(),
        robot_addr: addr,
    };
    let release_request = serde_json::to_vec(&release_request).expect("Failed to serialize release request");
    socket.send_to(&release_request, server_addr).expect("Failed to send release request");
}

fn add_flavor_to_ice_cream(sabor: &String, robot_id: usize) {
    println!("[Robot {}] accedo al contenedor de {}", robot_id, sabor);
    thread::sleep(Duration::from_secs(5)); // Simula tiempo de trabajo
}

fn make_request(socket: &UdpSocket, server_addr: &str, request: &Request) {
    let request = serde_json::to_vec(&request).expect("Failed to serialize request");
    socket.send_to(&request, server_addr).expect("Failed to send request");
}
