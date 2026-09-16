use crate::test_text::LoadingTextItem;
use crate::{
    idx_name::IndexName, random_alphanumeric_string, rect_richtext, rect_shift, self_trait::*,
};
use egui::{
    Color32, ColorImage, FontSelection, Id, Image, Painter, Pos2, Rect, Response, RichText, Sense,
    TextBuffer, TextEdit, TextureHandle, TextureId, UiBuilder, Vec2, load::SizedTexture, pos2,
    vec2,
};
use std::{collections::HashMap, fmt, fs, mem::take};
// 图片的显示相关内容

#[derive(Debug, Default, Clone)]
struct SizeCache {
    text_height: f32,
    center: Vec2, // 图片显示区域的中心相较于item中心的偏移矢量
    size: Vec2,   // 在上述区域中，一个图片最大时，可允许的区域大小
}
#[derive(Debug)]
pub struct ImageItem {
    pub name: String,
    editable: bool,
    item_id: Id,
    rect: Rect,                                   // item 的 rect
    pub texture_id_tuple: (TextureId, TextureId), // 如果可以，可以设置item大小是可以变化的，然后根据item size的大小来挑选 texture。但现在，先保持如此，然后将第二个id用于image_show
    disp_cache: SizeCache,
    pub attr: ImageAttr,
}
type SingleAttr = [usize; 2]; // 单一图片具有的信息是该图片的原始宽高
type ImageAttr = Content<SingleAttr, GroupAttr>;

impl Default for ImageItem {
    fn default() -> Self {
        ImageItem {
            name: String::new(),
            item_id: Id::new(String::new()),
            editable: false,
            rect: Rect::ZERO,
            texture_id_tuple: (TextureId::default(), TextureId::default()),
            disp_cache: SizeCache::default(),
            attr: ImageAttr::default(),
        }
    }
}
impl fmt::Debug for ImageAttr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Content::Single(single) => f.debug_struct("Single Content").field("", &single).finish(),
            Content::Group(grp) => f.debug_struct("Group Content").field("", &grp).finish(),
        }
    }
}
impl Default for ImageAttr {
    fn default() -> Self {
        Content::Group(GroupAttr::default())
    }
}
type ImageFrame = [Vec<ImageItem>; 3];

#[derive(Debug, Default)]
pub struct GroupAttr {
    pub inner: ImageFrame,
    inner_disp: [(Vec2, Vec2); 3], // 含有两部分信息，第一个是距离图片rect中心的相对位移，第二个显示大小
}

impl GroupAttr {
    pub fn max_len(&self) -> usize {
        let mut len = self.len();
        let inner = &self.inner;
        for i in 0..3 {
            let data = &inner[i];
            for img in data {
                match &img.attr {
                    Content::Single(_) => (),
                    Content::Group(grp) => {
                        len = len.max(grp.max_len());
                    }
                }
            }
        }
        len
    }
    fn len(&self) -> usize {
        self.inner.iter().map(|v| v.len()).sum()
    }
    fn disp_len(&self) -> usize {
        // 显示的内部item个数
        self.inner[0].len().min(3)
    }
    fn show(&self, painter: &Painter, screen_disp_center_pos: Pos2) {
        let len = self.disp_len();
        for i in (0..len).rev() {
            let texture_id = self.inner[0][i].texture_id_tuple.0;
            let (rel_center, size) = self.inner_disp[i];
            let center = screen_disp_center_pos + rel_center;
            painter.image(
                texture_id,
                Rect::from_center_size(center, size),
                Rect::from_min_max(pos2(0., 0.), pos2(1., 1.)),
                Color32::WHITE,
            );
        }
    }
}
impl ImageItem {
    pub fn upload_image(
        text_item: &LoadingTextItem,
        texture_handle_tuple: (&TextureHandle, &TextureHandle),
    ) -> Self {
        let mut image_item =
            ImageItem::from_texturehandle(texture_handle_tuple.0, texture_handle_tuple.1);
        image_item.update_item_rect(text_item.item_rect());
        image_item
    }
    pub fn update_id(&mut self) {
        self.item_id = Id::new(&self.name);
    }
    fn from_texturehandle(thumbnailer_handle: &TextureHandle, full_handle: &TextureHandle) -> Self {
        let full_texture_id = full_handle.id();
        let thumbnailer_texture_id = thumbnailer_handle.id();
        let name = full_handle.name();
        let item_id = Id::new(&name);
        let dimension = full_handle.size();
        let attr = Content::Single(dimension); // 假定缩略图和原图的长宽比相等，或者不会太影响显示
        let mut disp_cache = SizeCache::default();
        let texture_id_tuple = (thumbnailer_texture_id, full_texture_id);
        disp_cache.text_height = 16.; // ??
        Self {
            name,
            item_id,
            texture_id_tuple,
            disp_cache,
            attr,
            ..Default::default()
        }
    }
    fn disp_rect(&self) -> Rect {
        // 在没有偏移矢量的情况下，图片的展示区域
        Rect::from_center_size(
            self.rect.center() + self.disp_cache.center,
            self.disp_cache.size,
        )
    }
}

