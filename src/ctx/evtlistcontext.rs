pub struct EvtListContext {
    pub selected_event: u32
}

impl EvtListContext {
    pub fn new(idx: u32) -> Self {
        EvtListContext { selected_event: idx }
    }

    pub fn default() -> Self {
        EvtListContext { selected_event: 0 }
    }
}

