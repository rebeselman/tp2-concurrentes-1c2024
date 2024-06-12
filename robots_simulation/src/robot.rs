use serde::{Deserialize, Serialize};
use std::io;
use std::net::UdpSocket;
use std::thread;
use std::thread::sleep;
use std::time::Duration;
use rand::Rng;

#[derive(Serialize, Deserialize, Debug)]
enum Request {
    SolicitarAcceso { robot_id: usize, gusto: String },
    LiberarAcceso { robot_id: usize, gusto: String },
}

#[derive(Serialize, Deserialize, Debug)]
enum Response {
    AccesoConcedido,
    AccesoDenegado(String),
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let sabores_helado = ["chocolate", "frutilla", "vainilla", "menta", "limón"];
    let robot_id: usize = args[1].parse().expect("Invalid robot ID");

    let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind socket");
    socket.set_read_timeout(Some(Duration::from_secs(1))).expect("Failed to set read timeout");

    let server_addr = "127.0.0.1:8080";

    loop {
        // Elegir sabor según número random
        let mut rng = rand::thread_rng();
        let num_sabor = rng.gen_range(0..sabores_helado.len());

        let request = Request::SolicitarAcceso {
            robot_id,
            gusto: sabores_helado[num_sabor].to_string(),
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
                        sleep(Duration::from_secs(5)); // Simula tiempo de trabajo
                        let release_request = Request::LiberarAcceso {
                            robot_id,
                            gusto: sabores_helado[num_sabor].to_string(),
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
