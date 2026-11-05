//! Scrolling a chat with the mouse wheel's button, as a browser does: press
//! it to drop a mark, then the distance from the mark to the pointer sets the
//! speed. A quick click leaves it running until the next click; holding the
//! button stops on release.

use egui::{Color32, Pos2, Stroke, vec2};

use crate::theme::{self, Icon, Palette};

/// Around the mark the pointer rests without scrolling.
const DEAD_ZONE: f32 = 12.0;
/// Points per second for each point past the dead zone.
const PACE: f32 = 6.0;
/// Adds speed with the square of that distance, so a small move stays easy to
/// read by and a long one covers the chat.
const ACCELERATION: f32 = 0.12;
const MAX_SPEED: f32 = 12000.0;
/// A press shorter than this, without a scroll, is a click.
const CLICK_SECONDS: f64 = 0.3;
const MARK_RADIUS: f32 = 16.0;

/// A wheel-button scroll under way in the message list.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Autoscroll {
    anchor: Pos2,
    pressed_at: f64,
    /// Whether the press is still down. Once released after a click, the
    /// scroll runs until something stops it.
    held: bool,
}

impl Autoscroll {
    pub fn new(anchor: Pos2, time: f64) -> Self {
        Self {
            anchor,
            pressed_at: time,
            held: true,
        }
    }

    /// Points per second toward newer messages, negative toward older ones.
    pub fn speed(&self, pointer_y: f32) -> f32 {
        let distance = pointer_y - self.anchor.y;
        let past = (distance.abs() - DEAD_ZONE).max(0.0);
        if past == 0.0 {
            return 0.0;
        }
        (past * PACE + past * past * ACCELERATION)
            .min(MAX_SPEED)
            .copysign(distance)
    }

    /// The scroll after the button came up, `None` if that ended it.
    pub fn released(self, pointer_y: f32, time: f64) -> Option<Self> {
        let clicked = time - self.pressed_at < CLICK_SECONDS && self.speed(pointer_y) == 0.0;
        clicked.then_some(Self {
            held: false,
            ..self
        })
    }

    pub fn held(&self) -> bool {
        self.held
    }

    /// Draws the mark and sets the cursor to the way the list is moving.
    pub fn paint(&self, ui: &egui::Ui, palette: &Palette, pointer_y: f32) {
        let speed = self.speed(pointer_y);
        ui.ctx().set_cursor_icon(if speed < 0.0 {
            egui::CursorIcon::ResizeNorth
        } else if speed > 0.0 {
            egui::CursorIcon::ResizeSouth
        } else {
            egui::CursorIcon::ResizeVertical
        });
        let painter = ui.painter();
        painter.circle(
            self.anchor,
            MARK_RADIUS,
            palette.overlay,
            Stroke::new(1.0, palette.outline),
        );
        painter.circle_filled(self.anchor, 2.0, palette.secondary);
        let lit = |on: bool| -> Color32 {
            if on {
                palette.accent
            } else {
                palette.secondary
            }
        };
        let size = vec2(14.0, 14.0);
        let up = egui::Rect::from_center_size(self.anchor - vec2(0.0, 8.0), size);
        let down = egui::Rect::from_center_size(self.anchor + vec2(0.0, 8.0), size);
        theme::paint_icon(ui, Icon::ChevronUp, up, 14.0, lit(speed < 0.0));
        theme::paint_icon(ui, Icon::ChevronDown, down, 14.0, lit(speed > 0.0));
    }
}

/// Whether a press at `pos` lands on the list itself, not a menu or dialog
/// drawn over it.
pub fn starts_here(ui: &egui::Ui, list: egui::Rect, pos: Pos2) -> bool {
    list.contains(pos) && ui.ctx().layer_id_at(pos) == Some(ui.layer_id())
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::pos2;

    fn at(y: f32) -> Autoscroll {
        Autoscroll::new(pos2(100.0, y), 10.0)
    }

    #[test]
    fn rests_near_the_mark_and_speeds_up_with_distance() {
        let scroll = at(200.0);
        assert_eq!(scroll.speed(200.0), 0.0);
        assert_eq!(scroll.speed(200.0 + DEAD_ZONE), 0.0);
        assert_eq!(scroll.speed(200.0 - DEAD_ZONE), 0.0);
        assert!(scroll.speed(260.0) > scroll.speed(230.0));
        assert!(scroll.speed(230.0) > 0.0);
        assert_eq!(scroll.speed(140.0), -scroll.speed(260.0));
        let gain = |from: f32, to: f32| scroll.speed(200.0 + to) - scroll.speed(200.0 + from);
        assert!(gain(150.0, 200.0) > gain(50.0, 100.0), "it accelerates");
        assert_eq!(scroll.speed(100_000.0), MAX_SPEED);
    }

    #[test]
    fn a_click_keeps_running_and_a_hold_stops_on_release() {
        let scroll = at(200.0);
        let running = scroll.released(201.0, 10.1).expect("a click keeps it");
        assert!(!running.held());
        assert_eq!(scroll.released(201.0, 11.0), None, "held too long");
        assert_eq!(scroll.released(300.0, 10.1), None, "dragged to scroll");
    }
}
