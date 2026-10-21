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
    #[serde(alias = "errors")]
    pub error_counts: ChannelCounts,
    pub max_iterations: usize,
    pub llr_magnitude: f64,
    pub llr_limit: f64,
}

impl Default for LdpcConfig {
    fn default() -> Self {
        Self {
            error_counts: ChannelCounts::default(),
            max_iterations: 50,
            llr_magnitude: 3.0,
            llr_limit: 20.0,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ChannelCounts {
    One(usize),
    Many(Vec<usize>),
}

impl Default for ChannelCounts {
    fn default() -> Self {
        Self::One(1)
    }
}

impl ChannelCounts {
    pub fn points(&self) -> Vec<usize> {
        let mut points = match self {
            Self::One(n) => vec![*n],
            Self::Many(points) => points.clone(),
        };
        points.sort_unstable();
        points
    }

    pub fn validate(&self, n: usize) -> Result<(), String> {
        let points = self.points();
        if points.is_empty()
            || points.iter().any(|&errors| errors > n)
            || points.windows(2).any(|pair| pair[0] == pair[1])
        {
            return Err(format!(
                "channel counts must be nonempty, distinct and within 0..={n}"
            ));
        }
        Ok(())
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
        config
            .repetitions
            .checked_mul(config.ldpc.error_counts.points().len())
            .ok_or("total trials overflow")?;
        Ok(config)
    }
}

impl LdpcConfig {
    pub fn decoder(&self) -> Result<DecoderConfig, String> {
        DecoderConfig::try_new(self.max_iterations, self.llr_limit).map_err(|e| e.to_string())
    }

    pub fn validate(&self, n: usize) -> Result<(), String> {
        self.decoder()?;
        self.error_counts.validate(n)?;
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
                config.ldpc.error_counts.points()
            ),
            (42, 5, 100, vec![1])
        );
        assert!(Config::parse("warmup=0\n[ldpc]\nerrors=0\nmax_iterations=0").is_ok());
        let series = Config::parse("[ldpc]\nerror_counts=[8, 0, 1]").unwrap();
        assert_eq!(series.ldpc.error_counts.points(), [0, 1, 8]);
        for source in [
            "repetitions=0",
            "repetitons=10",
            "log_level='unknown'",
            "[ldpc]\nerrors=129",
            "[ldpc]\nerrors=-1",
            "[ldpc]\nerror_counts=[]",
            "[ldpc]\nerror_counts=[1, 1]",
            "[ldpc]\nerror_counts=[0, 129]",
            "[ldpc]\nerrors=1\nerror_counts=[0, 1]",
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
