use crate::image_item::ImageItem;
use crate::image_item::{GroupAttr, rename_inner_layer};
use crate::mix_item::LoadState::Loading;
use crate::rename::idx_name::IndexName;
use crate::self_trait::TextEditable;
use crate::self_trait::*;
use crate::test_text::{LoadErrTextItem, LoadingTextItem, TextItemPos};
use egui::{Sense, TextBuffer, TextureHandle};

use std::collections::hash_map::Entry;
use std::mem;
// 不阻塞主ui线程时，图片加载的三个状态
pub enum LoadState {
    LoadDone(ImageItem),
    Loading(LoadingTextItem),
    LoadErr(LoadErrTextItem),
}
impl Default for LoadState {
    fn default() -> Self {
        LoadState::Loading(TextItemPos::new())
    }
}
impl LoadState {
    fn new_with_string(str: String) -> Self {
        LoadState::Loading(TextItemPos::from_str(str))
    }
    pub fn is_loading(&self) -> bool {
        match self {
            LoadState::Loading(_) => true,
            _ => false,
        }
    }
    pub fn is_loaddone(&self) -> bool {
        match self {
            LoadState::LoadDone(_) => true,
            _ => false,
        }
    }
    // 将loading转化为loaderr
    pub fn err_load(&mut self) {
        match mem::take(self) {
            LoadState::Loading(txt) => {
                *self = LoadState::LoadErr(txt.trans2othercolor());
            }
            _ => panic!("err_load只可能作用于loadstate::loading状态"),
        }
    }
    // 将loading转化为loaddone，
    pub fn load_image(
        &mut self,
        icon_full_handle: (TextureHandle, TextureHandle),
        proof_entry: Entry<'_, <LoadState as Counterfoil>::Id, <LoadState as Counterfoil>::Proof>,
    ) {
        match self {
            LoadState::Loading(text_item) => {
                let image_item =
                    ImageItem::upload_image(text_item, (&icon_full_handle.0, &icon_full_handle.1));
                if let Some((_, ext)) = text_item.name().rsplit_once('.') {
                    proof_entry.or_insert(LoadEnum::Img((
                        text_item.name().to_string(),
                        ext.to_string(),
                        icon_full_handle,
                    ))); // 更新proof_hash
                    *self = LoadState::LoadDone(image_item);
                } else {
                    panic!("不合法的文件名");
                }
            } // 暂时没有处理错误的，如要那么应该改变输入参数类型。
            _ => {
                panic!("同一item不能load两次以上");
            }
        }
    }
}
// 由于loaderr和loading除去显示效果外，不存在其他差异，所以loadstate的衍生状态只存在两种，一个只和str有关，另一个是只和img有关。
#[derive(Debug, Hash)]
pub enum LoadEnum<I, T> {
    Str(T),
    Img(I),
}
impl<I: PartialEq, T: PartialEq> PartialEq for LoadEnum<I, T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Str(t0), Self::Str(t1)) => t0 == t1,
            (Self::Img(i0), Self::Img(i1)) => i0 == i1,
            _ => false,
        }
    }
}
impl<I: Eq, T: Eq> Eq for LoadEnum<I, T> {}
impl<I, T: Default> Default for LoadEnum<I, T> {
    fn default() -> Self {
        LoadEnum::Str(Default::default())
    }
}
//下面是loadstate的各个trait的实现，大部分很无聊。由于设计的类型都实现了相应的突然爱他，所以大部分根据loadenum 或者loadstate进行匹配，然后返回内层的结果
type LoadClone = LoadEnum<
    <ImageItem as CloneSizedDisplay>::Cloned,
    <LoadErrTextItem as CloneSizedDisplay>::Cloned,
>;

