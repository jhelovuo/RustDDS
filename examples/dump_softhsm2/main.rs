use anyhow::{Result};
use cryptoki::context::{
    Pkcs11, CInitializeArgs,
};

fn main() -> Result<()> {

    // Create and initialize the PKCS11 client object
    let pkcs11client = Pkcs11::new("/usr/lib/x86_64-linux-gnu/softhsm/libsofthsm2.so")?;

    println!("Initializing.");
    pkcs11client.initialize(CInitializeArgs::OsThreads)?;
    println!("Initialized.");

    println!("Library info: {:?}", pkcs11client.get_library_info());

    let slots = pkcs11client.get_all_slots()?;
    println!("Found {} slots.", slots.len());
    for (num, slot) in slots.iter().enumerate() {
      let slot_info = pkcs11client.get_slot_info(*slot);
      println!("\n{}:\n{:?}", num, slot_info);
      match slot_info {
        Ok(si) if si.token_present() => 
          println!("token info: {:?}", pkcs11client.get_token_info(*slot)),
        Ok(_) => println!("No token."),
        Err(_) => {}
      }
    }
    Ok(())
}