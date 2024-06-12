use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::UdpSocket;
use std::sync::Mutex;
use std::sync::Arc;
use tokio::sync::Semaphore;
use std::time::Duration;

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

struct Coordinator {
    contenedores: HashMap<String, (bool, Arc<Semaphore>)>,
}

impl Coordinator {
    fn new(gustos: Vec<String>) -> Self {
        let contenedores = gustos.into_iter().map(|gusto| (gusto, (true, Arc::new(Semaphore::new(1))))).collect();
        Coordinator { contenedores }
    }

    fn handle_request(&mut self, request: Request) -> Response {
        match request {
            Request::SolicitarAcceso { robot_id, gusto } => {
                if let Some((available, semaphore)) = self.contenedores.get_mut(&gusto) {
                    if *available {
                        let _permit = semaphore.acquire();
                        *available = false;
                        println!("[Coordinator] Robot {} accedió al contenedor de {}", robot_id, gusto);
                        Response::AccesoConcedido
                    } else {
                        println!("[Coordinator] Robot {} no pudo acceder al contenedor de {} (ocupado)", robot_id, gusto);
                        Response::AccesoDenegado("Contenedor ocupado".into())
                    }
                } else {
                    println!("El contenedor de {} no existe", gusto);
                    Response::AccesoDenegado("Contenedor no existe".into())
                }
            }
            Request::LiberarAcceso { robot_id, gusto } => {
                if let Some((available, _semaphore)) = self.contenedores.get_mut(&gusto) {
                    if *available == false {
                        *available = true;
                        println!("[Coordinator] Robot {} liberó contenedor de {}", robot_id, gusto);
                        Response::AccesoConcedido
                    } else {
                        println!("El contenedor de {} no está ocupado", gusto);
                        Response::AccesoDenegado("Contenedor no existe".into())
                    }
                } else {
                    println!("El contenedor de {} no existe", gusto);
                    Response::AccesoDenegado("Contenedor no existe".into())
                }
            }
        }
    }

    fn start(&mut self, addr: &str) {
        let socket = UdpSocket::bind(addr).expect("Failed to bind address");
        println!("Coordinator escuchando en {}", addr);

        let mut buf = [0; 1024];

        loop {
            let (amt, src) = socket.recv_from(&mut buf).expect("Failed to receive data");

            let request: Request = serde_json::from_slice(&buf[..amt]).expect("Failed to parse request");
            let response = self.handle_request(request);

            let response = serde_json::to_vec(&response).expect("Failed to serialize response");
            socket.send_to(&response, src).expect("Failed to send response");
        }
    }
}

fn main() {
    let gustos = vec!["vainilla".to_string(), "chocolate".to_string(), "frutilla".to_string(), "limón".to_string(), "menta".to_string()];
    let mut coordinador = Coordinator::new(gustos);
    coordinador.start("127.0.0.1:8080");
}
