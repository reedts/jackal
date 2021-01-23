pub struct EvtListContext {
    pub event: u32,
}

impl Default for EvtListContext {
    fn default() -> Self {
        EvtListContext { event: 0 }
    }
}

impl EvtListContext {
    pub fn new(idx: u32) -> Self {
        EvtListContext { event: idx }
    }
}
