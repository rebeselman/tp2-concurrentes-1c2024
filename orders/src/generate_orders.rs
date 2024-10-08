use super::container_type::ContainerType;
use super::ice_cream_flavor::IceCreamFlavor;
use super::item::Item;
use super::order::Order;
use rand::distributions::Alphanumeric;
use rand::prelude::{SliceRandom, ThreadRng};
use rand::Rng;
use std::fs::File;
use std::io::Write;

pub fn generate_orders(screen_number: u32) -> Result<(), Box<dyn std::error::Error>> {
    let mut rng = rand::thread_rng();

    for i in 0..screen_number {
        let mut file = File::create(format!("orders_screen_{}.jsonl", i))?;
        let mut order_id = rng.gen_range(0..1000);
        for _ in 0..rng.gen_range(1..10) {
            let order = create_order_with_id(&mut rng, order_id)?;
            order_id += 1;

            file.write_all(serde_json::to_string(&order)?.as_bytes())?;
            file.write_all(b"\n")?;
        }
    }
    Ok(())
}

pub fn create_order_with_id(mut rng: &mut ThreadRng, order_id: usize) -> Result<Order, Box<dyn std::error::Error>> {
    let client_id = rng.gen_range(0..1000);
    let credit_card: String = (0..16).map(|_| rng.sample(Alphanumeric) as char).collect();
    let mut items = Vec::new();
    for _ in 0..rng.gen_range(1..10) {
        // choose a random container

        let container: ContainerType = *ContainerType::values()
            .choose(&mut rng)
            .ok_or_else(|| String::from("Error choosing random container"))?;
        let units = rng.gen_range(1..5);
        let number_of_flavors = rng.gen_range(1..3);
        // vector of ice cream flavors
        let flavors: Vec<IceCreamFlavor> = (0..number_of_flavors)
            .map(|_| {
                IceCreamFlavor::values()
                    .choose(&mut rng)
                    .ok_or_else(|| String::from("Error choosing flavors"))
                    .copied()
            })
            .collect::<Result<Vec<IceCreamFlavor>, String>>()?;

        items.push(Item::new(container, units, flavors));
    }
    let order = Order::new(order_id, client_id, credit_card, items);
    Ok(order)
}
