//! Represents a screen of an ice cream local

use actix::{Actor, Context, Handler};
use orders::order::Order;
use std::sync::MutexGuard;
use std::{
    collections::HashMap,
    env,
    error::Error,
    fs::File,
    io::{BufRead, BufReader},
    net::UdpSocket,
    sync::{Arc, Condvar, Mutex},
    thread,
    time::{Duration, Instant},
};
use std::net::SocketAddr;
//use clients_interfaces::screen_message::ScreenMessage;
use clients_interfaces::{
    order_state::OrderState, screen_message::ScreenMessage, screen_state::ScreenState,
};

const TIMEOUT: Duration = Duration::from_secs(60);
const PAYMENT_GATEWAY_IP: &str = "127.0.0.1:8081";
const ORDER_MANAGEMENT_IP: &str = "127.0.0.1:8090";
const STAKEHOLDERS: usize = 2;
const PAYMENT_GATEWAY: usize = 1;
const ORDER_MANAGEMENT: usize = 0;
const TIMEOUT_PONG: Duration = Duration::from_secs(200);
const PING_INTERVAL: Duration = Duration::from_secs(40);
const SCREENS: usize = 3;

/// A screen is a process that receives orders from clients and processes them.
/// It communicates with the payment gateway and the order management to process the orders.
/// The screen follows a two-phase commit protocol to process the orders.
/// The screen is also an actor that can communicate with other screens to check if they are still alive
/// and to exchange information about the last order processed. This would be used to reassign orders from a screen that has crashed to another screen.
/// Contains the following elements:
///

pub struct Screen {
    id: usize,
    log: HashMap<usize, OrderState>,
    socket: UdpSocket,
    responses: Arc<(Mutex<Vec<Option<OrderState>>>, Condvar)>,
    order_management_ip: SocketAddr,
    screen_in_charge_state: Arc<(Mutex<Option<ScreenState>>, Condvar)>,
    last_order_completed: Option<usize>,
    screen_in_charge: usize,
    ping_screen: usize,
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

// otras cosas a tener en cuenta:
// Resiliencia en las pantallas
// Para verificar el estado de cada pantalla, se envían mensajes tipo ping entre sí cada cierto tiempo para verificar que
// siguen procesando pedidos. En el mensaje ping se envía información del último pedido completado.
// De esta forma, una pantalla puede tomar los pedidos de esa pantalla caída desde esa orden. Se utilizaría el modelo de actores para la comunicación entre las pantallas.
// Cuando se detecta que una pantalla está caída, los pedidos que estaba manejando se reasignan a otra pantalla.
// Ya se tendría establecido qué pantalla se hace cargo de qué pantalla en caso de que se caiga alguna.
// Por ejemplo: tenemos las pantallas 0, 1, 2, 3. Si se cae la 0, se hace cargo la 1, si se cae la 1, se
// hace cargo la 2, si se cae la 3, se hace cargo la 0.

fn id_to_addr(id: usize) -> String {
    "127.0.0.1:1234".to_owned() + &*id.to_string()
}

impl Screen {
    pub fn id(&self) -> usize {
        self.id
    }

