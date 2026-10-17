use rm_codes::{Bit, RmConfig, RmConfigurator, RmDecodeEvent, RmDecodeObserver};

fn bits(text: &str) -> Result<Vec<Bit>, rm_codes::RmError> {
    text.bytes()
        .map(|byte| Bit::try_from(byte - b'0'))
        .collect()
}

struct Log;

impl RmDecodeObserver for Log {
    fn on_event(&mut self, event: &RmDecodeEvent) {
        println!("{event:?}");
    }
}

fn main() -> Result<(), rm_codes::RmError> {
    let config = RmConfig::new(2, 4)?;
    let code = RmConfigurator::build(config);

    let received = bits("1100000101111000")?;
    let mut log = Log;
    let result = code.decode(&received, Some(&mut log))?;

    println!("status = {:?}", result.status());
    println!("message = {:?}", result.message());

    Ok(())
}
