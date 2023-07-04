pub mod builtin_types;
pub mod cryptographic_builtin;
pub mod cryptographic_plugin;
pub mod types;


// Cryptographic operations are specified as three separate traits,
// but we gather them into one, so that we can implement them in a
// single object. Having three separate interfaces and using them as such
// would force us to have three references to the potentially same object
// under different dyn types, which is too hard to do.
pub trait Cryptographic : 
	cryptographic_plugin::CryptoKeyFactory 
	+ cryptographic_plugin::CryptoKeyExchange 
	+ cryptographic_plugin::CryptoTransform {}