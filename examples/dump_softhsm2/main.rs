use anyhow::{Result};
use std::str::FromStr;

use cryptoki::context::{
    Pkcs11, CInitializeArgs,
};
use cryptoki::object::AttributeType;
use cryptoki::types::AuthPin;
use cryptoki::session::UserType;

fn main() -> Result<()> {

    // change here any PKCS#11 library implementation that you are using.
    let pkcs11client = Pkcs11::new("/usr/lib/softhsm/libsofthsm2.so")?;

    println!("Initializing Cryptoki.");
    pkcs11client.initialize(CInitializeArgs::OsThreads)?;
    println!("Initialized.");

    println!("Library info: {:?}", pkcs11client.get_library_info());

    let interesting_attributes =
      vec![
        AttributeType::Class,
        AttributeType::Label,
        AttributeType::KeyType,
        AttributeType::Sign,
      ];

    let slots = pkcs11client.get_all_slots()?;
    println!("Found {} slots.", slots.len());
    for (num, slot) in slots.iter().enumerate() {
      let slot_info = pkcs11client.get_slot_info(*slot);
      println!("\n{}:\n{:?}", num, slot_info);
      match slot_info {
        Ok(si) if si.token_present() => {
          println!("token info: {:?}", pkcs11client.get_token_info(*slot));
          //println!("mechanisms: {:?}", pkcs11client.get_mechanism_list(*slot));
          match pkcs11client.open_ro_session(*slot) {
            Err(e) => println!("Session open failed: {e:?}"),
            Ok(session) => {
              let secret_pin = AuthPin::from_str("DDSTest_1234").unwrap();
              let login_result =
                session.login(UserType::User, Some(&secret_pin));
              println!("login result: {login_result:?}");
              match login_result {
                Ok(()) => {
                  match session.find_objects(&[]) {
                    Ok(object_handles) => {
                      for (k,obj) in object_handles.iter().enumerate() {
                        println!("  Object {k}");
                        match session.get_attributes(*obj,&interesting_attributes) {
                          Ok(attrs) => {
                            for a in attrs {
                              println!("  {:?}",a );
                            }
                          }
                          Err(e) => println!("{e:?}"),
                        }
                      }
                    }
                    Err(e) => println!("find_objects error: {e:?}"),
                  }
                }
                Err(_) => {}
              }
            }
          }
        }
        Ok(_) => println!("No token."),
        Err(_) => {}
      }
    }
    Ok(())
}