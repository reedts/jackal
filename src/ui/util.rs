use crate::ui::Measure;

use tui::layout::Rect;

pub(crate) fn center_in<T: Measure>(widget: &T, area: &Rect) -> Option<Rect> {
    let (width, height) = (widget.width(), widget.height());

    if width > area.width || height > area.height {
        None
    } else {
        Some(Rect::new(
            area.x + (area.width - width) / 2,
            area.x + (area.height - height) / 2,
            width,
            height,
        ))
    }
}
