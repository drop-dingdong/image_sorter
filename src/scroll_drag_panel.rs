use crate::related_struct::{DragInfo, Event, Position};
use crate::self_trait::{DragItem, MultiRect, PosItem, RectEdge, rect_edge_show};

use egui::{Color32, CornerRadius, FontId, Pos2, Rect, Response, Vec2, pos2, vec2};
// 固定显示大小的可滚动页面。
pub struct FixedDispSizeScrollArea {
    pub lt: Pos2,        // 区域左上角位置
    pub size: Vec2,      // 所占矩形区域大小
    pub item_size: Vec2, //一个item占据位置的大小
    vhspace: f32,        // item之间的间距
    direct: bool,        // 滚动页面延伸方向， true y, false x;
    line_num: usize,     // 非滚动方向最大容纳的item数
    offset: Vec2,        //滚动带来的位移值
    max_velocity: f32,   //自动滚动的最大最小速度
    min_velocity: f32,
    max_offset: Vec2,       //滚动带来的最大位移值
    auto_scroll_limit: f32, //触发自动位移距离边界的尺度
    pub name: String,
} // 假定排列如下，左侧上侧为vhspace,然后item双向之间差距为vhspace
impl Default for FixedDispSizeScrollArea {
    fn default() -> Self {
        FixedDispSizeScrollArea {
            lt: pos2(0., 60.),
            size: vec2(900., 840.),
            item_size: vec2(100., 100.),
            vhspace: 10.,
            offset: Vec2::ZERO,
            direct: true,
            line_num: 8,
            max_velocity: 500.,
            min_velocity: 100.,
            max_offset: Vec2::ZERO,
            auto_scroll_limit: 100.,
            name: String::new(),
        }
    }
}
impl RectEdge for FixedDispSizeScrollArea {
    fn edge_rect(&self) -> Rect {
        Rect::from_min_size(self.lt, self.size)
    }
}
impl FixedDispSizeScrollArea {
    pub fn new(
        lt: Pos2,
        size: Vec2,
        item_size: Vec2,
        vhspace: f32,
        direct: bool,
        name: impl Into<String>,
    ) -> Self {
        let offset = Vec2::ZERO;
        let line_num = if direct {
            (size.x / (item_size.x + vhspace)).floor() as usize
        } else {
            (size.y / (item_size.y + vhspace)).floor() as usize
        };
        let max_velocity = 500.;
        let min_velocity = 100.;
        let max_offset = Vec2::ZERO;
        let auto_scroll_limit = 100.;
        let name = name.into();
        FixedDispSizeScrollArea {
            lt,
            size,
            item_size,
            vhspace,
            direct,
            line_num,
            offset,
            max_velocity,
            min_velocity,
            max_offset,
            auto_scroll_limit,
            name,
        }
    }
    fn rect(&self) -> Rect {
        Rect::from_min_size(self.lt, self.size)
    }
    pub fn vec_insert_extra_item<T: PosItem>(
        &mut self,
        vec: &mut Vec<T>,
        mut new_item: T,
    ) -> usize {
        // 向给定数据vec追加一个new_item，并同时初始化new_item的pos
        // 如果有外部的item的插入当前的scroll列，那么在这个操作后，可以使用vec内部的操作进行变化
        let len = vec.len();
        self.update_pos(&mut new_item, len);
        vec.push(new_item);
        self.update_offset_limit(len + 1);
        len
    }
    pub fn update_pos<T: PosItem>(&self, aim_item: &mut T, idx: usize) {
        // 为aim_item赋予指标为idx的item应有的pos
        let main_idx = (idx / self.line_num) as f32;
        let sec_idx = (idx % self.line_num) as f32;
        let bias = vec2(self.vhspace, self.vhspace);
        let unit_size = self.item_size + bias;
        let lt_pos = self.lt
            + if self.direct {
                vec2(sec_idx * unit_size.x, main_idx * unit_size.y) + bias
            } else {
                vec2(main_idx * unit_size.x, sec_idx * unit_size.y) + bias
            };
        let rect = Rect::from_min_size(lt_pos, self.item_size); // 也就是这个rect记录的是该item如果完全显示，实际占据的位置，如果计算当前的显示位置，那么需要减去self.offset
        aim_item.update_item_rect(rect);
    }
    pub fn update_offset_limit(&mut self, len: usize) {
        // 根据当前内部容纳的item数目len 更新max_offset
        let mut main_idx = len / self.line_num;
        let mut sec_idx = len % self.line_num;
        let bias = vec2(self.vhspace, self.vhspace);
        let unit_size = self.item_size + bias;
        if sec_idx != 0 {
            main_idx += 1;
        };
        sec_idx = self.line_num;
        let sec_idx = sec_idx as f32;
        let main_idx = main_idx as f32;
        println!("main_idx {} sec_idx {}", main_idx, sec_idx);
        let rb = if self.direct {
            vec2(sec_idx * unit_size.x, main_idx * unit_size.y)
        } else {
            vec2(main_idx * unit_size.x, sec_idx * unit_size.y)
        };
        self.max_offset = (rb.max(self.size) - self.size).max(Vec2::ZERO);
        self.offset = self.offset.min(self.max_offset); // 考虑该结构在show中对于滚动操作的更新。为了设计更新只对当前指针存在的区域进行更新offset，如果max_offset发生变化，且指针不处在该panel上，那么max_offset的更新不会体现在panel的show上，所以随手更新是好习惯。
    }
    pub fn update_pos_for_new_vec<T: PosItem>(&mut self, vec_item: &mut Vec<T>) {
        // 当前界面如果容纳一个新的item序列 vec_item，对自己以及vec_item更新相关信息，
        for (idx, item) in vec_item.iter_mut().enumerate() {
            self.update_pos(item, idx);
        }
        let len = vec_item.len();
        self.update_offset_limit(len);
    }
    pub fn containes(&self, center: Pos2) -> bool {
        // 是否包含一个点
        let vec = center - self.lt;
        vec2_bigger(vec, Vec2::ZERO) && vec2_bigger(self.size, vec)
    }
    pub fn self_scroll(&mut self, pointer: Pos2, delta_t: f32) {
        // 以pointer触发自动滚动 pointer越接近边界，滚动速度越快
        // if !Rect::from_min_size(self.lt, self.size).contains(pointer) {
        if !self.containes(pointer) {
            return;
        }
        if self.direct {
            let disp_top = pointer.y - self.lt.y;
            let disp_bot = self.size.y - disp_top;
            if disp_top < self.auto_scroll_limit {
                let ratio = (disp_top) / self.auto_scroll_limit;
                let velocity = -1. * (ratio * self.min_velocity + (1. - ratio) * self.max_velocity);
                self.offset.y += velocity * delta_t;
            } else if disp_bot < self.auto_scroll_limit {
                let ratio = (disp_bot) / self.auto_scroll_limit;
                let velocity = (ratio * self.min_velocity + (1. - ratio) * self.max_velocity);
                self.offset.y += velocity * delta_t;
            }
        } else {
            let disp_left = pointer.x - self.lt.x;
            let disp_righ = self.size.x - disp_left;
            if disp_left < self.auto_scroll_limit {
                let ratio = disp_left / self.auto_scroll_limit;
                let velocity = -1. * (ratio * self.min_velocity + (1. - ratio) * self.max_velocity);
                self.offset.x += velocity * delta_t;
            } else if disp_righ < self.auto_scroll_limit {
                let ratio = disp_righ / self.auto_scroll_limit;
                let velocity = ratio * self.min_velocity + (1. - ratio) * self.max_velocity;
                self.offset.x += velocity * delta_t;
            }
        };
        self.offset = self.offset.min(self.max_offset).max(Vec2::ZERO);
    }
    pub fn pos_to_inner_relation(&self, center: Pos2) -> (usize, Position) {
        // 如果一个点center在self滚动区域内，那么他对应的item指标应当和该item的位置关系如何
        // 由于self内部item摆放非常规整，所以任一内部点都可以计算对应的item指标，即便该指标超过外部认为的该区域显示的item个数
        // 有两种思路计算一个点具体对应哪个item，
        //一个是利用当前设计的排序策略，手动计算对应的item位置
        // 另一个是利用重叠面积，计算重叠面积最大的item个体。
        // 如果有跨scrollarea，计算pos的打算，那么dragged或者说draginfo必须是整体维护的一个量，
        // 现在使用第一种思路
        let inner_vec = center - self.lt + self.offset;
        let unit_size = self.item_size + Vec2::splat(self.vhspace);
        let x_idx = (inner_vec.x / unit_size.x).floor() as usize;
        let y_idx = (inner_vec.y / unit_size.y).floor() as usize;
        let (main_idx, sec_idx) = if self.direct {
            (y_idx, x_idx)
        } else {
            (x_idx, y_idx)
        };
        let idx = main_idx * self.line_num + sec_idx;
        // println!("探测结果，作用对象{}, 中心显示位置 {}", idx, center);
        if sec_idx >= self.line_num {
            (idx, Position::Before)
        } else {
            let idx_center = vec2(
                unit_size.x * (x_idx as f32) + self.vhspace,
                unit_size.y * (y_idx as f32) + self.vhspace,
            ) + self.item_size / 2.;
            // println!(
            //     "作用目标中心 {} {} {} {} {} {}",
            //     idx_center, x_idx, y_idx, unit_size, self.vhspace, inner_vec
            // );
            let dist = inner_vec - idx_center;
            let quart_size = self.item_size / 4.;
            let mut direct = self.direct;
            if self.line_num == 1 {
                direct = !direct;
            };
            let (inner, positive) = if direct {
                (dist.x.abs() < quart_size.x, dist.x > 0.)
            } else {
                (dist.y.abs() < quart_size.y, dist.y > 0.)
            };
            if inner {
                (idx, Position::Middle)
            } else if positive {
                (idx, Position::Behind)
            } else {
                (idx, Position::Before)
            }
        }
    }
    pub fn show<T: DragItem + MultiRect>(
        &mut self,
        ui: &mut egui::Ui,
        data: &mut Vec<T>,
        drag_info: &mut Option<DragInfo>,
        panel_idx: usize,
    ) -> Event {
        let rect = Rect::from_min_size(self.lt, self.size);
        let mut font_id = FontId::default();
        font_id.size = 18.;
        rect_edge_show(
            self,
            ui,
            CornerRadius::same(20),
            2.,
            self.name.as_ref(),
            self.lt + vec2(100., 0.),
            Color32::GRAY,
            Color32::BLACK,
            Color32::WHITE,
            font_id,
        );
        let offset = self.offset;
        let shift_vec = -offset;
        let bias = vec2(self.vhspace, self.vhspace);
        let unit_size = self.item_size + bias;
        // 计算可以显示的item的指标范围
        let x_min_idx = (offset.x / unit_size.x).floor() as usize;
        let y_min_idx = (offset.y / unit_size.y).floor() as usize;
        let rb = offset + self.size;
        let x_max_idx = (rb.x / unit_size.x).floor() as usize;
        let y_max_idx = (rb.y / unit_size.y).floor() as usize;
        let (main_min_idx, main_max_idx) = if self.direct {
            (y_min_idx, y_max_idx)
        } else {
            (x_min_idx, x_max_idx)
        };
        let min_idx = main_min_idx * self.line_num;
        let max_idx = ((main_max_idx + 1) * self.line_num).min(data.len());
        // 获取拖拽的item指标，如果没有则设置为容纳的item个数
        let drag_idx = if let Some(drag_info) = drag_info
            && drag_info.panel_idx == panel_idx
        {
            drag_info.item_idx
        } else {
            data.len()
        };
        let mut event = Event::Empty;
        // let disp_bias = self.lt + self.offset;
        for idx in min_idx..max_idx {
            // 利用如果min_idx >= max_idx那么将不返回任何值。
            if idx == drag_idx {
                continue;
            }
            let item = &mut data[idx];
            let center_pos = item.item_rect().center() + shift_vec;
            let item_rect_vec = item.sense_rect(rect, center_pos);
            let (rect_vec, vec_respone): (Vec<Rect>, Vec<Response>) = item_rect_vec
                .into_iter()
                .map(|(rect, id, sense)| (rect, ui.interact(rect, id, sense)))
                .unzip();
            item.show_static(&rect_vec, center_pos, ui);
            if let (rect_idx, drag_start_right) = item.target_response("drag_start", &vec_respone)
                && drag_start_right
            {
                // 如果确实触发了drag_start事件，那么给drag_info赋予新值。
                drag_info.replace(DragInfo::new(
                    panel_idx,
                    idx,
                    rect_vec[rect_idx],
                    center_pos,
                ));
            }
            if let (_, single_click_right) = item.target_response("single_clicked", &vec_respone)
                && single_click_right
            {
                event.or(Event::Sclicked((panel_idx, idx)));
            }
            if let (_, double_click_right) = item.target_response("double_clicked", &vec_respone)
                && double_click_right
            {
                event.or(Event::Dclicked((panel_idx, idx)));
            }
        }
        if let Some(pos) = ui.input(|i| i.pointer.latest_pos())
            && self.containes(pos)
        // 只有鼠标指针在当前panel中，scroll才会引起对应的变化。
        {
            self.offset += ui.input(|i| i.smooth_scroll_delta());
            self.offset = self.offset.min(self.max_offset).max(Vec2::ZERO);
        }
        event
    }
}
fn vec2_bigger(vec0: Vec2, vec1: Vec2) -> bool {
    vec0.x >= vec1.x && vec0.y >= vec1.y
}
