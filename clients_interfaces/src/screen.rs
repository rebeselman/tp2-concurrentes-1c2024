//! Represents a screen of a ice cream local

use std::{collections::HashMap, error::Error, fmt::format, fs::File, io::{BufRead, BufReader}, net::UdpSocket, sync::{Arc, Condvar, Mutex}, time::{Duration, Instant}};
use actix::Message;
use serde::de::Expected;

use crate::{order::Order, order_state::OrderState};



const TIMEOUT: Duration = Duration::from_secs(10);

const PAYMENT_GATEWAY_IP: &str = "127.0.0.1:1024";
const ORDER_MANAGEMENT_IP: &str = "127.0.0.1:8080";



#[derive(Clone)]
pub struct Screen {
    id: usize,
    log: HashMap<usize, OrderState>,
    socket: Arc<Mutex<UdpSocket>>,
    responses: Arc<(Mutex<Vec<Option<OrderState>>>, Condvar)>
    
}

// fase 1: screen es como el coordinador, escribe prepare en su log y envia el mensaje
// prepare a gateway de pagos y espera a que este le conteste con ready. Si recibe abort aborta el pedido.
// Si recibe ready envia mensaje prepare a gestion de pedidos y espera a que este le conteste con ready. Si recibe abort aborta el pedido.
// Gateway de pagos y gestions de pedidos que reciben este mensaje, escriben ready en el log y envian ready al coordinador, o abort.
// Fase 2: El coordinador hace los cambios y envia el mensaje de commit al resto de los procesos
// Los procesos que reciben el mensaje, escriben commit en el log y envían finished al coordinador.
// ABORT: en la fase 1, tanto gestion de pedidos como gateway de pagos pueden enviar abort. El coordinador debe broadcasteralo.





fn id_to_addr(id: usize) -> String { "127.0.0.1:1234".to_owned() + &*id.to_string() }

impl Screen {
    fn new(id: usize) -> Result<Screen, Box<dyn Error>>{
        let ret = Screen {
            id,
            log: HashMap::new(),
            socket: Arc::new(Mutex::new(UdpSocket::bind(id_to_addr(id))?)),
            responses: Arc::new((Mutex::new(Vec::new()), Condvar::new()))
            
        };
        let mut clone = ret.clone();
        std::thread::spawn(move || {
            clone.process_orders();
            
        });
        Ok(ret)

    }


    fn protocol(&mut self, order: Order) -> bool {
        if self.prepare_capture(&order) {
            if self.prepare_order(&order) {
                self.commit(&order)
            } else {
                self.abort_capture(&order)
            }
        } else {
            self.abort(&order)
        }
    }

    fn process_orders(&mut self) -> Result<(), Box<dyn Error>>{
        // por cada pedido en archivo:
        let file_path = format!("orders_screen_{}.jsonl", self.id);
        let file = File::open(&file_path)?;
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let order: Order = serde_json::from_str(&line?)?;
            self.protocol(order);
        }
        Ok(())
    }

    fn prepare_capture(&mut self, order: &Order) -> bool {
        self.log.insert(order.id(), OrderState::Wait(Instant::now()));
        println!("[SCREEN] prepare {}", order.id());
        let message = format!("prepare {} {} {}\n", order.id(), order.client_id(), order.credit_card());

        self.broadcast_and_wait(message.as_bytes(), order.id(), OrderState::Commit)
    }

    fn abort_capture(&mut self, order: &Order) -> bool {
        todo!()
    }

    fn prepare_order(&mut self, order: &Order) -> bool {
        todo!()
    }

    fn commit(&mut self, order: &Order) -> bool {
        todo!()
    }

    fn abort(&mut self, order: &Order) -> bool {
        todo!()
    }

    





    
    
    fn broadcast_and_wait(&self, message: &[u8], id: usize, expected: OrderState) -> bool {
        todo!()
    }


}