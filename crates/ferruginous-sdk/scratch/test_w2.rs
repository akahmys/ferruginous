use std::collections::BTreeMap;
use ferruginous_sdk::core::Object;

fn main() {
    let mut cid_widths2 = BTreeMap::new();
    let start = 10;
    // Trio variant: [start [w1y vx vy]]
    let metrics = vec![
        Object::Integer(900), // w1y
        Object::Integer(500), // vx
        Object::Integer(880), // vy
    ];
    
    let mut j = 0;
    let mut idx = 0;
    while j + 2 < metrics.len() {
        let w1y = match &metrics[j] { Object::Integer(n) => *n as f64, _ => 0.0 };
        let vx = match &metrics[j+1] { Object::Integer(n) => *n as f64, _ => 0.0 };
        let vy = match &metrics[j+2] { Object::Integer(n) => *n as f64, _ => 0.0 };
        cid_widths2.insert(start + idx, (w1y, vx, vy));
        j += 3;
        idx += 1;
    }
    
    println!("Cid 10: {:?}", cid_widths2.get(&10));
}
