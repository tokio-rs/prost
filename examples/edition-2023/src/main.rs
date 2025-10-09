use prost::Message;
use prost_types::Timestamp;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

// Include the generated protobuf code
mod proto {
    include!(concat!(env!("OUT_DIR"), "/bar.rs"));
}

use proto::{Beverage, Order};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse tip from command line argument --tip <amount>
    let args: Vec<String> = env::args().collect();
    let tip = parse_tip_arg(&args)?;

    if let Some(amount) = tip {
        println!("💰 Tip provided: {} gold doubloons", amount);
    } else {
        println!("💰 No tip provided (use: cargo run -p edition-2023-example -- --tip <amount>)");
    }
    println!();

    // Get the current timestamp
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?;
    let timestamp = Timestamp {
        seconds: now.as_secs() as i64,
        nanos: now.subsec_nanos() as i32,
    };

    // Create a humorous order message with the provided tip
    let order = Order {
        timestamp: Some(timestamp),
        beverage: Some(Beverage::Rum as i32),
        message: Some("Arrr! Why did the pirate choose RUM? Because it's the only drink that makes his treasure map look straighter! 🗺️".to_string()),
        tip,
    };

    println!("📝 Original Order:");
    print_order(&order);

    // Serialize the message to protobuf bytes
    let mut buf = Vec::new();
    order.encode(&mut buf)?;
    println!("\n📦 Serialized to {} bytes", buf.len());

    // Deserialize the message back from bytes
    let deserialized_order = Order::decode(&buf[..])?;
    println!("\n🔓 Deserialized Order:");
    print_order(&deserialized_order);

    // Check if the tip field was set after deserialization
    println!("\n💰 Tip Check:");
    if let Some(tip_amount) = deserialized_order.tip.filter(|&tip| tip > 0) {
        println!("  ✅ Tip received: {} gold doubloons!", tip_amount);
        println!("  🍻 \"Fair winds and following seas, generous matey!\"");
    } else {
        println!("  ❌ No tip detected!");
        println!("\n  ⚓ ANGRY PIRATE SAYS:");
        println!("  SHIVER ME TIMBERS! No tip?! Ye scurvy dog!");
        println!("  I hope ye get marooned on a desert island with");
        println!("  nothin' but warm grog and soggy hardtack!");
        println!("  May yer treasure map lead ye in circles,");
        println!("  and may ye never find the buried booty!");
        println!("  ARRRRR! 🦜");
    }

    Ok(())
}

fn print_order(order: &Order) {
    let json = serde_json::json!({
        "timestamp": order.timestamp.as_ref().map(|t| t.to_string()),
        "beverage": order.beverage.and_then(|b| Beverage::try_from(b).ok().map(|bev| format!("{:?}", bev))),
        "message": order.message,
        "tip": order.tip
    });
    println!("{}", serde_json::to_string_pretty(&json).unwrap());
}

fn parse_tip_arg(args: &[String]) -> Result<Option<u32>, String> {
    // Look for --tip argument
    if let Some(pos) = args.iter().position(|arg| arg == "--tip") {
        // Get the value after --tip
        let value = args
            .get(pos + 1)
            .ok_or_else(|| "Missing value after --tip. Usage: --tip <amount>".to_string())?;

        // Parse the value as u32
        let amount = value.parse::<u32>().map_err(|e| {
            format!(
                "Invalid tip amount '{}': {}. Please provide a valid number.",
                value, e
            )
        })?;

        Ok(Some(amount))
    } else {
        Ok(None)
    }
}
