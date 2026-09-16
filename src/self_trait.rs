use egui::{
    Color32, CornerRadius, FontId, Id, Painter, Pos2, Rect, Response, RichText, Sense, Stroke,
    StrokeKind, Vec2,
};
use epaint::RectShape;
pub trait RectItem {
    // 可在一个rect中显示的item
    fn show_with_rect(&self, ui: &mut egui::Ui, rect: Rect);
}

pub trait CloneSizedDisplay {
    // 方便多位置显示的trait
    type Cloned: RectItem + Default;
    fn lightclone(&self) -> (Self::Cloned, Id); // 也因此复制的结果可能不带有可用id
}

pub trait PosItem
where
    Self: Default,
{
    // 具有大小和固定位置的item
    // 显示相关函数
    fn show_with_shift_static(&self, painter: Painter, shift_vec: Vec2); // 在一个剪切的painter中绘图。shift_vec为该item的中心相对于实际在屏幕上应处于的中心的shift_vec
    fn show_with_center(&self, ui: &mut egui::Ui, center: Pos2); // 非剪切绘制，要求item实际显示中心为center

    // 更新位置信息
    // fn update_pos(&mut self, delta: Vec2);
    fn update_item_rect(&mut self, rect: Rect); //效果上要保证两者一致，也就是说，如果rect = item_rect + delta，那么update_pos和update_item_rect对item的影响应当相同

    // 获取位置信息
    fn item_rect(&self) -> Rect;
    fn center(&self) -> Pos2;

    // 动作

    fn clicked(&self, response: &Response) -> bool;
    fn double_clicked(&self, response: &Response) -> bool;
}

pub trait DragItem
where
    Self: PosItem,
{
    // 可以拖拽的pos item
    fn id(&self) -> Id; // 为保证拖拽行为的可行性。
    // 原则上要保证唯一性，对于测试类型textitempos来说，string是随机生成的，假定不一致。
}

// 一个item的内容物可以有两个状态，单个的，成组的
pub enum Content<S, G> {
    Single(S),
    Group(G),
}

// 对于成组的，可以有一套方法，提取内部内容以及将外部内容融合进去的方法
pub trait SplitMergeItem<T>
where
    T: IntoContent<Group = Self>,
{
    // 从成组的内容物中内提取内容，留有一个空的group
    fn split(&mut self) -> [Vec<T>; 3];
    // 将内部内容放回group中，一般为此时self为空的group。返回不应放入的内容
    fn embed(&mut self, vvec: [Vec<T>; 3]) -> Vec<T>;
    // 将group放回parent元素中
    fn back_top(self, parent: &mut T);
    // 判断一个外部元素是否有存在的必要
    fn is_valid(outer_type: &T) -> bool;
    // 内部表层item个数
    fn content_num(&self) -> usize;
}
pub trait IntoContent
where
    Self: Sized,
    Self::Group: SplitMergeItem<Self>,
    Self::MergeUsed: Default,
{
    // single group为内容物的两种状态
    // group需要从内部取出item，所以需要group实现splitMergeItem<Self>
    type Single;
    type Group;
    // item融合需要的信息。
    type MergeUsed;
    // 得到内容物，原位置清空
    fn get_content(&mut self) -> Content<Self::Single, Self::Group>;
    // 在没有取出的情况下，判断内容物为single还是group
    fn is_single(&self) -> bool;
    // 用于显示当前的内层目录名。
    fn name(&self) -> &str;
    // 多个item可以相互融合，single + single  = group，
    // 用于判断是是否可用于融合
    fn merge_able(&self, other_item: &Self) -> bool;
    // 如何融合
    fn merge(self, other_item: Self, help_source: &Self::MergeUsed) -> Self;
}

pub trait TextEditable {
    // 可以在显示字符串以及编辑字符串两者之间变化的item
    // 存在这样的问题，到底由谁来识别双击呢？最后方案非重叠响应区域
    fn show_text(&mut self, rect: Rect, center_pos: Pos2, ui: &mut egui::Ui);
    fn editable(&self) -> bool;
    fn set_able(&mut self, able: bool);
}

