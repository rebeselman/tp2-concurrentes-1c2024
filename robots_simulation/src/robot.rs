use std::io;
use std::net::UdpSocket;
use std::thread;
use std::time::Duration;
use rand::prelude::IndexedRandom;
use rand::Rng;
use robots_simulation::{ContainerType, IceCreamFlavor, Item, Request, Response};

/*pub fn generate_ice_cream_orders(num_orders: usize) -> Vec<Vec<Item>> {
    let mut rng = rand::thread_rng();

    let flavors = vec![
        IceCreamFlavor::Chocolate,
        IceCreamFlavor::Strawberry,
        IceCreamFlavor::Vanella,
        IceCreamFlavor::Mint,
        IceCreamFlavor::Lemon,
    ];

    let container_types:Vec<ContainerType> = vec![
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
            let container = container_types.choose(&mut rng).unwrap().clone();
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
*/
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let sabores_helado = ["chocolate", "frutilla", "vainilla", "menta", "limón"];
    let robot_id: usize = args[1].parse().expect("Invalid robot ID");


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
        let num_sabor = rng.gen_range(0..sabores_helado.len());

        let request = Request::SolicitarAcceso {
            robot_id,
            gusto: sabores_helado[num_sabor].to_string(),
            robot_addr: addr.clone().to_string(),
        };

        let request = serde_json::to_vec(&request).expect("Failed to serialize request");
        socket.send_to(&request, server_addr).expect("Failed to send request");

        let mut buf = [0; 1024];
        match socket.recv_from(&mut buf) {
            Ok((amt, _)) => {
                let response: Response = serde_json::from_slice(&buf[..amt]).expect("Failed to parse response");
                match response {
                    Response::AccesoConcedido => {
                        println!("[Robot {}] accedo al contenedor de {}", robot_id, sabores_helado[num_sabor]);
                        thread::sleep(Duration::from_secs(5)); // Simula tiempo de trabajo
                        let release_request = Request::LiberarAcceso {
                            robot_id,
                            gusto: sabores_helado[num_sabor].to_string(),
                            robot_addr: addr.clone().to_string(),
                        };
                        let release_request = serde_json::to_vec(&release_request).expect("Failed to serialize release request");
                        socket.send_to(&release_request, server_addr).expect("Failed to send release request");
                    },
                    Response::AccesoDenegado(err) => println!("[Robot {}] No pudé acceder al contenedor de {}: {}", robot_id, sabores_helado[num_sabor], err),
                }
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

        thread::sleep(Duration::from_secs(5)); // Simula tiempo de trabajo
    }
}
