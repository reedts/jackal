pub struct EvtListContext {
    pub event: u32
}

impl EvtListContext {
    pub fn new(idx: u32) -> Self {
        EvtListContext { event: idx }
    }

    pub fn default() -> Self {
        EvtListContext { event: 0 }
    }
}

