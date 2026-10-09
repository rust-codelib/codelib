use ldpc_codes::{
    DecodeEvent, DecodeInput, DecodeObserver, Decoder, DecoderConfig, LdpcError, ParityCheckMatrix,
    SpaDecoder,
};

struct StdoutObserver;

impl DecodeObserver for StdoutObserver {
    fn on_event(&mut self, event: &DecodeEvent) {
        match event {
            DecodeEvent::Started { unsatisfied_checks } => {
                println!("Начальное решение: нарушено проверок {unsatisfied_checks}");
            }
            DecodeEvent::IterationFinished {
                iteration,
                unsatisfied_checks,
                changed_bits,
            } => {
                println!(
                    "Итерация {iteration}: нарушено проверок {unsatisfied_checks}; позиций, отличающихся от начального решения: {changed_bits}"
                );
            }
            DecodeEvent::Finished {
                iterations,
                status,
                unsatisfied_checks,
                changed_bits,
            } => {
                println!(
                    "Завершено: статус {status:?}, итераций {iterations}, нарушено проверок {unsatisfied_checks}; позиций, отличающихся от начального решения: {changed_bits}"
                );
            }
        }
    }
}

fn main() -> Result<(), LdpcError> {
    let checks = ParityCheckMatrix::try_from_rows(3, vec![vec![0, 1], vec![1, 2]])?;
    let config = DecoderConfig::try_new(2, 20.0)?;
    let mut decoder = SpaDecoder::try_new(checks, config)?;
    let llrs = [2.0, -3.0, 4.0];
    let mut observer = StdoutObserver;

    let result = decoder.decode(
        DecodeInput {
            llrs: &llrs,
            erasures: &[],
        },
        Some(&mut observer),
    )?;

    println!("Итоговый статус: {:?}", result.status());
    println!("Итоговый синдром: {:?}", result.syndrome());
    println!(
        "Изменившихся относительно начального решения позиций: {}",
        result.changed_bits()
    );
    Ok(())
}
