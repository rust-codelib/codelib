use crate::{DecodeInput, DecoderConfig, LdpcError};

pub(crate) fn prepared_channel(
    input: DecodeInput<'_>,
    bits: usize,
    config: DecoderConfig,
) -> Result<Vec<f64>, LdpcError> {
    let mut channel = vec![0.0; bits];
    let mut erased_positions = vec![false; bits];
    crate::decode::prepare_channel(input, &mut channel, &mut erased_positions, config)?;
    Ok(channel)
}