impl RectItem for LoadClone {
    fn show_with_rect(&self, ui: &mut egui::Ui, rect: egui::Rect) {
        match self {
            LoadClone::Img(img) => img.show_with_rect(ui, rect),
            LoadClone::Str(str) => str.show_with_rect(ui, rect),
        }
    }
}
impl CloneSizedDisplay for LoadState {
    type Cloned = LoadClone;
    fn lightclone(&self) -> (Self::Cloned, egui::Id) {
        match self {
            LoadState::LoadDone(img) => {
                let (cloned, id) = img.lightclone();
                (LoadClone::Img(cloned), id)
            }
            LoadState::LoadErr(str) => {
                let (cloned, id) = str.lightclone();
                (LoadClone::Str(cloned), id)
            }
            LoadState::Loading(str) => {
                let (cloned, id) = str.lightclone();
                (LoadClone::Str(cloned), id)
            }
        }
    }
}
impl PosItem for LoadState {
    fn show_with_shift_static(&self, painter: egui::Painter, shift_vec: egui::Vec2) {
        match self {
            LoadState::LoadDone(img) => img.show_with_shift_static(painter, shift_vec),
            LoadState::LoadErr(text) => text.show_with_shift_static(painter, shift_vec),
            LoadState::Loading(text) => text.show_with_shift_static(painter, shift_vec),
        }
    }
    fn show_with_center(&self, ui: &mut egui::Ui, center: egui::Pos2) {
        match self {
            LoadState::LoadDone(img) => img.show_with_center(ui, center),
            LoadState::LoadErr(text) => text.show_with_center(ui, center),
            LoadState::Loading(text) => text.show_with_center(ui, center),
        }
    }
    // fn update_pos(&mut self, delta: egui::Vec2) {
    //     match self {
    //         LoadState::LoadDone(img) => img.update_pos(delta),
    //         LoadState::LoadErr(text) => text.update_pos(delta),
    //         LoadState::Loading(text) => text.update_pos(delta),
    //     }
    // }
    fn update_item_rect(&mut self, rect: egui::Rect) {
        match self {
            LoadState::LoadDone(img) => img.update_item_rect(rect),
            LoadState::LoadErr(text) => text.update_item_rect(rect),
            LoadState::Loading(text) => text.update_item_rect(rect),
        }
    }
    fn item_rect(&self) -> egui::Rect {
        match self {
            LoadState::LoadDone(img) => img.item_rect(),
            LoadState::LoadErr(text) => text.item_rect(),
            LoadState::Loading(text) => text.item_rect(),
        }
    }
    fn center(&self) -> egui::Pos2 {
        match self {
            LoadState::LoadDone(img) => img.center(),
            LoadState::LoadErr(text) => text.center(),
            LoadState::Loading(text) => text.center(),
        }
    }
    fn clicked(&self, response: &egui::Response) -> bool {
        match self {
            LoadState::LoadDone(img) => img.clicked(response),
            LoadState::LoadErr(text) => text.clicked(response),
            LoadState::Loading(text) => text.clicked(response),
        }
    }
    fn double_clicked(&self, response: &egui::Response) -> bool {
        // 只有img有可能响应双击
        match self {
            LoadState::LoadDone(img) => img.double_clicked(response),
            _ => false,
        }
    }
}
impl DragItem for LoadState {
    fn id(&self) -> egui::Id {
        match self {
            LoadState::LoadDone(img) => img.id(),
            LoadState::LoadErr(text) => text.id(),
            LoadState::Loading(text) => text.id(),
        }
    }
}

impl TextEditable for LoadState {
    fn show_text(&mut self, rect: egui::Rect, center_pos: egui::Pos2, ui: &mut egui::Ui) {
        match self {
            LoadState::LoadDone(img) => img.show_text(rect, center_pos, ui),
            LoadState::LoadErr(txt) => txt.show_text(rect, center_pos, ui),
            LoadState::Loading(txt) => txt.show_text(rect, center_pos, ui),
        }
    }
    fn editable(&self) -> bool {
        // editable的初衷是为了给show_text用的，既然show_text利用内部实现，那么editable对loadstate的实现没有任何意义
        match self {
            LoadState::LoadDone(img) => img.editable(),
            LoadState::LoadErr(txt) => txt.editable(),
            LoadState::Loading(txt) => txt.editable(),
        }
    }
    fn set_able(&mut self, able: bool) {
        // 不支持对loading loaderr的修改
        // 由于imageitem textitempos内editable的初始值都为false，因此对于两者，不存在可编辑的可能
        match self {
            LoadState::LoadDone(img) => img.set_able(able),
            _ => (),
        }
    }
}

