use ldpc_codes::DecoderConfig;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub seed: u64,
    pub warmup: usize,
    pub repetitions: usize,
    pub log_level: String,
    pub ldpc: LdpcConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            seed: 42,
            warmup: 5,
            repetitions: 100,
            log_level: "info".into(),
            ldpc: LdpcConfig::default(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LdpcConfig {
    pub errors: usize,
    pub max_iterations: usize,
    pub llr_magnitude: f64,
    pub llr_limit: f64,
}

impl Default for LdpcConfig {
    fn default() -> Self {
        Self {
            errors: 1,
            max_iterations: 50,
            llr_magnitude: 3.0,
            llr_limit: 20.0,
        }
    }
}

impl Config {
    pub fn parse(source: &str) -> Result<Self, String> {
        let config: Self = toml::from_str(source).map_err(|e| e.to_string())?;
        if config.repetitions == 0 {
            return Err("repetitions must be positive".into());
        }
        config
            .log_level
            .parse::<log::LevelFilter>()
            .map_err(|e| e.to_string())?;
        config.ldpc.validate(crate::ldpc::N)?;
        Ok(config)
    }
}

impl LdpcConfig {
    pub fn decoder(&self) -> Result<DecoderConfig, String> {
        DecoderConfig::try_new(self.max_iterations, self.llr_limit).map_err(|e| e.to_string())
    }

    pub fn validate(&self, n: usize) -> Result<(), String> {
        self.decoder()?;
        if self.errors > n {
            return Err(format!("errors must be at most {n}"));
        }
        if !self.llr_magnitude.is_finite()
            || self.llr_magnitude <= 0.0
            || self.llr_magnitude > self.llr_limit
        {
            return Err("llr_magnitude must be finite, positive and <= llr_limit".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_invalid_settings() {
        let config = Config::parse("").unwrap();
        assert_eq!(
            (
                config.seed,
                config.warmup,
                config.repetitions,
                config.ldpc.errors
            ),
            (42, 5, 100, 1)
        );
        assert!(Config::parse("warmup=0\n[ldpc]\nerrors=0\nmax_iterations=0").is_ok());
        for source in [
            "repetitions=0",
            "repetitons=10",
            "log_level='unknown'",
            "[ldpc]\nerrors=129",
            "[ldpc]\nerrors=-1",
            "[ldpc]\ntypo=1",
            "[ldpc]\nllr_magnitude=0.0",
            "[ldpc]\nllr_magnitude=inf",
            "[ldpc]\nllr_magnitude=21.0",
            "[ldpc]\nllr_limit=0.0",
        ] {
            assert!(Config::parse(source).is_err(), "accepted {source}");
        }
    }
}
