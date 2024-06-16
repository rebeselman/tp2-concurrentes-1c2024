//! Represents a screen of an ice cream local

use std::{collections::HashMap, error::Error, fs::File, io::{BufRead, BufReader}, mem::size_of, net::UdpSocket, sync::{Arc, Condvar, Mutex}, time::{Duration, Instant}};
use crate::{order::Order, order_state::OrderState};
const _TIMEOUT: Duration = Duration::from_secs(10);
const PAYMENT_GATEWAY_IP: &str = "127.0.0.1:1024";
const ORDER_MANAGEMENT_IP: &str = "127.0.0.1:8080";
const STAKEHOLDERS: usize = 2;
const PAYMENT_GATEWAY : usize = 0;
const ORDER_MANAGEMENT : usize = 1;

pub struct Screen {
    id: usize,
    log: HashMap<usize, OrderState>,
    socket: UdpSocket,
    responses: Arc<(Mutex<Vec<OrderState>>, Condvar)>
    
}
// cosas a tener en cuenta:
// fase 1: screen es como el coordinador, escribe prepare en su log y envia el mensaje
// prepare a gateway de pagos y espera a que este le conteste con ready. Si recibe abort aborta el pedido.
// Si recibe ready envia mensaje prepare a gestion de pedidos y espera a que este le conteste con ready. Si recibe abort aborta el pedido.
// Gateway de pagos y gestions de pedidos que reciben este mensaje, escriben ready en el log y envian ready al coordinador, o abort.
// Fase 2: El coordinador hace los cambios y envia el mensaje de commit al resto de los procesos
// Los procesos que reciben el mensaje, escriben commit en el log y envían finished al coordinador.
// ABORT: en la fase 1, tanto gestion de pedidos como gateway de pagos pueden enviar abort. El coordinador debe broadcasteralo.
// Se asume que la pantalla atiende un pedido a la vez. 

fn id_to_addr(id: usize) -> String { "127.0.0.1:1234".to_owned() + &*id.to_string() }

impl Screen {
    /// Creates a new screen with the given id. 
    /// The screen will bind to the address and will spawn a new thread to receive messages from the payment gateway and the order management.
    pub fn new(id: usize) -> Result<Screen, Box<dyn Error>>{
        let ret = Screen {
            id,
            log: HashMap::new(),
            socket: UdpSocket::bind(id_to_addr(id))?,
            responses: Arc::new((Mutex::new(vec![OrderState::Wait(Instant::now()); STAKEHOLDERS]), Condvar::new()))
            
        };
        let clone = ret.clone()?;
        std::thread::spawn(move || {
            match clone.receiver(){
                Ok(_) => {}
                Err(e) => println!("[SCREEN] Error al recibir mensajes: {:?}", e)
            }
            
        });
        Ok(ret)

    }

    fn clone(&self) -> Result<Screen, Box<dyn Error>> {
        let ret = Screen {
            id: self.id,
            log: HashMap::new(),
            socket: self.socket.try_clone()?,
            responses: self.responses.clone(),
        };
        Ok(ret)
    }

    /// This is the protocol that the screen follows to process an order
    fn protocol(&mut self, order: Order) -> Result<bool, Box<dyn Error>> {
        if self.prepare(&order)? {
            self.commit(&order)
        } else {
            self.abort(&order)
        }
    }

