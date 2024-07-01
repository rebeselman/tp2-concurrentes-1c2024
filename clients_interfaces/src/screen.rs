//! Represents a screen of an ice cream local

use std::{collections::HashMap, error::Error, fs::File, io::{BufRead, BufReader}, net::UdpSocket, sync::{Arc, Condvar, Mutex}, thread, time::{Duration, Instant}};
use std::sync::MutexGuard;
use orders::order::Order;
use crate::order_state::OrderState;
const TIMEOUT: Duration = Duration::from_secs(60);
const PAYMENT_GATEWAY_IP: &str = "127.0.0.1:8081";
const ORDER_MANAGEMENT_IP: &str = "127.0.0.1:8090";
const STAKEHOLDERS: usize = 2;
const PAYMENT_GATEWAY: usize = 1;
const ORDER_MANAGEMENT: usize = 0;

pub struct Screen {
    id: usize,
    log: HashMap<usize, OrderState>,
    socket: UdpSocket,
    responses: Arc<(Mutex<Vec<Option<OrderState>>>, Condvar)>
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
    pub fn new(id: usize) -> Result<Screen, Box<dyn Error>> {
        let ret = Screen {
            id,
            log: HashMap::new(),
            socket: UdpSocket::bind(id_to_addr(id))?,
            responses: Arc::new((Mutex::new(vec![None; STAKEHOLDERS]), Condvar::new()))
        };
        let clone = ret.clone()?;
        thread::spawn(move || {
            if let Err(e) = clone.receiver() {
                println!("[SCREEN {}] Error receiving messages: {:?}", id, e);
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
        println!("[SCREEN {}] Processing order in protocol: {:?}", self.id, order.id());
        if self.prepare(&order)? {
            self.commit(&order)
        } else {
            self.abort(&order)
        }
    }

    /// The screen processes the orders in the file using the protocol (two-phase commit)
    pub fn process_orders(&mut self) -> Result<(), Box<dyn Error>> {
        println!("[SCREEN {}] Processing orders", self.id);
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
    fn prepare(&mut self, order: &Order) -> Result<bool, Box<dyn Error>> {
        self.log.insert(order.id(), OrderState::Wait(Instant::now()));
        let order_serialized = serde_json::to_vec(order)?;
        println!("[SCREEN {}]Preparing order: {:?}", self.id, order.id());
        let mut message: Vec<u8> = b"prepare\n".to_vec();
        message.extend_from_slice(&order_serialized);
        self.broadcast_and_wait(&message, OrderState::Ready)
    }

    /// This represents the second phase of the two-phase commit protocol. The screen sends a
    /// "commit" message to the payment gateway and the order management and waits for a "finished" message as well.
    /// At this point, they can't abort the order.
    fn commit(&mut self, order: &Order) -> Result<bool, Box<dyn Error>> {
        println!("[SCREEN {}] commiting order: {:?}", self.id, order.id());
        self.log.insert(order.id(), OrderState::Finished);
        let order_serialized = serde_json::to_vec(order)?;
        let mut message: Vec<u8> = b"commit\n".to_vec();
        message.extend_from_slice(&order_serialized);
        self.broadcast_and_wait(&message, OrderState::Finished)
    }

    /// This method is called when the screen receives an "abort" message from the payment gateway or the order management in
    /// the first phase of the two-phase commit protocol. It sends an "abort" message to the other party and returns false.
    /// The cases in which the screen sends an "abort" message are:
    /// - The payment gateway sends an "abort" message to the screen because the credit card of the client was declined.
    /// - The order management sends an "abort" message to the screen because the order can't be prepared for some reason.
    fn abort(&mut self, order: &Order) -> Result<bool, Box<dyn Error>> {
        println!("[SCREEN {}] Aborting order: {:?}", self.id, order.id());
        self.log.insert(order.id(), OrderState::Abort);
        let order_serialized = serde_json::to_vec(order)?;
        let mut message: Vec<u8> = b"abort\n".to_vec();
        message.extend_from_slice(&order_serialized);
        self.broadcast_and_wait(&message, OrderState::Abort)
    }




    /// This method sends a message to the payment gateway and the order management and waits for a response.
    /// The screen waits for an expected response from the payment gateway and the order management.
    /// What happens if the screen receives ready from order management (to support the change of coordinator):
    ///        If the screen was expecting ready (it doesn't matter if it is received twice), then everything is fine, it continues waiting for ready from the payment gateway
    ///        If the screen was expecting abort, and it receives ready from order management, it should send abort to order management to clarify that the transaction should not continue as the card failed in this case
    ///        If the screen was expecting finished, and it receives ready from order management, it should send commit to order management to clarify that the transaction should continue since the card was already accepted in this case (when ready was received before)

    fn broadcast_and_wait(&mut self, message: &[u8], expected: OrderState) -> Result<bool, Box<dyn Error>> {
        {
            let mut responses = self.responses.0.lock().map_err(|e| e.to_string())?;
            *responses = vec![None; STAKEHOLDERS];
        }
        println!("[SCREEN {} ]Sending message", self.id);
        self.socket.send_to(message, PAYMENT_GATEWAY_IP)?;
        self.socket.send_to(message, ORDER_MANAGEMENT_IP)?;
        let (lock, cvar) = &*self.responses;
        let mut responses = lock.lock().map_err(|e| e.to_string())?;
        loop {
            let result: (MutexGuard<Vec<Option<OrderState>>>, std::sync::WaitTimeoutResult) = cvar.wait_timeout_while(responses, TIMEOUT, |responses| {
                responses.iter().any(Option::is_none)
            }).map_err(|e| e.to_string())?;
            responses = result.0;
            if result.1.timed_out() {
                println!("[SCREEN {}] Timeout waiting for responses", self.id);
                return Ok(false);
            }

            if responses[PAYMENT_GATEWAY] ==  Some(expected){
                if responses[ORDER_MANAGEMENT] == Some(expected) {
                    return Ok(true);

                }
                else if (expected == OrderState::Abort || expected == OrderState::Finished ) && responses[ORDER_MANAGEMENT] == Some(OrderState::Ready){
                    self.socket.send_to(message, ORDER_MANAGEMENT_IP)?;
                    continue;
                }

            }
            else if responses[PAYMENT_GATEWAY] != Some(expected) {
                return Ok(false);
            }





        }
    }

    /// This method receives messages that could be from either the payment gateway or the order management.
    /// And modifies the order state in the log accordingly.
    /// Should receive a message in this format
    /// <order_id>\n<response>
    fn receiver(&self) -> Result<(), Box<dyn Error>> {
        loop {
            let mut buf = [0; 1024];
            let (size, from) = self.socket.recv_from(&mut buf)?;
            let message = String::from_utf8_lossy(&buf[..size]);
            //println!("Received message: {}", message);
            let mut parts = message.split('\n');
            let order_id = parts.next().ok_or("No order id")?.parse::<usize>()?;
            let response = parts.next().ok_or("No response")?;
            //println!("Received message for order {} with response {}", order_id, response);
            let mut responses = self.responses.0.lock().map_err(|e| e.to_string())?;
            match response {
                "ready" => {
                    if from.to_string() == PAYMENT_GATEWAY_IP {
                        responses[PAYMENT_GATEWAY] = Some(OrderState::Ready);
                        println!("[SCREEN {}] received READY from payment gateway for order {}", self.id, order_id);
                    } else {
                        responses[ORDER_MANAGEMENT] = Some(OrderState::Ready);
                        println!("[SCREEN {}] received READY from order management for order {}", self.id, order_id);
                        //a double ready from order management means that the coordinator has changed
                    }

                    self.responses.1.notify_all();

                }
                "abort" => {
                    if from.to_string() == PAYMENT_GATEWAY_IP {
                        responses[PAYMENT_GATEWAY] = Some(OrderState::Abort);
                        println!("[SCREEN {}] received ABORT from payment gateway for order {}", self.id, order_id);
                    } else {
                        responses[ORDER_MANAGEMENT] = Some(OrderState::Abort);
                        println!("[SCREEN {}] received ABORT from order management for order {}", self.id, order_id);
                    }
                    self.responses.1.notify_all();
                }
                "finished" => {
                    if from.to_string() == PAYMENT_GATEWAY_IP {
                        responses[PAYMENT_GATEWAY] = Some(OrderState::Finished);
                        println!("[SCREEN {}] received FINISHED from payment gateway for order {}", self.id, order_id);
                    } else {
                        responses[ORDER_MANAGEMENT] = Some(OrderState::Finished);
                        println!("[SCREEN {}] received FINISHED from order management for order {}", self.id, order_id);
                    }
                    self.responses.1.notify_all();
                }
                "keepalive" => {
                    if from.to_string() == PAYMENT_GATEWAY_IP {
                        responses[PAYMENT_GATEWAY] = Some(OrderState::Wait(Instant::now()));
                        println!("[SCREEN {}] received KEEPALIVE from payment gateway for order {}", self.id, order_id);
                    } else {
                        responses[ORDER_MANAGEMENT] = Some(OrderState::Wait(Instant::now()));
                        println!("[SCREEN {}] received KEEPALIVE from order management for order {}", self.id, order_id);
                    }
                    self.responses.1.notify_all();

                }
                _ => {
                    println!("[SCREEN {}] ??? received from order {}", self.id, order_id);
                }
            }


        }
    }


}
