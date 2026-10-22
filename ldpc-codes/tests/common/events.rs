use coding_core::Observer;
use ldpc_codes::DecodeEvent;

#[derive(Default)]
pub struct Events(pub Vec<DecodeEvent>);

impl Observer<DecodeEvent> for Events {
    fn on_event(&mut self, event: &DecodeEvent) {
        self.0.push(*event);
    }
}