    /// The screen processes the orders in the file using the protocol (two-phase commit)
    pub fn process_orders(&mut self) -> Result<(), Box<dyn Error>>{
        // por cada pedido en archivo:
        let file_path = format!("orders_screen_{}.jsonl", self.id);
        let file = File::open(&file_path)?;
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let order: Order = serde_json::from_str(&line?)?;
            if !self.protocol(order)? {
                println!("[SCREEN] abort");
            }
        }
        Ok(())
    }

    /// The screen sends a "prepare" message to the payment gateway and the order management
    /// and waits for a "ready" message from both. If it receives an "abort" message from any
    /// of them, it aborts the order completely meaning that it will return false. 
    /// A "prepare" message has different meanings depending on the recipient:
    /// - For the payment gateway, it means that the transaction of the client has been captured succesfully.
    /// - For the order management, it means that the order is able to be prepared and is ready to be served to the client.
    fn prepare(&mut self, order: &Order) -> Result<bool, Box<dyn Error> > {
        self.log.insert(order.id(), OrderState::Wait(Instant::now()));
        let order_serialized = serde_json::to_vec(order)?;
        let mut message:Vec<u8> = b"prepare\n".to_vec();
        message.extend_from_slice(&order_serialized); 
        self.broadcast_and_wait(&message, order.id(), OrderState::Ready)
    }

    /// This represents the second phase of the two-phase commit protocol. The screen sends a
    /// "commit" message to the payment gateway and the order management and waits for a "finished" message as well.
    /// At this point, they can't abort the order.
    fn commit(&mut self, order: &Order) -> Result<bool, Box<dyn Error>> {
        self.log.insert(order.id(), OrderState::Commit); 
        let order_serialized = serde_json::to_vec(order)?;
        let mut message:Vec<u8> = b"commit\n".to_vec();
        message.extend_from_slice(&order_serialized);
        self.broadcast_and_wait(&message, order.id(), OrderState::Commit)
    }

    /// This method is called when the screen receives an "abort" message from the payment gateway or the order management in
    /// the first phase of the two-phase commit protocol. It sends an "abort" message to the other party and returns false.
    /// The cases in which the screen sends an "abort" message are:
    /// - The payment gateway sends an "abort" message to the screen because the credit card of the client was declined.
    /// - The order management sends an "abort" message to the screen because the order can't be prepared for some reason.
    fn abort(&mut self, order: &Order) -> Result<bool, Box<dyn Error>> {
        self.log.insert(order.id(), OrderState::Abort);
        let order_serialized = serde_json::to_vec(order).unwrap();
        let mut message:Vec<u8> = b"abort\n".to_vec();
        message.extend_from_slice(&order_serialized);
        self.broadcast_and_wait(&message, order.id(), OrderState::Abort)
    }

    /// This method sends a message to the payment gateway and the order management and waits for a response.
    /// The screen waits for a expected response from the payment gateway and the order management.
    fn broadcast_and_wait(&self, message: &[u8], id: usize, expected: OrderState) -> Result<bool, Box<dyn Error>> {
        let mut responses = self.responses.0.lock().map_err(|e|e.to_string())?;
        *responses = vec![OrderState::Wait(Instant::now()); STAKEHOLDERS];
        
        self.socket.send_to(message, PAYMENT_GATEWAY_IP)?;
        // self.socket.send_to(message, ORDER_MANAGEMENT_IP)?;
        //     let mut responses = self.responses.1.wait_timeout_while(responses, _TIMEOUT, |re| )
        //     Ok(responses.iter().all(|r| r == &expected))

        return Ok(true);
    }

    /// This method receives messages that could be from either the payment gateway or the order management.
    /// And modifies the order state in the log accordingly.
    /// Should receive a message in this format
    /// <order_id>\n<response>
    fn receiver(&self) -> Result<(), Box<dyn Error>> {
        loop {
            let mut buf = [0; size_of::<usize>() + 1];
            let (size, from) = self.socket.recv_from(&mut buf)?;
            let message = String::from_utf8_lossy(&buf[..size]);
            let mut parts = message.split('\n');
            let order_id = parts.next().ok_or("No order id")?.parse::<usize>()?;
            let response = parts.next().ok_or("No response")?;
            match response {
                "ready" => {
                    let mut responses = self.responses.0.lock().map_err(|e|e.to_string())?;
                    if from.to_string() == PAYMENT_GATEWAY_IP {
                        responses[PAYMENT_GATEWAY] = OrderState::Ready;
                        println!("[SCREEN {}]  recibí READY de gateway de pagos de la orden n° {}", self.id, order_id);
                    } else {
                        responses[ORDER_MANAGEMENT] = OrderState::Ready;
                        println!("[SCREEN {}]  recibí READY de gestion de pedidos de la orden  n° {}", self.id, order_id);
                        
                    }
                    
                    self.responses.1.notify_all();
                    
                }
                "abort" => {
                    let mut responses = self.responses.0.lock().map_err(|e|e.to_string())?;
                    if from.to_string() == PAYMENT_GATEWAY_IP {
                        responses[PAYMENT_GATEWAY] = OrderState::Abort;
                        println!("[SCREEN {}]  recibí ABORT de gateway de pagos de la orden n° {}", self.id, order_id);
                    } else {
                        responses[ORDER_MANAGEMENT] = OrderState::Abort;
                        println!("[SCREEN {}]  recibí ABORT de gestion de pedidos de la orden  n° {}", self.id, order_id);
                        
                    }
                    self.responses.1.notify_all();
                }
                "finished" => {

                    let mut responses = self.responses.0.lock().map_err(|e|e.to_string())?;
                    if from.to_string() == PAYMENT_GATEWAY_IP {
                        responses[PAYMENT_GATEWAY] = OrderState::Commit;
                        println!("[SCREEN {}]  recibí FINISHED de gateway de pagos de la orden n° {}", self.id, order_id);
                    } else {
                        responses[ORDER_MANAGEMENT] = OrderState::Commit;
                        println!("[SCREEN {}]  recibí FINISHED de gestion de pedidos de la orden  n° {}", self.id, order_id);
                        
                    }
                    self.responses.1.notify_all();
                }
                "keepalive" => {
                   
                    let mut responses = self.responses.0.lock().map_err(|e|e.to_string())?;
                    if from.to_string() == PAYMENT_GATEWAY_IP {
                        responses[PAYMENT_GATEWAY] = OrderState::Wait(Instant::now());
                        println!("[SCREEN {}]  recibí KEEPALIVE de gateway de pagos de la orden n° {}", self.id, order_id);
                    } else {
                        responses[ORDER_MANAGEMENT] = OrderState::Wait(Instant::now());
                        println!("[SCREEN {}]  recibí KEEPALIVE de gestion de pedidos de la orden  n° {}", self.id, order_id);
                        
                    }
                    self.responses.1.notify_all();

                }
                _ => {
                    println!("[SCREEN] ??? recibe de orden n° {}", order_id);
                }
            }
            

        }
    }


}