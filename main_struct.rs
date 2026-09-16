use crate::asset::*;
use crate::idx_name::IndexName;
use crate::mix_item::{LoadEnum, LoadState, rename_load_inner};
use crate::related_struct::{DragInfo, Event};
use crate::scroll_drag_panel::FixedDispSizeScrollArea;
use crate::self_trait::*;
use crate::{DataShow, filter_file};
use crate::{thread_load_image, vec_process_with_id_output, vec_remove_item};
use egui::{Key, Modifiers, Rect, Sense, TextureHandle, pos2, vec2};
use image::RgbaImage;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::thread;
use std::{fs, mem};
use tokio::sync::Semaphore;
#[derive(Debug)]
struct OuterStack<C> {
    data: [Vec<C>; 3],
    pos: (usize, usize),
    del_name: String,
}
// 整个gui的主体部分，负责显示item以及拖拽合并，进入，返回。外部app调用的rename以及load操作也都在这里。
// 对于ImageItem和TextItemPos以及LoadState都可以作为MainStruct的泛型参数，
pub struct MainStruct<C: CloneSizedDisplay + DragItem + IntoContent + MultiRect + Counterfoil> {
    disp_list: [FixedDispSizeScrollArea; 3], // 三个滚动显示拖拽区域
    data_list: [Vec<C>; 3],                  // 对应的item序列
    bigg_show: DataShow<C>,                  // 显示item的大区域
    drag_info: Option<DragInfo>,             // 当前正在拖动的item的信息
    info_stack: Vec<OuterStack<C>>,
    proof_hash: HashMap<C::Id, C::Proof>, // 为了显示所必须要持有的内容
    merged_used: C::MergeUsed,            //触发merge操作的时候，需要用到的数据
}
impl MainStruct<LoadState> {
    pub fn new_with_asset(cc: &eframe::CreationContext<'_>) -> Self {
        let mut res = MainStruct::new_with_empty();
        let handle = load_dir_icon(&cc.egui_ctx);
        let hash_map = &mut res.proof_hash;
        let dir_icon_id = handle.id();
        hash_map.insert(
            dir_icon_id,
            LoadEnum::Img((String::new(), String::new(), (handle.clone(), handle))),
        );
        res.merged_used = dir_icon_id;
        res.disp_list[0].name = "Main Panel".to_string();
        res.disp_list[1].name = "Assist Panel".to_string();
        res.disp_list[2].name = "Unrename Panel".to_string();

        res.bigg_show.name = "Img Show".to_string();
        res
    }
    pub fn all_clear(&mut self) {
        let mut new_hash_map = HashMap::with_capacity(1);
        let dir_id = self.merged_used;
        if let Some(dir_texture) = self.proof_hash.remove(&dir_id) {
            new_hash_map.insert(dir_id.clone(), dir_texture);
        } else {
            panic!("未在proof_hash中找到dir相关的纹理信息");
        }
        self.proof_hash = new_hash_map;
        self.info_stack.clear();
        self.drag_info = None;
        self.bigg_show.clear();
        for i in 0..3 {
            self.data_list[i].clear();
        }
    }
    fn not_done_clear(&mut self) {
        // 仅清理那些loading的和loaderror的
        if self.info_stack.len() != 0 {
            panic!("只有顶层时才可使用该函数");
        }
        let data_list_0 = &mut self.data_list[0];
        let len = data_list_0.len();
        for idx in (0..len).rev() {
            match data_list_0[idx] {
                LoadState::LoadErr(_) | LoadState::Loading(_) => {
                    vec_remove_item(data_list_0, idx);
                }
                _ => (),
            }
        }
    }
    pub fn find_loading_and_load_image(
        &mut self,
        name: String,
        texture_id_tuple: (TextureHandle, TextureHandle),
    ) -> bool {
        // 在表层找到与name相同的loading项，加载图片信息，将其变为loaddone
        let mut right = false;
        let first_layer = if self.info_stack.len() > 0 {
            &mut self.info_stack[0].data[0]
        } else {
            &mut self.data_list[0]
        };
        for item in first_layer.iter_mut() {
            // 原位修改item
            if item.is_loading() && item.name() == name {
                let id = texture_id_tuple.0.id();
                item.load_image(texture_id_tuple, self.proof_hash.entry(id));
                right |= true;
                break;
            }
        }
        right
    }
    pub fn find_loading_and_err(&mut self, name: String) -> bool {
        // 在表层找到name相同的loading项，然后将其变为loaderr项
        let mut right = false;
        let first_layer = if self.info_stack.len() > 0 {
            &mut self.info_stack[0].data[0]
        } else {
            &mut self.data_list[0]
        };
        for item in first_layer.iter_mut() {
            // 原位修改item
            if item.is_loading() && item.name() == name {
                item.err_load();
                right |= true;
                break;
            }
        }
        right
    }
    pub fn max_len(&mut self) -> usize {
        // 返回任一层的最大包含的可保存的item个数
        let count = |x: &Vec<LoadState>| x.iter().filter(|e| e.is_loaddone()).count();
        let mut len = count(&self.data_list[0]) + count(&self.data_list[1]);
        for pidx in 0..2 {
            let data_list_i = &self.data_list[pidx];
            for load_i in data_list_i {
                if let LoadState::LoadDone(img) = load_i {
                    match &img.attr {
                        Content::Single(_) => (),
                        Content::Group(grp) => {
                            len = len.max(grp.max_len());
                        }
                    }
                }
            }
        }
        let mut del_len = count(&self.data_list[2]); // 如果原地计算长度的话，需要计算回退的长度
        let stack_len = self.info_stack.len();
        if stack_len != 0 {
            for load_i in &self.data_list[2] {
                // del panel内的group长度会在上层assist panel中重新命名
                if let LoadState::LoadDone(img) = load_i {
                    match &img.attr {
                        Content::Single(_) => (),
                        Content::Group(grp) => {
                            len = len.max(grp.max_len());
                        }
                    }
                }
            }
        }
        for stack_idx in (0..stack_len).rev() {
            let stack_data = &self.info_stack[stack_idx].data;
            len = len.max(del_len + count(&stack_data[0]) + count(&stack_data[1]));
            for pidx in 0..2 {
                let data_list_i = &stack_data[pidx];
                for load_i in data_list_i {
                    if let LoadState::LoadDone(img) = load_i {
                        match &img.attr {
                            Content::Single(_) => (),
                            Content::Group(grp) => {
                                len = len.max(grp.max_len());
                            }
                        }
                    }
                }
            }
            del_len = count(&stack_data[2]);
            if stack_idx != 0 {
                // 除了首层，其他的outer_panel内的数据都要储存，所以首层内unrename_panel数据不用统计其长度影响。
                for load_i in &stack_data[2] {
                    if let LoadState::LoadDone(img) = load_i {
                        match &img.attr {
                            Content::Single(_) => (),
                            Content::Group(grp) => {
                                len = len.max(grp.max_len());
                            }
                        }
                    }
                }
            }
        }
        len
    }
    fn rename(&mut self, idx_name: IndexName) -> Result<(), String> {
        if self.info_stack.len() != 0 {
            panic!("只有顶层时才可使用该函数");
        }

        rename_load_inner(idx_name, &mut self.data_list);
        Ok(())
    }
    pub fn save(&mut self, path: &str, idx_str: &str, idx_name: IndexName) -> Result<(), String> {
        // 作用以新名字在新目录下保存所有应保存的图片
        // 副作用，
        // 如果没有顺利创建新目录，那么没有其他副作用
        // 如果创建了新目录，那么会回到首层，back_first_layer()会将所有处于各层outer/unrename panel中的数据搬到上一层的assist_panel。
        // 如果创建了新目录，但是重命名过程失败了，会将新目录删除，但是上述操作不退回。
        let mut old_dir = path.to_string();
        old_dir.push('/');
        let new_dir = format!("{}/{}/", path, idx_str);
        match fs::create_dir(&new_dir).map_err(|err| format!("{}", err)) {
            Ok(()) => match (|| -> Result<(), String> {
                self.back_first_layer();
                self.rename(idx_name)?;

                let mut hash_map = HashMap::new();
                for (key, val) in self.proof_hash.iter() {
                    match val {
                        LoadEnum::Img(res) => {
                            hash_map.insert(key.clone(), res.clone());
                        }
                        _ => (),
                    };
                }

                for i in 0..2 {
                    let empty_string = String::new();
                    for load_i in &self.data_list[i] {
                        if let LoadState::LoadDone(img) = load_i {
                            img.copy_rename(&hash_map, &empty_string, &new_dir, &old_dir)?;
                        }
                    }
                }
                Ok(())
            })() {
                Ok(()) => Ok(()),
                Err(err) => {
                    fs::remove_dir(&new_dir);
                    Err(err)
                }
            },
            Err(err) => Err(err),
        }
    }
    pub fn load_figure(
        &mut self,
        path: &str,
        ctx: &egui::Context,
    ) -> Result<Receiver<(String, Result<(RgbaImage, RgbaImage), String>)>, String> {
        let (vec_fig_name, receiver) = parallel_load(PathBuf::from(path))?;
        let mut loading_vec: Vec<LoadState> = vec_fig_name
            .into_iter()
            .map(|file_name| LoadState::new_with_proof(file_name, &ctx).0)
            .collect();
        self.disp_list[0].update_pos_for_new_vec(&mut loading_vec);
        self.data_list[0] = loading_vec;
        Ok(receiver)
    }
}
impl<C: CloneSizedDisplay + DragItem + IntoContent + MultiRect + Counterfoil> MainStruct<C> {
    fn new_with_empty() -> MainStruct<C> {
        let hash_map = HashMap::new();
        let disp0 = FixedDispSizeScrollArea::default();
        let disp1 = FixedDispSizeScrollArea::new(
            pos2(910., 60.),
            vec2(390., 340.),
            vec2(120., 120.),
            10.,
            false,
            "",
        );

        let disp2 = FixedDispSizeScrollArea::new(
            pos2(0., 910.),
            vec2(1300., 120.),
            vec2(100., 100.),
            10.,
            false,
            "",
        );
        MainStruct {
            disp_list: [disp0, disp1, disp2],
            data_list: [vec![], vec![], vec![]],
            bigg_show: DataShow::new(Rect::from_min_max(pos2(910., 410.), pos2(1300., 900.))),
            drag_info: None,
            info_stack: Vec::new(),
            proof_hash: hash_map,
            merged_used: C::MergeUsed::default(),
        }
    }
    pub fn inactive_show(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.show(ui, frame);
        let min = self.disp_list[0].lt;
        let max = self.disp_list[2].edge_rect().max;
        let rect = Rect::from_min_max(min, max);
        let _ = ui.allocate_rect(rect, Sense::click_and_drag());
    }

