use serde::{Deserialize, Serialize};
use serde_json::to_string_pretty;
use rand::seq::SliceRandom;
use rand::prelude::IndexedRandom;
use rand::Rng;
use std::fs::File;
use std::io::Write;

#[derive(Serialize, Deserialize, Debug)]
struct Item {
    tipo_item: String,
    cantidad: usize,
    sabores: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Pedido {
    pedido_id: usize,
    cliente_id: String,
    items: Vec<Item>,
}

fn main() {
    let sabores = vec![
        "vainilla".to_string(),
        "chocolate".to_string(),
        "frutilla".to_string(),
        "limón".to_string(),
        "menta".to_string(),
        "dulce de leche".to_string(),
        "granizado".to_string(),
        "banana split".to_string(),
        "tramontana".to_string(),
        "chocolate amargo".to_string(),
        "menta granizada".to_string(),
        "americana".to_string(),
    ];

    let tipo_items = vec![
        "cucurucho".to_string(),
        "vasito".to_string(),
        "1/4 kilo".to_string(),
        "1/2 kilo".to_string(),
        "1 kilo".to_string(),
    ];

    let mut rng = rand::thread_rng();
    let num_pedidos = 10;
    let mut pedidos = Vec::new();

    for pedido_id in 1..=num_pedidos {
        let cliente_id = format!("{}", rng.gen_range(1000..9999));
        let num_items = rng.gen_range(1..=3);
        let mut items = Vec::new();

        for _ in 0..num_items {
            let tipo_item = tipo_items.choose(&mut rng).unwrap().clone();
            let cantidad = rng.gen_range(1..=3);
            let num_sabores = match tipo_item.as_str() {
                "cucurucho" | "vasito" => 2,
                "1/4 kilo" => 2,
                "1/2 kilo" => 2,
                "1 kilo" => 3,
                _ => 1,
            };
            let sabores_item: Vec<String> = sabores.as_slice().choose_multiple(&mut rng, num_sabores).cloned().collect();
            items.push(Item { tipo_item, cantidad, sabores: sabores_item });
        }

        pedidos.push(Pedido { pedido_id, cliente_id, items });
    }

    let json_output = to_string_pretty(&pedidos).unwrap();
    println!("{}", json_output);

    let mut file = File::create("orders_screen_0.jsonl").unwrap();
    file.write_all(json_output.as_bytes()).unwrap();
}