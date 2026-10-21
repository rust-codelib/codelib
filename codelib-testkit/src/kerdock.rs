use std::borrow::Cow;

use codelib_testkit::{
    candidate, report::PointReport, BlockLayout, BlockScenario, Channel, CodecOps, Decoded,
};
use coding_core::{Decoder, Encoder, NoopObserver};
use kerdock_codes::{Kerdock, KerdockSize};
use rand::Rng;

use crate::{config::Config, smoke::check};

pub const N: usize = 64;

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub error_counts: crate::config::ChannelCounts,
}

pub fn run(config: &Config) -> Result<Vec<PointReport>, String> {
    let code = Kerdock::new(KerdockSize::Bits64);
    let layout = BlockLayout {
        message_len: code.message_len(),
        codeword_len: code.codeword_len(),
        payload_bits: code.message_len(),
        error_unit: "bits",
    };
    let mut scenario = BlockScenario::new(
        code,
        layout,
        Channel::BitFlips(|bit: &bool| !bit),
        CodecOps {
            symbol: |rng| rng.random::<bool>(),
            buffers: |_, _| (),
            encode: |c, trial| Encoder::encode(c, trial.message.as_slice(), &mut NoopObserver),
            encoded: |c, trial, encoded| {
                let word = &encoded.as_ref().map_err(ToString::to_string)?.0;
                check(
                    c.extract_message(word).map_err(|e| e.to_string())? == trial.message,
                    "encoder changed the message",
                )?;
                Ok(word)
            },
            prepare: |_, _, _, received| {
                received
                    .iter()
                    .map(|bit| {
                        bit.ok_or_else(|| "Kerdock bit-flip input contains an erasure".into())
                    })
                    .collect::<Result<Vec<_>, String>>()
            },
            decode: |c, word| Decoder::decode(c, word.as_slice(), &mut NoopObserver),
            decoded: |_, _, decoded| {
                Ok(candidate(decoded)?.map(|c| Decoded {
                    word: Some(c.value.word()),
                    message: Some(Cow::Borrowed(c.value.message())),
                    iterations: None,
                }))
            },
        },
    );
    scenario.run(
        "kerdock-codes",
        config.seed,
        config.repetitions,
        config.warmup,
        &config.kerdock.error_counts.points(),
    )
}