impl MultiRect for LoadState {
    fn sense_rect(
        &self,
        disp_rect: egui::Rect,
        center_pos: egui::Pos2,
    ) -> Vec<(egui::Rect, egui::Id, egui::Sense)> {
        // 在设计中loading loaderr除了single click外不响应任何操作，
        match self {
            LoadState::LoadDone(img) => img.sense_rect(disp_rect, center_pos),
            LoadState::Loading(txt) => {
                let mut res = txt.sense_rect(disp_rect, center_pos);
                res[0].2 = Sense::click();
                res[1].2 = Sense::empty();
                res
            }
            LoadState::LoadErr(txt) => {
                let mut res = txt.sense_rect(disp_rect, center_pos);
                res[0].2 = Sense::click();
                res[1].2 = Sense::empty();
                res
            }
        }
    }
    fn get_target_id(&self, target: &str) -> egui::Id {
        // 在设计中loading loaderr除了single click外不响应任何操作，
        // 所以get_target_id随便返回一个即好
        match self {
            LoadState::LoadDone(img) => img.get_target_id(target),
            LoadState::Loading(txt) => txt.get_target_id(target),
            LoadState::LoadErr(txt) => txt.get_target_id(target),
        }
    }
    fn show_static(
        &mut self,
        rect_vec: &Vec<egui::Rect>,
        center_pos: egui::Pos2,
        ui: &mut egui::Ui,
    ) {
        match self {
            LoadState::LoadDone(img) => img.show_static(rect_vec, center_pos, ui),
            LoadState::LoadErr(txt) => txt.show_static(rect_vec, center_pos, ui),
            LoadState::Loading(txt) => txt.show_static(rect_vec, center_pos, ui),
        }
    }
    fn show_dynamic(
        &mut self,
        rect_vec: &Vec<egui::Rect>,
        center_pos: egui::Pos2,
        ui: &mut egui::Ui,
    ) {
        match self {
            LoadState::LoadDone(img) => img.show_dynamic(rect_vec, center_pos, ui),
            LoadState::LoadErr(txt) => txt.show_dynamic(rect_vec, center_pos, ui),
            LoadState::Loading(txt) => txt.show_dynamic(rect_vec, center_pos, ui),
        }
    }
    fn target_response(
        &mut self,
        target: &str,
        vec_respone: &Vec<egui::Response>,
    ) -> (usize, bool) {
        // 在设计中loading loaderr除了single click外不响应任何操作，
        match self {
            LoadState::LoadDone(img) => img.target_response(target, vec_respone),
            _ => (
                0,
                match target {
                    "single_clicked" => self.clicked(&vec_respone[0]),
                    "drag_start" | "double_clicked" => false,
                    _ => panic!("未设置的情况"),
                },
            ),
        }
    }
}
type Filename = (String, String); // 前者为完整文件名,后者为扩展名
impl Counterfoil for LoadState {
    // 对于loadstate来说，counterfoil有点问题，
    // 因为loadstate的img相关内容是后续添加的，而不是原本就有的。source是作为loading一开始传入的，而id则为后续保存中用于查询原本的信息的变量
    // 产生的时间不同，因此也无法和之前的imageitem textitempos一样有个好的定义。
    // 设计中，整体类型要求同imageitem较为接近。
    // 总的来说，这个counterfoil是纯粹为了实现而实现，或者说单纯为了满足main_struct的trait要求
    // 实际上proof_hash确实有必要存在，但似乎没有什么好的方法统一旧有的imageitem textitempos和现在的loadstate
    // 在loadstate中
    type Proof = LoadEnum<(String, String, (TextureHandle, TextureHandle)), ()>;
    type Source = Filename; // 是否直接用texturehandle输入？
    type Id = egui::TextureId;
    fn new_with_proof(input: Self::Source, _: &egui::Context) -> (Self, Self::Proof) {
        let (name, _) = input;
        (Loading(TextItemPos::from_str(name)), Default::default())
    }
    fn get_proof_id(&self) -> Self::Id {
        match self {
            LoadState::LoadDone(img) => img.get_proof_id(),
            _ => Default::default(),
        }
    }
}

