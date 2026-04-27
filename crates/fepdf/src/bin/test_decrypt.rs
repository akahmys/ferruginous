use lopdf::Document;

fn main() {
    let path = "/Users/jun/Downloads/UnicodeStandard-16.0.pdf";
    match Document::load(path) {
        Ok(mut doc) => {
            println!("Is Encrypted: {}", doc.is_encrypted());
            if doc.is_encrypted() {
                match doc.decrypt("") {
                    Ok(_) => {
                        println!("Decryption Success!");
                        if let Ok(obj) = doc.get_object((6938, 0)) {
                            println!("Sample Object (6938): {:?}", obj);
                        }
                    },
                    Err(e) => println!("Decryption Failed: {:?}", e),
                }
            }
        },
        Err(e) => println!("Load Failed: {:?}", e),
    }
}