    pub fn new(input: Vec<C::Source>, cc: &eframe::CreationContext<'_>) -> MainStruct<C> {
        let mut empty = Self::new_with_empty();
        let hash_map = &mut empty.proof_hash;
        let num = input.len();
        let mut vec = Vec::with_capacity(num);
        for inp in input {
            vec.push(C::new_with_proof(inp, &cc.egui_ctx));
        }
        let (mut vec, proof_vec): (Vec<C>, Vec<C::Proof>) = vec.into_iter().unzip();
        for (idx, item) in proof_vec.into_iter().enumerate() {
            hash_map.insert(vec[idx].get_proof_id(), item);
        }
        let disp0 = &mut empty.disp_list[0];
        disp0.update_pos_for_new_vec(&mut vec);
        empty.data_list[0] = vec;
        empty
    }
    // 原本用于back_first_layer()的反操作，但因为back_first_layer对空group的删除，所以该方法不可用
    // pub fn special_path_inner(&mut self, mut path: Vec<(usize, usize)>) {
    //     while let Some(pos) = path.pop() {
    //         self.inner_with_pos(pos);
    //     }
    //     self.refresh_all_max_offset();
    // }
    pub fn back_first_layer(&mut self) -> Vec<(usize, usize)> {
        // 从当前位置回到首层，并返回位置信息。
        // 但注意返回过程中会一并将所有outer_panel的内容带回上层assist_panel中，途中会发生group的删除，因此返回的path不可使用。
        let mut res = Vec::with_capacity(self.info_stack.len());
        while self.info_stack.len() > 0
            && let Some(inner_pos) = self.pop_top_layer()
        {
            res.push(inner_pos);
        }
        res
    }
    fn replace_data(&mut self, data: [Vec<C>; 3]) -> [Vec<C>; 3] {
        // 使用一个数据集替代当前显示的数据集，
        // 这个data数据集的来源主要有两个，一个是由为group的个体产生（产生的同时清空自身），另一个是由info_stack得到
        let now_layer = mem::replace(&mut self.data_list, data);
        for pidx in 0..3 {
            self.disp_list[pidx].update_pos_for_new_vec(&mut self.data_list[pidx]);
        }
        now_layer
    }
    // 重置所有panel的滚动offset的上限
    fn refresh_all_max_offset(&mut self) {
        for i in 0..3 {
            self.disp_list[i].update_offset_limit(self.data_list[i].len());
        }
    }
    fn pop_top_layer(&mut self) -> Option<(usize, usize)> {
        // 回到上一层，如果成功返回，则返回旧层相对新层的位置信息
        // 但这个位置信息用处不大，因为回到上一层的过程中，会根据当前item是否有效，而选择删除该item
        if let Some(OuterStack {
            data,
            pos: (pidx, iidx),
            del_name,
        }) = self.info_stack.pop()
        {
            let bot_layer = self.replace_data(data);
            // let bot_layer = mem::replace(&mut self.data_list, data);
            match self.data_list[pidx][iidx].get_content() {
                Content::Group(mut grp) => {
                    let out = grp.embed(bot_layer);
                    SplitMergeItem::back_top(grp, &mut self.data_list[pidx][iidx]);
                    if !<<C as IntoContent>::Group>::is_valid(&self.data_list[pidx][iidx]) {
                        self.bigg_show
                            .if_same_clear(vec_remove_item(&mut self.data_list[pidx], iidx).id());
                        self.disp_list[pidx].update_offset_limit(self.data_list[pidx].len());
                    }
                    let mut len = self.data_list[1].len();
                    for mut outi in out.into_iter() {
                        self.disp_list[1].update_pos(&mut outi, len);
                        self.data_list[1].push(outi);
                        len += 1;
                    }
                    self.disp_list[1].update_offset_limit(len); // 每个这样改变data_vec，都要手动调用disp_list方法。考虑将其转化为disp的一个方法？
                    self.disp_list[2].name = del_name;
                    println!(
                        "now back and the length of assistant disp is {}",
                        self.data_list[1].len()
                    );
                }
                Content::Single(_) => panic!("位置不匹配"),
            }
            Some((pidx, iidx))
        } else {
            None
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let mut event = Event::Empty;
        self.bigg_show.show(ui);
        for i in 0..3 {
            event.or(self.disp_list[i].show(ui, &mut self.data_list[i], &mut self.drag_info, i));
        }

        self.drag_action(ui);
        ui.input_mut(|i| {
            if i.consume_key(Modifiers::NONE, Key::Escape) {
                event.or(Event::Back)
            }
        });
        self.event_process(event);
    }
    fn drag_action(&mut self, ui: &mut egui::Ui) {
        // 根据drag_info，绘制正在拖动的item
        match &mut self.drag_info {
            None => (),
            Some(DragInfo {
                panel_idx,
                item_idx,
                init_rect,
                disp_center_pos,
            }) => {
                let item = &mut self.data_list[*panel_idx][*item_idx];

                let response = ui.interact(
                    *init_rect,
                    item.get_target_id("content"),
                    Sense::click_and_drag(),
                );
                item.show_with_center(ui, *disp_center_pos);
                if response.dragged() {
                    // 处理位移
                    *disp_center_pos += response.drag_delta();
                }
                let center_pos = *disp_center_pos;
                // 处理各个panel的auto_scroll
                let dt = ui.input(|i| i.stable_dt);
                let hover_idx = self.disp_list.iter().position(|i| i.containes(center_pos));
                match hover_idx {
                    None => (),
                    Some(hover_idx) => {
                        self.disp_list[hover_idx].self_scroll(center_pos, dt);
                        ui.ctx().request_repaint();
                    }
                }
                // 处理拖拽结束的情况
                if response.drag_stopped() {
                    match hover_idx {
                        None => {
                            self.drag_info = None;
                        }
                        Some(hover_idx) if hover_idx == *panel_idx => {
                            // 自作用
                            let disp = &mut self.disp_list[hover_idx];
                            let data = &mut self.data_list[hover_idx];
                            let (target_idx, position) = disp.pos_to_inner_relation(center_pos);
                            let elimin_id = vec_process_with_id_output(
                                data,
                                *item_idx,
                                target_idx,
                                position,
                                &self.merged_used,
                            );
                            match elimin_id {
                                None => (),
                                Some((id0, id1)) => {
                                    disp.update_offset_limit(data.len());
                                    self.bigg_show.if_same_clear(id0);
                                    self.bigg_show.if_same_clear(id1);
                                }
                            }
                            self.drag_info = None;
                        }
                        Some(hover_idx) => {
                            // 异作用
                            let orig_disp = &mut self.disp_list[*panel_idx];
                            let orig_data = &mut self.data_list[*panel_idx];
                            let extra_item = vec_remove_item(orig_data, *item_idx);
                            orig_disp.update_offset_limit(orig_data.len());
                            let target_disp = &mut self.disp_list[hover_idx];
                            let target_data = &mut self.data_list[hover_idx];
                            let (target_idx, position) =
                                target_disp.pos_to_inner_relation(center_pos);
                            let drag_idx =
                                target_disp.vec_insert_extra_item(target_data, extra_item);
                            let elimin_id = vec_process_with_id_output(
                                target_data,
                                drag_idx,
                                target_idx,
                                position,
                                &self.merged_used,
                            );
                            match elimin_id {
                                None => (),
                                Some((id0, id1)) => {
                                    target_disp.update_offset_limit(target_data.len());
                                    self.bigg_show.if_same_clear(id0);
                                    self.bigg_show.if_same_clear(id1);
                                }
                            }
                            self.drag_info = None;
                        }
                    }
                }
            }
        }
    }
    fn inner_with_pos(&mut self, pos: (usize, usize)) {
        // 根据pos，进入该item的内部，要求pos本身合法，否则panic，
        // 该pos一般是由drag panel给出，需要item响应double clicked的时候确定只有为group的时候，double_clicked() == true
        let (pidx, iidx) = pos;
        let item = &mut self.data_list[pidx][iidx];
        match item.get_content() {
            Content::Group(mut str) => {
                self.drag_info = None;
                let vec = str.split();
                let now_layer = self.replace_data(vec);
                let name = now_layer[pidx][iidx].name();
                let origin_name =
                    mem::replace(&mut self.disp_list[2].name, format!("Out {} Panel", name));
                self.info_stack.push(OuterStack {
                    data: now_layer,
                    pos,
                    del_name: origin_name,
                });
                // self.replace_data(vec, (pidx, iidx));
            }
            Content::Single(_) => panic!("调用者需保证pos合法"),
        }
    }
    fn event_process(&mut self, event: Event) {
        // 需要在mainstruct的层级处理的操作，且这类操作涉及不同部分ui之间的数据变化。例如drag这种动作虽然也由mainstruct处理，但是不在此处理
        // 其中sclicked dclicked由内部的drag panel给出，back由自身抓取escape获取
        match event {
            // 实际event和drag_info也应该是互斥的
            Event::Sclicked((pidx, iidx)) => self.bigg_show.new_disp(&self.data_list[pidx][iidx]),
            Event::Dclicked((pidx, iidx)) => {
                self.inner_with_pos((pidx, iidx));
            }
            Event::Back => {
                self.drag_info = None;
                self.pop_top_layer();
            }
            _ => (),
        }
    }
}
impl<C: CloneSizedDisplay + DragItem + IntoContent + MultiRect + Counterfoil> eframe::App
    for MainStruct<C>
{
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.show(ui, frame);
    }
}
fn parallel_load(
    dir_path: PathBuf,
) -> Result<
    (
        Vec<(String, String)>,
        Receiver<(String, Result<(RgbaImage, RgbaImage), String>)>,
    ),
    String,
> {
    let result = filter_file(dir_path, &["png", "jpeg", "jpg"])?;
    let direct_return: Vec<_> = result
        .iter()
        .map(|(str0, str1, _)| (str0.clone(), str1.clone()))
        .collect();
    let (sx, tx) = channel();
    thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .max_blocking_threads(8)
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async move {
            let semaphore = Arc::new(Semaphore::new(5));
            let mut join_handles = Vec::new();
            for (name, ext, file) in result.into_iter() {
                let permit = semaphore.clone().acquire_owned().await.unwrap();
                let sx = sx.clone();
                join_handles.push(tokio::spawn(async move {
                    let img_raw_data_tuple = thread_load_image(name.as_str(), ext.as_str(), file);
                    sx.send((name, img_raw_data_tuple));
                    drop(permit);
                }))
            }
            for handle in join_handles {
                handle.await.unwrap();
            }
        });
        println!("all image load done");
    });
    Ok((direct_return, tx))
}
