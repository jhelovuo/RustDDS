use crate::security::cryptographic::cryptographic_builtin::*;

impl CryptoKeyExchange for CryptographicBuiltIn {
  fn create_local_participant_crypto_tokens(
    &mut self,
    local_participant_crypto_handle: ParticipantCryptoHandle,
    remote_participant_crypto_handle: ParticipantCryptoHandle,
  ) -> SecurityResult<Vec<ParticipantCryptoToken>> {
    //TODO: this is only a mock implementation (or is it?)
    self
      .get_encode_key_materials_(&remote_participant_crypto_handle)
      .cloned()
      // Convert to CryptoTokens
      .and_then(Vec::<DatawriterCryptoToken>::try_from)
  }

  fn set_remote_participant_crypto_tokens(
    &mut self,
    local_participant_crypto_handle: ParticipantCryptoHandle,
    remote_participant_crypto_handle: ParticipantCryptoHandle,
    remote_participant_tokens: Vec<ParticipantCryptoToken>,
  ) -> SecurityResult<()> {
    //TODO: this is only a mock implementation (or is it?)
    KeyMaterial_AES_GCM_GMAC_seq::try_from(remote_participant_tokens).and_then(|key_materials| {
      self.insert_decode_key_materials_(remote_participant_crypto_handle, key_materials)
    })
  }

  fn create_local_datawriter_crypto_tokens(
    &mut self,
    local_datawriter_crypto_handle: DatawriterCryptoHandle,
    remote_datareader_crypto_handle: DatareaderCryptoHandle,
  ) -> SecurityResult<Vec<DatawriterCryptoToken>> {
    //TODO: this is only a mock implementation (or is it?)

    self
      .get_encode_key_materials_(&remote_datareader_crypto_handle)
      .cloned()
      // Convert to CryptoTokens
      .and_then(Vec::<DatawriterCryptoToken>::try_from)
  }

  fn set_remote_datawriter_crypto_tokens(
    &mut self,
    local_datareader_crypto_handle: DatareaderCryptoHandle,
    remote_datawriter_crypto_handle: DatawriterCryptoHandle,
    remote_datawriter_tokens: Vec<DatawriterCryptoToken>,
  ) -> SecurityResult<()> {
    //TODO: this is only a mock implementation
    KeyMaterial_AES_GCM_GMAC_seq::try_from(remote_datawriter_tokens).and_then(|key_materials| {
      self.insert_decode_key_materials_(remote_datawriter_crypto_handle, key_materials)
    })
  }

  fn create_local_datareader_crypto_tokens(
    &mut self,
    local_datareader_crypto_handle: DatareaderCryptoHandle,
    remote_datawriter_crypto_handle: DatawriterCryptoHandle,
  ) -> SecurityResult<Vec<DatareaderCryptoToken>> {
    //TODO: this is only a mock implementation (or is it?)

    self
      .get_encode_key_materials_(&remote_datawriter_crypto_handle)
      .cloned()
      // Convert to CryptoTokens
      .and_then(Vec::<DatawriterCryptoToken>::try_from)
  }

  fn set_remote_datareader_crypto_tokens(
    &mut self,
    local_datawriter_crypto_handle: DatawriterCryptoHandle,
    remote_datareader_crypto_handle: DatareaderCryptoHandle,
    remote_datareader_tokens: Vec<DatareaderCryptoToken>,
  ) -> SecurityResult<()> {
    //TODO: this is only a mock implementation
    KeyMaterial_AES_GCM_GMAC_seq::try_from(remote_datareader_tokens).and_then(|key_materials| {
      self.insert_decode_key_materials_(remote_datareader_crypto_handle, key_materials)
    })
  }

  fn return_crypto_tokens(&mut self, crypto_tokens: Vec<CryptoToken>) -> SecurityResult<()> {
    //TODO: this is only a mock implementation
    Ok(())
  }
}