impl ImageAttr {
    fn update_size(&mut self, size: Vec2) {
        // 如果外部的item_size发生变化，那么image_attr或许要发生变化
        match self {
            Content::Single(_) => (),
            Content::Group(grp) => {
                let len = grp.disp_len();
                let size_scale = size * 0.6;
                for i in 0..len {
                    let i_center_rel_pos = (i as f32 - 1.) * size * 0.15;
                    let inner_item = &grp.inner[0][i];
                    if inner_item.is_single()
                        && let Content::Single(img_dig) = inner_item.attr
                    {
                        let real_size = max_image_size(size_scale, img_dig);
                        grp.inner_disp[i] = (i_center_rel_pos, real_size);
                    } else {
                        grp.inner_disp[i] = (i_center_rel_pos, size_scale); // 对于Content::Group 的attr，直接使用全部可显示面积。
                    }
                }
            }
        }
    }
}
fn max_image_size(allowed: Vec2, img_dim: [usize; 2]) -> Vec2 {
    // 在允许的范围内 allowed，尺寸为img_dim的图片等比缩放后最大的显示范围
    let [width, height] = img_dim;
    if width == 0 || height == 0 {
        return Vec2::ZERO;
    }
    let mut range_x = allowed.x;
    let mut range_y = allowed.y;
    let width = width as f32;
    let height = height as f32;
    if range_x * height >= range_y * width {
        range_x = range_y * width / height;
    } else {
        range_y = range_x * height / width;
    }
    vec2(range_x, range_y)
}
fn max_image_vec2(allowed: Vec2, img_dim: Vec2) -> Vec2 {
    // 同上，但是输入的img_dim的高宽类型为f32
    let width = img_dim.x;
    let height = img_dim.y;
    if width <= 0. || height <= 0. {
        return Vec2::ZERO;
    }
    let mut range_x = allowed.x;
    let mut range_y = allowed.y;
    if range_x * height >= range_y * width {
        range_x = range_y * width / height;
    } else {
        range_y = range_x * height / width;
    }
    vec2(range_x, range_y)
}
impl PosItem for ImageItem {
    fn show_with_shift_static(&self, painter: Painter, shift_vec: Vec2) {
        let disp_rect = rect_shift(self.disp_rect(), shift_vec);
        let screen_disp_center_pos = disp_rect.center();
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        painter.image(self.texture_id_tuple.0, disp_rect, uv, Color32::WHITE);
        if let Content::Group(grp) = &self.attr {
            grp.show(&painter, screen_disp_center_pos);
        }
    }
    fn show_with_center(&self, ui: &mut egui::Ui, center: Pos2) {
        let disp_rect = rect_shift(self.disp_rect(), center - self.rect.center());
        Image::new(SizedTexture::new(
            self.texture_id_tuple.0,
            self.disp_cache.size,
        ))
        .paint_at(ui, disp_rect);
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
    fn item_rect(&self) -> Rect {
        self.rect
    }
    // fn update_pos(&mut self, delta: Vec2) {
    //     self.rect = rect_shift(self.rect, delta);
    // }
    fn update_item_rect(&mut self, rect: Rect) {
        let size = rect.size();
        if size != self.rect.size() {
            let text_height = self.disp_cache.text_height;
            let disp_size = vec2(size.x, size.y - text_height);
            let center = vec2(0., -text_height / 2.);
            let real_disp_size = if let Content::Single(img_dim) = &self.attr {
                max_image_size(disp_size, *img_dim)
            } else {
                disp_size
            };
            self.disp_cache.center = center;
            self.disp_cache.size = real_disp_size;
            self.attr.update_size(size);
        }
        self.rect = rect;
    }
}

impl DragItem for ImageItem {
    fn id(&self) -> Id {
        self.item_id // 因此还是需要保证没有重名
    }
}

#[derive(Default, Clone)]
pub struct SelfSizedTexture {
    texture_id: TextureId,
    size: Vec2,
}
impl SelfSizedTexture {
    fn new(texture_id: TextureId, size: Vec2) -> Self {
        SelfSizedTexture { texture_id, size }
    }
}
impl RectItem for SelfSizedTexture {
    fn show_with_rect(&self, ui: &mut egui::Ui, rect: Rect) {
        Image::new(SizedTexture::new(self.texture_id, self.size)).paint_at(
            ui,
            Rect::from_center_size(rect.center(), max_image_vec2(rect.size(), self.size)),
        );
    }
}
impl CloneSizedDisplay for ImageItem {
    type Cloned = SelfSizedTexture;
    fn lightclone(&self) -> (Self::Cloned, Id) {
        let cloned = SelfSizedTexture::new(self.texture_id_tuple.1, self.disp_cache.size);
        let id = self.item_id;
        (cloned, id)
    }
}
impl SplitMergeItem<ImageItem> for GroupAttr {
    fn split(&mut self) -> [Vec<ImageItem>; 3] {
        let inner_data = take(&mut self.inner);
        inner_data
    }
    fn embed(&mut self, vvec: [Vec<ImageItem>; 3]) -> Vec<ImageItem> {
        let [vec0, vec1, out] = vvec;
        self.inner[0] = vec0;
        self.inner[1] = vec1;
        out
    }
    fn back_top(self, parent: &mut ImageItem) {
        parent.attr = Content::Group(self);
        let size = parent.disp_cache.size;
        parent.attr.update_size(size);
    }
    fn is_valid(inner_type: &ImageItem) -> bool {
        match &inner_type.attr {
            Content::Single(_) => true,
            Content::Group(grp) => grp.len() != 0,
        }
    }
    fn content_num(&self) -> usize {
        let content = &self.inner;
        content[0].len() + content[1].len()
    }
}

impl IntoContent for ImageItem {
    type Group = GroupAttr;
    type Single = SingleAttr;
    type MergeUsed = TextureId;
    fn is_single(&self) -> bool {
        match &self.attr {
            Content::Single(_) => true,
            Content::Group(_) => false,
        }
    }
    fn name(&self) -> &str {
        self.name.as_str()
    }
    fn merge_able(&self, _: &Self) -> bool {
        true
    }
    fn get_content(&mut self) -> Content<Self::Single, Self::Group> {
        take(&mut self.attr)
    }
    fn merge(mut self, other_item: Self, texture_id: &Self::MergeUsed) -> Self {
        match &mut self.attr {
            Content::Single(_) => {
                let mut group_item = ImageItem::default();
                group_item.name = random_alphanumeric_string(5);
                group_item.item_id = Id::new(&group_item.name);
                group_item.disp_cache = self.disp_cache.clone();
                group_item.rect = self.rect;
                let mut size = self.rect.size();
                size -= vec2(0., self.disp_cache.text_height);
                group_item.disp_cache.size = size;
                group_item.texture_id_tuple = (*texture_id, *texture_id);
                group_item = group_item.merge(self, texture_id);
                group_item = group_item.merge(other_item, texture_id);
                group_item
            }
            Content::Group(grp) => {
                let len = grp.inner[0].len();
                if len <= 2 {
                    let size = self.disp_cache.size;
                    let i_center_rel_pos = (len as f32 - 1.) * 0.15 * size;
                    if let Content::Single(img_dim) = &other_item.attr {
                        grp.inner_disp[len] =
                            (i_center_rel_pos, max_image_size(size * 0.6, *img_dim));
                    }
                }
                grp.inner[0].push(other_item); // 这里并不更新插入item的rect信息。
                self
            }
        }
    }
}

impl TextEditable for ImageItem {
    // 和textedit的实现基本一致
    fn show_text(&mut self, text_rect: Rect, _center_pos: Pos2, ui: &mut egui::Ui) {
        if self.editable() {
            let response = ui
                .scope_builder(UiBuilder::new().max_rect(text_rect), |ui| {
                    TextEdit::singleline(&mut self.name).show(ui)
                })
                .inner
                .response;
            if response.lost_focus() || response.clicked_elsewhere() {
                self.set_able(false);
            }
        } else {
            rect_richtext(
                ui,
                text_rect,
                RichText::new(&self.name),
                FontSelection::Default,
                Color32::BLACK,
            );
        }
    }
    fn editable(&self) -> bool {
        self.editable
    }
    fn set_able(&mut self, able: bool) {
        // 只有group才可以修改名称
        self.editable = !(self.is_single()) && able;
    }
}
use image::RgbaImage;
impl Counterfoil for ImageItem {
    type Proof = (String, String, (TextureHandle, TextureHandle));
    type Source = (String, String, (RgbaImage, RgbaImage)); // 同texture_id_tuple 前者是缩略图，后者是完整视图。
    type Id = TextureId;
    fn new_with_proof(input: Self::Source, ctx: &egui::Context) -> (Self, Self::Proof) {
        let (filename, ext, (fig0, fig1)) = input;
        let (width, height) = fig0.dimensions();
        let width = width as usize;
        let height = height as usize;
        let thumbnailer_handle = ctx.load_texture(
            &filename,
            ColorImage::from_rgba_unmultiplied([width, height], fig0.into_raw().as_slice()),
            Default::default(),
        );
        let (width, height) = fig1.dimensions();
        let width = width as usize;
        let height = height as usize;
        let full_texture_handle = ctx.load_texture(
            &filename,
            ColorImage::from_rgba_unmultiplied([width, height], fig1.into_raw().as_slice()),
            Default::default(),
        );

        let res = ImageItem::from_texturehandle(&thumbnailer_handle, &full_texture_handle);
        (
            res,
            (filename, ext, (thumbnailer_handle, full_texture_handle)),
        )
    }
    fn get_proof_id(&self) -> Self::Id {
        self.texture_id_tuple.0 // 使用较小的
    }
}
impl MultiRect for ImageItem {
    fn sense_rect(&self, disp_rect: Rect, center_pos: Pos2) -> Vec<(Rect, Id, Sense)> {
        let self_rect = self.rect;
        let rect = rect_shift(self_rect, center_pos - self_rect.center());
        let text_height = 16.;
        let lt = rect.left_top();
        let rb = rect.right_bottom();
        let content_rect = Rect::from_min_max(lt, rb - vec2(0., text_height));
        let name_rect = Rect::from_min_max(lt + vec2(0., rect.height() - text_height), rb);
        vec![
            (
                disp_rect.intersect(content_rect),
                Id::new((self.item_id, "content")),
                Sense::click_and_drag(),
            ),
            (
                disp_rect.intersect(name_rect),
                Id::new((self.item_id, "name")),
                Sense::click(),
            ),
        ]
    }
    fn get_target_id(&self, target: &str) -> Id {
        Id::new((self.item_id, target))
    }
    fn show_static(&mut self, rect_vec: &Vec<Rect>, center_pos: Pos2, ui: &mut egui::Ui) {
        let disp_rect = rect_vec[0];
        let name_rect = rect_vec[1];
        self.show_with_shift_static(
            ui.painter().with_clip_rect(disp_rect),
            center_pos - self.rect.center(),
        );
        self.show_text(name_rect, center_pos, ui);
    }
    fn show_dynamic(&mut self, _: &Vec<Rect>, center_pos: Pos2, ui: &mut egui::Ui) {
        self.show_with_center(ui, center_pos);
    }
    fn target_response(&mut self, target: &str, vec_response: &Vec<Response>) -> (usize, bool) {
        if vec_response[1].double_clicked() {
            self.set_able(true);
        }
        (
            0,
            match target {
                "drag_start" => vec_response[0].drag_started(),
                "double_clicked" => self.double_clicked(&vec_response[0]),
                "single_clicked" => self.clicked(&vec_response[0]),
                _ => panic!("未设置的情况"),
            },
        )
    }
}
// 对vvec的所有item的名称以及内层的item名称，利用idx_name进行重命名，需要在用钱保证idx_name的数量可以满足要求
pub fn rename_inner_layer(mut idx_name: IndexName, vvec: &mut [Vec<ImageItem>; 3]) {
    let cloned = idx_name.clone();
    for i in 0..2 {
        let data = &mut vvec[i];
        for img in data {
            let new_name = idx_name.next();
            match (new_name, &mut img.attr) {
                (None, _) => panic!(),
                (Some(str), Content::Single(_)) => {
                    img.name = str;
                    img.update_id();
                }
                (Some(str), Content::Group(grp)) => {
                    let mut name = img.name.take();
                    name.insert(0, '_');
                    name.insert_str(0, &str);
                    img.name = name;
                    rename_inner_layer(cloned.clone(), &mut grp.inner);
                    img.update_id();
                }
            }
        }
    }
}
impl ImageItem {
    pub fn copy_rename(
        &self,
        proof_hash: &HashMap<<ImageItem as Counterfoil>::Id, <ImageItem as Counterfoil>::Proof>,
        prefix: &str,
        new_dir: &str, // 假定old_str, new_dir末尾有分隔符。
        old_dir: &str,
    ) -> Result<(), String> {
        let mut name = self.name.clone();
        if prefix.len() != 0 {
            name.insert(0, '_');
        }
        name.insert_str(0, prefix);

        match &self.attr {
            Content::Single(_) => {
                let id = self.get_proof_id();
                let (orig_name, ext, _) = proof_hash
                    .get(&id)
                    .map_or(Err(format!("can not find saved data for: {:?}", id)), |x| {
                        Ok(x)
                    })?;
                let len = name.len();
                name.insert(len, '.');
                name.insert_str(len + 1, ext);
                name.insert_str(0, new_dir);
                let new_name = name;
                let mut old_name = orig_name.to_string();
                old_name.insert_str(0, old_dir);
                fs::copy(old_name.as_str(), new_name.as_str()).map_err(|err| {
                    format!("{} old name {}, new name {}", err, old_name, new_name)
                })?;
            }
            Content::Group(grp) => {
                let inner = &grp.inner;
                for i in 0..3 {
                    for img in &inner[i] {
                        img.copy_rename(proof_hash, &name, new_dir, old_dir)?;
                    }
                }
            }
        }
        Ok(())
    }
}
