use crate::ui::{EstimatedWidgetSize, WidgetSize};

use tui::layout::{Direction, Rect};

pub(crate) fn center_in<T: WidgetSize>(widget: &T, area: &Rect) -> Option<Rect> {
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

pub(crate) fn estimate_num_fits<T: EstimatedWidgetSize>(
    direction: Direction,
    space: &Rect,
    additional_padding: Option<u16>,
) -> u16 {
    match direction {
        Direction::Horizontal => {
            (space.width + additional_padding.unwrap_or_default()) / T::est_width()
        }
        Direction::Vertical => {
            (space.height + additional_padding.unwrap_or_default()) / T::est_height()
        }
    }
}
