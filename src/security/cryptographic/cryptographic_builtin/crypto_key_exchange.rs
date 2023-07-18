use crate::{security::cryptographic::cryptographic_builtin::*, security_error};

impl CryptoKeyExchange for CryptographicBuiltIn {
  fn create_local_participant_crypto_tokens(
    &mut self,
    local_participant_crypto: ParticipantCryptoHandle,
    remote_participant_crypto: ParticipantCryptoHandle,
  ) -> SecurityResult<Vec<ParticipantCryptoToken>> {
    //TODO: this is only a mock implementation (or is it?)
    self
      .encode_keys_
      .get(&remote_participant_crypto)
      .ok_or(security_error!(
        "Could not find encode keys for the handle {}",
        remote_participant_crypto
      ))
      .cloned()
      // Convert to CryptoTokens
      .and_then(Vec::<DatawriterCryptoToken>::try_from)
  }

  fn set_remote_participant_crypto_tokens(
    &mut self,
    local_participant_crypto: ParticipantCryptoHandle,
    remote_participant_crypto: ParticipantCryptoHandle,
    remote_participant_tokens: Vec<ParticipantCryptoToken>,
  ) -> SecurityResult<()> {
    //TODO: this is only a mock implementation (or is it?)
    KeyMaterial_AES_GCM_GMAC_seq::try_from(remote_participant_tokens)
      .and_then(|keymat_seq| self.insert_decode_keys_(remote_participant_crypto, keymat_seq))
  }

  fn create_local_datawriter_crypto_tokens(
    &mut self,
    local_datawriter_crypto: DatawriterCryptoHandle,
    remote_datareader_crypto: DatareaderCryptoHandle,
  ) -> SecurityResult<Vec<DatawriterCryptoToken>> {
    //TODO: this is only a mock implementation (or is it?)

    self
      .encode_keys_
      .get(&remote_datareader_crypto)
      .ok_or(security_error!(
        "Could not find encode keys for the handle {}",
        remote_datareader_crypto
      ))
      .cloned()
      // Convert to CryptoTokens
      .and_then(Vec::<DatawriterCryptoToken>::try_from)
  }

  fn set_remote_datawriter_crypto_tokens(
    &mut self,
    local_datareader_crypto: DatareaderCryptoHandle,
    remote_datawriter_crypto: DatawriterCryptoHandle,
    remote_datawriter_tokens: Vec<DatawriterCryptoToken>,
  ) -> SecurityResult<()> {
    //TODO: this is only a mock implementation
    KeyMaterial_AES_GCM_GMAC_seq::try_from(remote_datawriter_tokens)
      .and_then(|keymat_seq| self.insert_decode_keys_(remote_datawriter_crypto, keymat_seq))
  }

  fn create_local_datareader_crypto_tokens(
    &mut self,
    local_datareader_crypto: DatareaderCryptoHandle,
    remote_datawriter_crypto: DatawriterCryptoHandle,
  ) -> SecurityResult<Vec<DatareaderCryptoToken>> {
    //TODO: this is only a mock implementation (or is it?)

    self
      .encode_keys_
      .get(&remote_datawriter_crypto)
      .ok_or(security_error!(
        "Could not find encode keys for the handle {}",
        remote_datawriter_crypto
      ))
      .cloned()
      // Convert to CryptoTokens
      .and_then(Vec::<DatawriterCryptoToken>::try_from)
  }

  fn set_remote_datareader_crypto_tokens(
    &mut self,
    local_datawriter_crypto: DatawriterCryptoHandle,
    remote_datareader_crypto: DatareaderCryptoHandle,
    remote_datareader_tokens: Vec<DatareaderCryptoToken>,
  ) -> SecurityResult<()> {
    //TODO: this is only a mock implementation
    KeyMaterial_AES_GCM_GMAC_seq::try_from(remote_datareader_tokens)
      .and_then(|keymat_seq| self.insert_decode_keys_(remote_datareader_crypto, keymat_seq))
  }

  fn return_crypto_tokens(&mut self, crypto_tokens: Vec<CryptoToken>) -> SecurityResult<()> {
    //TODO: this is only a mock implementation
    Ok(())
  }
}
