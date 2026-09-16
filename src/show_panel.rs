use crate::self_trait::{CloneSizedDisplay, RectEdge, RectItem, rect_edge_show};

use egui::{Color32, CornerRadius, FontId, Id, Rect, vec2};
// 一个用较大面积展示图片的区域
pub struct DataShow<T: CloneSizedDisplay> {
    pub rect: Rect,
    id: Id,
    show: T::Cloned,
    pub name: String,
}
impl<T: CloneSizedDisplay> RectEdge for DataShow<T> {
    fn edge_rect(&self) -> Rect {
        self.rect
    }
}
impl<T: CloneSizedDisplay> DataShow<T> {
    pub fn new(rect: Rect) -> Self {
        DataShow {
            rect,
            id: Id::NULL,
            show: (T::Cloned::default()),
            name: Default::default(),
        }
    }
    pub fn show(&self, ui: &mut egui::Ui) {
        let mut font_id = FontId::default();
        font_id.size = 18.;
        rect_edge_show(
            self,
            ui,
            CornerRadius::same(20),
            2.,
            self.name.as_ref(),
            self.rect.min + vec2(100., 0.),
            Color32::GRAY,
            Color32::BLACK,
            Color32::WHITE,
            font_id,
        );
        let rect = self.rect;
        if self.id != Id::NULL {
            self.show.show_with_rect(ui, rect);
        }
    }
    fn identity(&self, id: Id) -> bool {
        if self.id == Id::NULL {
            false
        } else {
            self.id == id
        }
    }
    pub fn clear(&mut self) -> Id {
        let id = self.id;
        self.id = Id::NULL;
        id
    }
    pub fn if_same_clear(&mut self, id: Id) {
        if self.identity(id) {
            self.clear();
        }
    }
    pub fn new_disp(&mut self, t: &T) {
        let (show, id) = t.lightclone();
        self.id = id;
        self.show = show;
    }
}
