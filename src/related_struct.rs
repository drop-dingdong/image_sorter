use egui::{Pos2, Rect};
#[derive(Debug)]
// 需要app层级处理的信息
// 假定这些消息是排他的，并假定所有的panel的信息也是排他的。同一帧只能出现一个信息
pub enum Event {
    Dclicked((usize, usize)),
    Sclicked((usize, usize)),
    Back,
    Empty,
}
impl Default for Event {
    fn default() -> Self {
        Event::Empty
    }
}
impl Event {
    fn is_empty(&self) -> bool {
        match self {
            Event::Empty => true,
            _ => false,
        }
    }
    pub fn or(&mut self, event: Event) {
        match (&self, event) {
            (Event::Empty, event) => {
                *self = event;
            }
            (Event::Sclicked((pidx1, iidx1)), event @ Event::Dclicked((pidx2, iidx2)))
                if (*pidx1 == pidx2 && *iidx1 == iidx2) =>
            {
                *self = event;
            }
            (Event::Dclicked((pidx2, iidx2)), Event::Sclicked((pidx1, iidx1)))
                if (pidx1 == *pidx2 && iidx1 == *iidx2) => {}
            (_, event) => {
                if !event.is_empty() {
                    println!("self event {:?}, input event {:?}", self, event);
                    panic!("我假定同一帧内所有的信息必须是排他的，否则当前程序的假设有问题");
                }
            }
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Position {
    Before,
    Middle,
    Behind,
}

#[derive(Debug)]
pub struct DragInfo {
    pub panel_idx: usize,
    pub item_idx: usize,
    pub init_rect: Rect,
    pub disp_center_pos: Pos2,
}
impl DragInfo {
    pub fn new(panel_idx: usize, item_idx: usize, init_rect: Rect, disp_center_pos: Pos2) -> Self {
        DragInfo {
            panel_idx,
            item_idx,
            init_rect,
            disp_center_pos,
        }
    }
}
impl Default for DragInfo {
    fn default() -> Self {
        DragInfo {
            panel_idx: 0,
            item_idx: 0,
            init_rect: Rect::ZERO,
            disp_center_pos: Pos2::ZERO,
        }
    }
}
