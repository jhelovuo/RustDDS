use std::fmt::Debug;

#[derive(Debug)]
pub enum ConfigError {
  Parse(String),
  Pkcs7(String),
  Security(String),
  Other(String),
}

impl From<glob::PatternError> for ConfigError {
  fn from(e: glob::PatternError) -> ConfigError {
    ConfigError::Parse(format!("Bad glob pattern: {e:?}"))
  }
}

impl From<serde_xml_rs::Error> for ConfigError {
  fn from(e: serde_xml_rs::Error) -> ConfigError {
    ConfigError::Parse(format!("XML parse error: {e:?}"))
  }
}

pub(crate) fn to_config_error_other<E: Debug + 'static>(
  text: &str,
) -> impl FnOnce(E) -> ConfigError + '_ {
  move |e: E| ConfigError::Other(format!("{}: {:?}", text, e))
}

pub(crate) fn to_config_error_pkcs7<E: Debug + 'static>(
  text: &str,
) -> impl FnOnce(E) -> ConfigError + '_ {
  move |e: E| ConfigError::Pkcs7(format!("{}: {:?}", text, e))
}

pub(crate) fn to_config_error_parse<E: Debug + 'static>(
  text: &str,
) -> impl FnOnce(E) -> ConfigError + '_ {
  move |e: E| ConfigError::Parse(format!("{}: {:?}", text, e))
}

pub(crate) fn parse_config_error(text: String) -> ConfigError {
  ConfigError::Parse(text)
}

pub(crate) fn other_config_error(text: String) -> ConfigError {
  ConfigError::Other(text)
}

pub(crate) fn pkcs7_config_error(text: String) -> ConfigError {
  ConfigError::Pkcs7(text)
}