fn only_image(state_vec: Vec<LoadState>) -> Vec<ImageItem> {
    let res: Vec<ImageItem> = state_vec
        .into_iter()
        .map(|x| match x {
            LoadState::LoadDone(img) => Some(img),
            _ => None,
        })
        .flatten()
        .collect();
    res
}
fn box_load_img_vec(img_vec: Vec<ImageItem>) -> Vec<LoadState> {
    img_vec
        .into_iter()
        .map(|x| LoadState::LoadDone(x))
        .collect()
}

type LoadMergedUsed = <ImageItem as IntoContent>::MergeUsed;

impl SplitMergeItem<LoadState> for GroupAttr {
    // 按照设计，只能对imageitem进行进出，所以实际上inner和outer是不对陈的，合并分拆的过程中，始终保证内部是合法的image 序列，所以从内部返回外部的时候需要通过only_image box_load_img_vec转换
    // 也因此将group设置为imageitem的同样的group
    fn split(&mut self) -> [Vec<LoadState>; 3] {
        <Self as SplitMergeItem<ImageItem>>::split(self).map(|a| box_load_img_vec(a))
    }
    fn is_valid(inner_type: &LoadState) -> bool {
        match inner_type {
            LoadState::LoadDone(img) => <GroupAttr as SplitMergeItem<ImageItem>>::is_valid(img),
            _ => false,
        }
    }
    fn embed(&mut self, vvec: [Vec<LoadState>; 3]) -> Vec<LoadState> {
        box_load_img_vec(self.embed(vvec.map(|a| only_image(a))))
    }
    fn back_top(self, parent: &mut LoadState) {
        match parent {
            LoadState::LoadDone(img) => {
                <GroupAttr as SplitMergeItem<ImageItem>>::back_top(self, img)
            }
            LoadState::LoadErr(_) | LoadState::Loading(_) => {
                panic!("对于混合态，设计时应避免text合并")
            }
        }
    }
    fn content_num(&self) -> usize {
        <GroupAttr as SplitMergeItem<ImageItem>>::content_num(self)
    }
}
impl IntoContent for LoadState {
    type Single = ();
    type Group = GroupAttr;
    type MergeUsed = LoadMergedUsed;
    fn get_content(&mut self) -> Content<Self::Single, Self::Group> {
        match self {
            LoadState::LoadDone(img) => match img.get_content() {
                Content::Single(_) => Content::Single(()),
                Content::Group(grp) => Content::Group(grp),
            },
            LoadState::LoadErr(_) | LoadState::Loading(_) => Content::Single(()),
        }
    }
    fn is_single(&self) -> bool {
        match self {
            LoadState::LoadDone(img) => img.is_single(),
            LoadState::LoadErr(_) | LoadState::Loading(_) => true,
        }
    }
    fn name(&self) -> &str {
        match self {
            LoadState::LoadDone(img) => img.name(),
            LoadState::LoadErr(text) => text.name(),
            LoadState::Loading(text) => text.name(),
        }
    }
    fn merge_able(&self, other_item: &Self) -> bool {
        match (self, other_item) {
            (LoadState::LoadDone(_), LoadState::LoadDone(_)) => true,
            _ => false,
        }
    }
    fn merge(self, other_item: Self, help_source: &Self::MergeUsed) -> Self {
        match (self, other_item) {
            (LoadState::LoadDone(img0), LoadState::LoadDone(img1)) => {
                LoadState::LoadDone(img0.merge(img1, help_source))
            }
            _ => panic!("对于混合态，设计时应避免text合并"),
        }
    }
}
// 对vvec中的所有item，不论表层还是内层，利用idx_name进行重命名
pub fn rename_load_inner(mut idx_name: IndexName, vvec: &mut [Vec<LoadState>; 3]) {
    let cloned = idx_name.clone();
    for i in 0..2 {
        let data = &mut vvec[i];
        for load_i in data {
            if let LoadState::LoadDone(img) = load_i {
                let new_name = idx_name.next();
                match (new_name, &mut img.attr) {
                    (None, _) => panic!(),
                    (Some(new_name), Content::Single(_)) => {
                        img.name = new_name;
                        img.update_id();
                    }
                    (Some(new_name), Content::Group(grp)) => {
                        let mut name = img.name.take();
                        name.insert(0, '_');
                        name.insert_str(0, &new_name);
                        img.name = name;
                        rename_inner_layer(cloned.clone(), &mut grp.inner);
                        img.update_id();
                    }
                }
            }
        }
    }
}
