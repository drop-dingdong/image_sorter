use crate::{
    painter_richtext, random_alphanumeric_string, rect_richtext, rect_shift, self_trait::*,
};
use egui::{
    Color32, CornerRadius, FontSelection, Id, Painter, Pos2, Rect, Response, RichText, Sense,
    TextEdit, UiBuilder, Vec2, vec2,
};
use std::mem::take;

// 由于现阶段不支持将color32作为常量参数，那么这里用uszie作为常量泛型参数，而使用的时候利用该函数转换。
const fn usize2color32(input: usize) -> Color32 {
    match input {
        0 => Color32::RED,
        1 => Color32::BLACK,
        2 => Color32::GREEN,
        3 => Color32::TRANSPARENT,
        4 => Color32::YELLOW,
        5 => Color32::WHITE,
        _ => Color32::BLACK,
    }
}
// 由于文字产生加载非常快，所以使用仅含有文字的固定大小的item来代替图片做初期的测试
// 后来发现修改后可以用于其他load的状态
#[derive(Debug)]
pub struct TextItemPos<const BG: usize, const TXT: usize> {
    pub string: String,
    id: Id,
    rect: Rect,
    editable: bool,
}
pub type LoadingTextItem = TextItemPos<3, 5>; // 设置loading的textitem为背景色透明，字体颜色为白色。
pub type LoadErrTextItem = TextItemPos<1, 0>; // 设置loaderr的textitem为背景色黄色，字体颜色为红色。
impl<const BG: usize, const TXT: usize> Default for TextItemPos<BG, TXT> {
    fn default() -> Self {
        Self::new()
    }
}
impl<const BG: usize, const TXT: usize> TextItemPos<BG, TXT> {
    pub fn trans2othercolor<const BG1: usize, const TXT1: usize>(self) -> TextItemPos<BG1, TXT1> {
        let Self {
            string,
            id,
            rect,
            editable,
        } = self;
        TextItemPos {
            string,
            id,
            rect,
            editable,
        }
    }
    pub fn new() -> Self {
        let str = random_alphanumeric_string(10);
        let rect = Rect::ZERO;
        let id = Id::new(&str);
        let string = str;
        TextItemPos {
            string,
            id,
            rect,
            editable: false,
        }
    }
    pub fn from_str(str: String) -> Self {
        TextItemPos {
            id: Id::new(&str),
            string: str,
            rect: Rect::ZERO,
            editable: false,
        }
    }
}

impl<const BG: usize, const TXT: usize> CloneSizedDisplay for TextItemPos<BG, TXT> {
    type Cloned = String;
    fn lightclone(&self) -> (Self::Cloned, Id) {
        (self.string.clone(), self.id())
    }
}
impl<const BG: usize, const TXT: usize> DragItem for TextItemPos<BG, TXT> {
    fn id(&self) -> Id {
        self.id
    }
}

impl<const BG: usize, const TXT: usize> RectItem for TextItemPos<BG, TXT> {
    fn show_with_rect(&self, ui: &mut egui::Ui, rect: Rect) {
        rect_richtext(
            ui,
            rect,
            RichText::new(&self.string)
                .color(usize2color32(TXT))
                .background_color(usize2color32(BG)),
            FontSelection::Default,
            usize2color32(TXT),
        );
    }
}
impl RectItem for String {
    fn show_with_rect(&self, ui: &mut egui::Ui, rect: Rect) {
        rect_richtext(
            ui,
            rect,
            RichText::new(self).background_color(Color32::BROWN),
            FontSelection::Default,
            Color32::RED,
        );
    }
}
impl<const BG: usize, const TXT: usize> PosItem for TextItemPos<BG, TXT> {
    fn show_with_shift_static(&self, painter: Painter, shift_vec: Vec2) {
        let disp_rect = rect_shift(self.rect, shift_vec);
        painter.rect_filled(disp_rect, CornerRadius::same(1), usize2color32(BG));
        painter_richtext(
            painter,
            disp_rect,
            RichText::new(&self.string)
                .color(usize2color32(TXT))
                .background_color(usize2color32(BG)),
            FontSelection::Default,
            usize2color32(TXT),
        );
    }
    fn show_with_center(&self, ui: &mut egui::Ui, center: Pos2) {
        let rect = Rect::from_center_size(center, self.rect.size());
        rect_richtext(
            ui,
            rect,
            RichText::new(&self.string)
                .color(usize2color32(TXT))
                .background_color(usize2color32(BG)),
            FontSelection::Default,
            usize2color32(TXT),
        );
    }
    fn clicked(&self, response: &Response) -> bool {
        response.clicked()
    }
    fn double_clicked(&self, response: &Response) -> bool {
        !self.is_single() && response.double_clicked()
    }
    fn center(&self) -> Pos2 {
        self.rect.center()
    }
    // fn update_pos(&mut self, delta: Vec2) {
    //     self.rect = rect_shift(self.rect, delta);
    // }
    fn update_item_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    fn item_rect(&self) -> Rect {
        self.rect
    }
}

impl<const BG: usize, const TXT: usize> TextEditable for TextItemPos<BG, TXT> {
    fn show_text(&mut self, text_rect: Rect, center_pos: Pos2, ui: &mut egui::Ui) {
        if self.editable() {
            if ui
                .scope_builder(UiBuilder::new().max_rect(text_rect), |ui| {
                    TextEdit::singleline(&mut self.string).show(ui)
                })
                .inner
                .response
                .lost_focus()
            {
                self.set_able(false);
            }
        } else {
            self.show_with_rect(ui, text_rect);
        }
    }
    fn editable(&self) -> bool {
        self.editable
    }
    fn set_able(&mut self, able: bool) {
        self.editable = able;
    }
}

