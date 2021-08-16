use unsegen::base::*;
use unsegen::widget::*;

pub struct DayCell {
    day_num: u8,
    selected: bool,
    is_today: bool,
    style: Style,
    focus_style: StyleModifier,
    today_style: StyleModifier,
    focus_symbol: Option<char>,
    today_symbol: Option<char>,
}

impl DayCell {
    const CELL_HEIGHT: usize = 1;
    const CELL_WIDTH: usize = 4;

    pub fn new(day_num: u8) -> Self {
        DayCell {
            day_num,
            selected: false,
            is_today: false,
            style: Style::default(),
            focus_style: StyleModifier::new().bg_color(Color::Blue),
            today_style: StyleModifier::new(),
            focus_symbol: None,
            today_symbol: None,
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn focus_style(mut self, style: StyleModifier) -> Self {
        self.focus_style = style;
        self
    }

    pub fn today_style(mut self, style: StyleModifier) -> Self {
        self.today_style = style;
        self
    }

    pub fn focus_symbol(mut self, symbol: char) -> Self {
        self.focus_symbol = Some(symbol);
        self
    }

    pub fn focus_symbol_opt(mut self, symbol_opt: Option<char>) -> Self {
        self.focus_symbol = symbol_opt;
        self
    }

    pub fn today_symbol(mut self, symbol: char) -> Self {
        self.today_symbol = Some(symbol);
        self
    }

    pub fn today_symbol_opt(mut self, symbol_opt: Option<char>) -> Self {
        self.today_symbol = symbol_opt;
        self
    }

    pub fn select(&mut self) {
        self.selected = true;
    }

    pub fn unselect(&mut self) {
        self.selected = false;
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn is_today(&mut self, is_today: bool) {
        self.is_today = is_today;
    }

    pub fn today(mut self, is_today: bool) -> Self {
        self.is_today(is_today);
        self
    }

    pub fn day_num(&self) -> u8 {
        self.day_num
    }
}

impl Widget for DayCell {
    fn draw(&self, window: Window, hints: RenderingHints) {
        let mut c = Cursor::new(&mut window);
        c.move_by(ColDiff::new(1), RowDiff::new(0));
        if self.is_today {
            c.write(&self.today_symbol.unwrap_or('*').to_string());
        }
        c.style_modifier(if hints.active {
            self.focus_style
        } else {
            StyleModifier::default()
        })
        .write(&self.day_num.to_string());
    }

    fn space_demand(&self) -> Demand2D {
        Demand2D {
            width: ColDemand::exact(DayCell::CELL_WIDTH),
            height: RowDemand::exact(DayCell::CELL_HEIGHT),
        }
    }
}