pub trait MultiRect {
    // 如果一个自建的widget由多个rect构成的话
    // 但感觉不合理，合理的应该将所有的不同组分同一分发，而不是做个中间接口
    // 这里主要是为了将texteditable内部的操作封装起来
    // 上面的trait虽然说了如何绘制，但没有说明如何响应。texteditable 显然是需要接受sense::clicked()的，但是不能将这个响应传出去，
    // 包装成widget是个好主意，但没有什么好主意。
    // 这里sense_rect输入实际可显示区域的rect，以及当前该item的绝对中心位置center_pos
    // 返回vec，其中rect为经过裁剪后实际不同部分可显示的区域，Id为每个子部分的id，sense为每个部分可以响应的sense
    fn sense_rect(&self, disp_rect: Rect, center_pos: Pos2) -> Vec<(Rect, Id, Sense)>;
    // 返回具体某个部分的id，部分sense的响应需要同一部分的id唯一且固定。
    fn get_target_id(&self, target: &str) -> Id;
    // 在位置恒定的情况下的显示函数，需要sense_rect返回的&Vec<Rect>，以及sense_rect的同样的输入center_pos
    fn show_static(&mut self, rect_vec: &Vec<Rect>, center_pos: Pos2, ui: &mut egui::Ui);
    // 在位置主动发生变化下的显示函数，需求同上。但是这个实际没怎么用
    fn show_dynamic(&mut self, rect_vec: &Vec<Rect>, center_pos: Pos2, ui: &mut egui::Ui);
    // 对这个整体需要获取的信息
    // 输入内容需要的响应，外部获取的vec_response。返回内容，具体哪个部分响应的，是否有这个响应。
    // 对于包括texteditable的部分来说，完全可以根据vec_response来获取是否双击了可编辑部分。同时为外部的需要的响应提供统一的接口，
    fn target_response(&mut self, target: &str, vec_respone: &Vec<Response>) -> (usize, bool);
}

use std::hash::Hash;

use crate::{display_size_with_special_fontid, rect_richtext};
pub trait Counterfoil
where
    Self: Sized,
    Self::Proof: Sized,
    Self::Id: Eq + Hash,
{
    // 为正常工作需要顶层或者某一static生存周期的结构永久持有的变量
    // 主要是为了传递textureid. egui中如果将纹理装载入内存，那么如果没有任何变量持有handle，那么textureid无法绘制任何图形。
    type Proof;
    type Source;
    type Id;
    fn new_with_proof(input: Self::Source, ctx: &egui::Context) -> (Self, Self::Proof);
    fn get_proof_id(&self) -> Self::Id;
}

// 如果一个item有明确的rect外轮廓，可使用rect_edge_show为该item制作一个外边框及背景
pub trait RectEdge {
    fn edge_rect(&self) -> Rect;
}
pub fn rect_edge_show<T: RectEdge>(
    t: &T,
    ui: &mut egui::Ui,
    corn: CornerRadius,
    width: f32,
    name: &str,
    name_center_pos: Pos2,
    bg_color: Color32,
    text_color: Color32,
    edge_color: Color32,
    font_id: FontId,
) {
    let edge_rect = t.edge_rect();
    let stroke = Stroke::new(width, edge_color);
    let edge_rect_shape = RectShape::new(edge_rect, corn, bg_color, stroke, StrokeKind::Middle);
    ui.painter().add(edge_rect_shape);
    let size = display_size_with_special_fontid(name, font_id.clone(), ui.ctx());
    let text_rect = Rect::from_center_size(name_center_pos, size);
    rect_richtext(
        ui,
        text_rect,
        RichText::new(name)
            .font(font_id.clone())
            .background_color(bg_color)
            .color(text_color.clone()),
        font_id.into(),
        text_color,
    );
}
