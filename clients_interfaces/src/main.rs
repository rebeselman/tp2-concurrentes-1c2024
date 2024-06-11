
const _NUMBER_SCREENS: u8 = 3;
const _PAYMENT_GATEWAY_IP: &str = "127.0.0.1:8080";
const _ORDER_MANAGEMENT_IP: &str = "127.0.0.1:8081";
const _SCREEN_IP: &str = "127.0.0.1:8080";












fn main() {
// fn main
// para cada pantalla:
//     procesar pedidos()
//
//
  
   
   
}


// Simulates the process of orders of a screen
// fn process_orders(screen_id: u32) -> Result<(), Box<dyn Error>>{
//     let file_path = format!("orders_screen_{}.jsonl",screen_id);
//     let file = File::open(&file_path)?;
//     let reader = BufReader::new(file);
//     // bind to my address
//     let socket = UdpSocket::bind(String::from(SCREEN_IP) + &screen_id.to_string())?;
    
//     for line in reader.lines(){
//         let order_line = line?;
//         let order: Order = serde_json::from_str(&order_line)?;
//         println!("Screen {}: Prepare order {}", screen_id, order.id());

//         let _ = socket.send_to("prepare\n".as_bytes(), PAYMENT_GATEWAY_IP)?;
//         // should wait until i get a respond
    

//     }

//     Ok(())
// }





// fn procesar pedidos()
//      por cada pedido en archivo:
//          orden = deserializar(pedido)
//          escribe en su log prepare indicando comienzo de transaccion
//          envía mensaje de prepare a gatway de pagos
//          si gatway de pagos responde abort:
//                abortar pedido
//          envía mensaje prepare a gestion de pedidos
//          si gestion de pedidos responde abort:
//                enviar abortar a gateway de pagos
//                abortar
//          enviar mensaje de confirmar a gateway de pagos