    /// Creates a new screen with the given id.
    /// The screen will bind to the address and will spawn a new thread to receive messages from the payment gateway and the order management.
    pub fn new(id: usize) -> Result<Screen, Box<dyn Error>> {
        let screen_charge: usize = if id == SCREENS - 1 { 0 } else { id + 1 };

        let screen_that_pings: usize = if id == 0 { SCREENS - 1 } else { id - 1 };

        let ret = Screen {
            id,
            log: HashMap::new(),
            socket: UdpSocket::bind(id_to_addr(id))?,
            responses: Arc::new((Mutex::new(vec![None; STAKEHOLDERS]), Condvar::new())),
            order_management_ip: ORDER_MANAGEMENT_IP.to_owned().parse().unwrap(),
            screen_in_charge_state: Arc::new((Mutex::new(None), Condvar::new())),
            last_order_completed: None,
            screen_in_charge: screen_charge,
            ping_screen: screen_that_pings,
        };

        let mut clone = ret.clone()?;
        thread::spawn(move || {
            match clone.process_orders() {
                Ok(_) => {}
                Err(e) => println!("[SCREEN {}] Error processing orders: {:?}", id, e),
            }
            match clone.process_orders_from_down_screen() {
                Ok(_) => println!("Screen [{}] finished its work", id),
                Err(e) => println!(
                    "[SCREEN {}] Error processing orders from down screen: {:?}",
                    id, e
                ),
            }
        });

        // thread for pinging assigned screen

        let clone_ping = ret.clone()?;
        thread::spawn(move || {
            loop {
                // break if the screen is finished
                let (lock, _) = &*clone_ping.screen_in_charge_state;
                let responses = lock.lock().map_err(|e| e.to_string()).unwrap();
                if let Some(ScreenState::Finished) = *responses {
                    drop(responses);
                    break;
                }
                drop(responses);

                match clone_ping.broadcast_pings() {
                    Ok(_) => {}
                    Err(e) => println!("[SCREEN {}] Error broadcasting pings: {:?}", id, e),
                }

                thread::sleep(PING_INTERVAL);
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
            order_management_ip: self.order_management_ip.clone(),
            screen_in_charge_state: self.screen_in_charge_state.clone(),
            last_order_completed: self.last_order_completed,
            screen_in_charge: self.screen_in_charge,
            ping_screen: self.ping_screen,
        };
        Ok(ret)
    }

    /// This is the protocol that the screen follows to process an order
    fn protocol(&mut self, order: Order) -> Result<bool, Box<dyn Error>> {
        println!(
            "[SCREEN {}] Processing order in protocol: {:?}",
            self.id,
            order.id()
        );
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
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let order: Order = serde_json::from_str(&line?)?;
            match self.protocol(order) {
                Ok(_) => (),
                Err(e) => println!("[SCREEN {}] Error processing order: {:?}", self.id, e),
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
        self.log
            .insert(order.id(), OrderState::Wait(Instant::now()));
        let order_serialized = serde_json::to_vec(order)?;
        println!("[SCREEN {}]Preparing order: {:?}", self.id, order.id());
        let mut message: Vec<u8> = b"prepare\n".to_vec();
        message.extend_from_slice(&order_serialized);
        self.broadcast_and_wait(&message, OrderState::Ready, order)
    }

    /// This represents the second phase of the two-phase commit protocol. The screen sends a
    /// "commit" message to the payment gateway and the order management and waits for a "finished" message as well.
    /// At this point, they can't abort the order.
    fn commit(&mut self, order: &Order) -> Result<bool, Box<dyn Error>> {
        if let Some(state) = self.log.get(&order.id()) {
            if *state == OrderState::Finished {
                println!("[SCREEN {}] Order {} already committed", self.id, order.id());
                return Ok(true);
            }
        }

        println!("[SCREEN {}] Committing order: {:?}", self.id, order.id());
        self.log.insert(order.id(), OrderState::Finished);

        let order_serialized = serde_json::to_vec(order)?;
        let mut message: Vec<u8> = b"commit\n".to_vec();
        message.extend_from_slice(&order_serialized);
        self.broadcast_and_wait(&message, OrderState::Finished, order)
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
        self.broadcast_and_wait(&message, OrderState::Abort, order)
    }

    /// This method sends a message to another screen.
    fn send_message_to_screen(
        &self,
        screen_id: usize,
        message: ScreenMessage,
    ) -> Result<(), Box<dyn Error>> {
        let addr = id_to_addr(screen_id);
        let message_serialized = serde_json::to_vec(&message)?;
        let mut message: Vec<u8> = b"screen\n".to_vec();
        message.extend_from_slice(&message_serialized);
        self.socket.send_to(&message, addr)?;
        Ok(())
    }

    /// This method sends a message to the payment gateway and the order management and waits for a response.
    /// The screen waits for an expected response from the payment gateway and the order management.
    /// What happens if the screen receives ready from order management (to support the change of coordinator):
    ///        If the screen was expecting ready (it doesn't matter if it is received twice), then everything is fine, it continues waiting for ready from the payment gateway
    ///        If the screen was expecting abort, and it receives ready from order management, it should send abort to order management to clarify that the transaction should not continue as the card failed in this case
    ///        If the screen was expecting finished, and it receives ready from order management, it should send commit to order management to clarify that the transaction should continue since the card was already accepted in this case (when ready was received before)

    fn broadcast_and_wait(
        &mut self,
        message: &[u8],
        expected: OrderState,
        order: &Order,
    ) -> Result<bool, Box<dyn Error>> {
        {
            let mut responses = self.responses.0.lock().map_err(|e| e.to_string())?;
            *responses = vec![None; STAKEHOLDERS];
        }
        println!("[SCREEN {} ]Sending message", self.id);
        self.socket.send_to(message, PAYMENT_GATEWAY_IP)?;
        self.socket.send_to(message, self.order_management_ip)?;
        let (lock, cvar) = &*self.responses;
        let mut responses = lock.lock().map_err(|e| e.to_string())?;
        loop {
            let result: (
                MutexGuard<Vec<Option<OrderState>>>,
                std::sync::WaitTimeoutResult,
            ) = cvar
                .wait_timeout_while(responses, TIMEOUT, |responses| {
                    responses.iter().any(Option::is_none)
                })
                .map_err(|e| e.to_string())?;
            responses = result.0;
            if result.1.timed_out() {
                println!("[SCREEN {}] Timeout waiting for responses", self.id);
                return Ok(false);
            }

            if responses[PAYMENT_GATEWAY] == Some(expected) {
                if responses[ORDER_MANAGEMENT] == Some(expected) {
                    if expected == OrderState::Finished {
                        self.last_order_completed = Some(order.id());
                    }
                    return Ok(true);
                } else if (expected == OrderState::Abort || expected == OrderState::Finished)
                    && responses[ORDER_MANAGEMENT] == Some(OrderState::Ready)
                {
                    self.socket.send_to(message, ORDER_MANAGEMENT_IP)?;
                    continue;
                }
            } else if responses[PAYMENT_GATEWAY] != Some(expected) {
                return Ok(false);
            }
        }
    }

    /// In a different thread should broadcast to all the screens a ping message to check if they are still alive
    /// and to exchange information about the last order processed. This would be used to reassign orders from a screen that has crashed to another screen.
    /// A screen should do this by an interval of time and also wait for the response of the other screens.
    fn broadcast_pings(&self) -> Result<(), Box<dyn Error>> {
        {
            // if all are none, it is because the screens are just starting to operate -> we do nothing
            // if all are active, we change them to down with the last completed request
            // this is because we assume they are down and we check if they are still alive
            // if any does not respond, it remains down and we take action accordingly
            let mut screen_responses = self
                .screen_in_charge_state
                .0
                .lock()
                .map_err(|e| e.to_string())?;
            if let Some(screen_state) = *screen_responses {
                match screen_state {
                    ScreenState::Active(last_order) => {
                        *screen_responses = Some(ScreenState::Down(last_order));
                    }
                    ScreenState::Down(_) => {}
                    ScreenState::Finished => {
                        return Ok(());
                    }
                }
            }
        }
        let message = ScreenMessage::Ping { screen_id: self.id };
        let message_serialized = serde_json::to_vec(&message)?;
        let mut message: Vec<u8> = b"screen\n".to_vec();
        message.extend_from_slice(&message_serialized);
        self.socket
            .send_to(&message, id_to_addr(self.screen_in_charge))?;
        let (lock, cvar) = &*self.screen_in_charge_state;
        let mut responses = lock.lock().map_err(|e| e.to_string())?;
        // wait in the condvar until all the screens are active
        loop {
            let result: (
                MutexGuard<Option<ScreenState>>,
                std::sync::WaitTimeoutResult,
            ) = cvar
                .wait_timeout_while(responses, TIMEOUT_PONG, |responses| match responses {
                    Some(ScreenState::Down(_)) => true,
                    Some(ScreenState::Active(_)) => false,
                    Some(ScreenState::Finished) => false,
                    _ => true,
                })
                .map_err(|e| e.to_string())?;
            responses = result.0;
            if result.1.timed_out() {
                println!("[SCREEN {}] Timeout waiting for PING  responses", self.id);
                return Ok(());
            }
            if let Some(ScreenState::Active(_)) = *responses {
                return Ok(());
            }
            if let Some(ScreenState::Finished) = *responses {
                return Ok(());
            }
        }
    }

    fn process_pong(
        &mut self,
        screen_id: usize,
        last_order: Option<usize>,
    ) -> Result<(), Box<dyn Error>> {
        println!("[SCREEN{}] processing PONG from {}", self.id, screen_id);
        let (lock, cvar) = &*self.screen_in_charge_state;
        let mut responses = lock.lock().map_err(|e| e.to_string())?;
        *responses = Some(ScreenState::Active(last_order));
        cvar.notify_all();
        Ok(())
    }

    fn process_orders_from_down_screen(&mut self) -> Result<(), Box<dyn Error>> {
        let (lock, _) = &*self.screen_in_charge_state;
        let responses = lock.lock().map_err(|e| e.to_string())?;
        // check if my assigned screen is down
        if let Some(ScreenState::Down(last_order)) = *responses {
            println!("Screen {} is down", self.screen_in_charge);
            drop(responses);
            if let Some(order_id) = last_order {
                // I should take the orders that were being processed by that screen
                // and process them
                let file_path = format!("orders_screen_{}.jsonl", self.screen_in_charge);
                let file = File::open(file_path)?;
                let reader = BufReader::new(file);
                for line in reader.lines() {
                    let order: Order = serde_json::from_str(&line?)?;
                    let id_order = order.id();
                    if order.id() > order_id && !self.protocol(order)? {
                        println!("[SCREEN {}] abort", id_order);
                    }
                }
            }
        }
        // send finished message
        self.send_message_to_screen(
            self.ping_screen,
            ScreenMessage::Finished { screen_id: self.id },
        )?;
        Ok(())
    }

    fn process_finished_message(&self) -> Result<(), Box<dyn Error>> {
        let (lock, _) = &*self.screen_in_charge_state;
        let mut responses = lock.lock().map_err(|e| e.to_string())?;
        *responses = Some(ScreenState::Finished);
        Ok(())
    }
}

/// Implement the Actor trait for Screen
impl Actor for Screen {
    type Context = Context<Self>;
}

/// Implement the Handler trait for Screen
impl Handler<ScreenMessage> for Screen {
    type Result = ();

    fn handle(&mut self, msg: ScreenMessage, _ctx: &mut Context<Self>) {
        match msg {
            ScreenMessage::Ping { screen_id } => {
                println!(
                    "[SCREEN{}] received PING MESSAGE FROM {}",
                    self.id, screen_id
                );
                let response = ScreenMessage::Pong {
                    screen_id: self.id,
                    last_order: self.last_order_completed,
                };

                self.send_message_to_screen(screen_id, response)
                    .unwrap_or_else(|e| eprintln!("Error sending pong: {:?}", e));
            }
            ScreenMessage::Pong {
                screen_id,
                last_order,
            } => {
                println!("SCREEN {} received PONG from SCREEN {}", self.id, screen_id);
                self.process_pong(screen_id, last_order)
                    .unwrap_or_else(|e| eprintln!("Error processing pong: {:?}", e));
            }
            ScreenMessage::Finished { screen_id } => {
                println!(
                    "SCREEN {} received FINISHED message from SCREEN {}",
                    self.id, screen_id
                );
                self.process_finished_message().unwrap_or_else(|e| {
                    eprintln!("Error processing finished message from screen: {:?}", e)
                });
            }
        }
    }
}

/// Format of expected messages:
/// {message_type}\n{message}
#[actix_rt::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let id: usize = args[1].parse()?;
    let screen = Screen::new(id)?;
    let screen_cloned = screen.clone()?;
    let screen_actor = screen_cloned.start();
    loop {
        let mut buf = [0; 1024];
        let (size, from) = screen.socket.recv_from(&mut buf)?;
        let message = String::from_utf8_lossy(&buf[..size]);
        let mut parts = message.split('\n');

        let response = parts.next().ok_or("No response")?;
        let mut responses = screen.responses.0.lock().map_err(|e| e.to_string())?;
        match response {
            "ready" => {
                let order_id = parts.next().ok_or("No order id")?.parse::<usize>()?;
                if from.to_string() == PAYMENT_GATEWAY_IP {
                    responses[PAYMENT_GATEWAY] = Some(OrderState::Ready);
                    println!(
                        "[SCREEN {}] received READY from payment gateway for order {}",
                        screen.id, order_id
                    );
                } else {
                    if from != self.order_management_ip {
                        self.order_management_ip = from;
                    }
                    responses[ORDER_MANAGEMENT] = Some(OrderState::Ready);
                    println!(
                        "[SCREEN {}] received READY from order management for order {}",
                        screen.id, order_id
                    );
                    //a double ready from order management means that the coordinator has changed
                }

                screen.responses.1.notify_all();
            }
            "abort" => {
                let order_id = parts.next().ok_or("No order id")?.parse::<usize>()?;
                if from.to_string() == PAYMENT_GATEWAY_IP {
                    responses[PAYMENT_GATEWAY] = Some(OrderState::Abort);
                    println!(
                        "[SCREEN {}] received ABORT from payment gateway for order {}",
                        screen.id, order_id
                    );
                } else {
                    if from != self.order_management_ip {
                        self.order_management_ip = from;
                    }
                    responses[ORDER_MANAGEMENT] = Some(OrderState::Abort);
                    println!(
                        "[SCREEN {}] received ABORT from order management for order {}",
                        screen.id, order_id
                    );
                }
                screen.responses.1.notify_all();
            }
            "finished" => {
                let order_id = parts.next().ok_or("No order id")?.parse::<usize>()?;
                if from.to_string() == PAYMENT_GATEWAY_IP {
                    responses[PAYMENT_GATEWAY] = Some(OrderState::Finished);
                    println!(
                        "[SCREEN {}] received FINISHED from payment gateway for order {}",
                        screen.id, order_id
                    );
                } else {
                    if from != self.order_management_ip {
                        self.order_management_ip = from;
                    }
                    responses[ORDER_MANAGEMENT] = Some(OrderState::Finished);
                    println!(
                        "[SCREEN {}] received FINISHED from order management for order {}",
                        screen.id, order_id
                    );
                }
                screen.responses.1.notify_all();
            }
            "keepalive" => {
                let order_id = parts.next().ok_or("No order id")?.parse::<usize>()?;
                if from.to_string() == PAYMENT_GATEWAY_IP {
                    responses[PAYMENT_GATEWAY] = Some(OrderState::Wait(Instant::now()));
                    println!(
                        "[SCREEN {}] received KEEPALIVE from payment gateway for order {}",
                        screen.id, order_id
                    );
                } else {
                    if from != self.order_management_ip {
                        self.order_management_ip = from;
                    }
                    responses[ORDER_MANAGEMENT] = Some(OrderState::Wait(Instant::now()));
                    println!(
                        "[SCREEN {}] received KEEPALIVE from order management for order {}",
                        screen.id, order_id
                    );
                }
                screen.responses.1.notify_all();
            }
            "screen" => {
                let message: ScreenMessage =
                    serde_json::from_str(parts.next().ok_or("No message")?)?;
                // should convert this to main actix system
                drop(responses);
                screen_actor.send(message).await?;
            }
            _ => {
                println!("[SCREEN {}] ??? received", screen.id());
            }
        }
    }
}