impl<const BG: usize, const TXT: usize> MultiRect for TextItemPos<BG, TXT> {
    fn sense_rect(&self, disp_rect: Rect, center_pos: Pos2) -> Vec<(Rect, Id, Sense)> {
        let rect = rect_shift(self.rect, center_pos - self.rect.center());
        let text_height = 16.;
        let lt = rect.left_top();
        let rb = rect.right_bottom();
        let content_rect = Rect::from_min_max(lt, rb - vec2(0., text_height));
        let name_rect = Rect::from_min_max(lt + vec2(0., rect.height() - text_height), rb);
        vec![
            (
                disp_rect.intersect(content_rect),
                Id::new((self.id, "content")),
                Sense::click_and_drag(),
            ),
            (
                disp_rect.intersect(name_rect),
                Id::new((self.id, "name")),
                Sense::click(),
            ),
        ]
    }
    fn get_target_id(&self, target: &str) -> Id {
        Id::new((self.id, target))
    }
    fn show_static(&mut self, rect_vec: &Vec<Rect>, center_pos: Pos2, ui: &mut egui::Ui) {
        let disp_rect = rect_vec[0];
        let name_rect = rect_vec[1];
        self.show_with_shift_static(
            ui.painter().with_clip_rect(disp_rect),
            center_pos - self.rect.center(),
        );
        // println!("{} {}", center_pos, disp_rect);
        self.show_text(name_rect, center_pos, ui);
    }
    fn show_dynamic(&mut self, _: &Vec<Rect>, center_pos: Pos2, ui: &mut egui::Ui) {
        self.show_with_center(ui, center_pos);
    }
    fn target_response(&mut self, target: &str, vec_response: &Vec<Response>) -> (usize, bool) {
        // 利用别的通用匹配
        if vec_response[1].double_clicked() {
            self.set_able(true);
        }
        (
            0,
            match target {
                "drag_start" => (vec_response[0].drag_started()),
                "double_clicked" => self.double_clicked(&vec_response[0]),
                "single_clicked" => self.clicked(&vec_response[0]),
                _ => panic!("未设置的情况"),
            },
        )
    }
}

impl<const BG: usize, const TXT: usize> SplitMergeItem<TextItemPos<BG, TXT>> for String {
    fn split(&mut self) -> [Vec<TextItemPos<BG, TXT>>; 3] {
        let bytes = take(self).into_bytes();
        let vec: Vec<_> = bytes
            .chunks(10)
            .map(|chunk| String::from_utf8(chunk.to_vec()).unwrap())
            .map(|s| TextItemPos::from_str(s))
            .collect();
        [vec, vec![], vec![]]
    }
    fn embed(&mut self, vvec: [Vec<TextItemPos<BG, TXT>>; 3]) -> Vec<TextItemPos<BG, TXT>> {
        // 现在数据设计的和ui并没有解耦合，虽然trait看起来没有信息，但是实现的时候，这里要求out选自第三个vec，而app4利用output的时候将其插入到第二个vec的后面。
        let mut vec_str = vvec.map(|vec| {
            vec.into_iter()
                .map(|item| item.string)
                .collect::<Vec<String>>()
                .concat()
        });
        let out = take(&mut vec_str[2]);
        *self = format!("{}{}", *self, vec_str.concat());
        if out.len() != 0 {
            vec![TextItemPos::from_str(out)]
        } else {
            vec![]
        }
    }
    fn back_top(self, parent: &mut TextItemPos<BG, TXT>) {
        parent.string = format!("{}{}", parent.string, self);
        parent.id = Id::new(&parent.string);
    }
    fn is_valid(inner_type: &TextItemPos<BG, TXT>) -> bool {
        inner_type.string.len() != 0
    }
    fn content_num(&self) -> usize {
        let len = self.len();
        let a = len / 10;
        if len % 10 == 0 { a } else { a + 1 }
    }
}

impl<const BG: usize, const TXT: usize> IntoContent for TextItemPos<BG, TXT> {
    type Group = String;
    type Single = String;
    type MergeUsed = ();
    fn name(&self) -> &str {
        self.string.as_str()
    }
    fn is_single(&self) -> bool {
        let len = self.string.len();
        if len == 0 || len > 10 {
            // 如果len = 0视作空容器，len>10作为多个string融合的产物。
            false
        } else {
            true
        }
    }
    fn get_content(&mut self) -> Content<Self::Single, String> {
        let string = take(&mut self.string);
        let len = string.len();
        if len == 0 || len > 10 {
            Content::Group(string)
        } else {
            Content::Single(string)
        }
    }
    fn merge_able(&self, other_item: &Self) -> bool {
        true
    }
    fn merge(mut self, other_item: Self, help_source: &Self::MergeUsed) -> Self {
        let new_str = self.string + (&other_item.string);
        self.string = new_str;
        self.id = Id::new(&self.string);
        self
    }
}
impl<const BG: usize, const TXT: usize> Counterfoil for TextItemPos<BG, TXT> {
    type Proof = (); // TextItemPos本身完全持有字符串，不需要其他地方存在凭证。
    type Source = String;
    type Id = ();
    fn new_with_proof(input: Self::Source, _: &egui::Context) -> (Self, Self::Proof) {
        let res = Self::from_str(input);
        (res, ())
    }
    fn get_proof_id(&self) -> Self::Id {
        ()
    }
}
