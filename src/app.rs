use crate::dir::InputState;
use crate::input_panel::{InputItem, calculate_rect, dir_input_ui, rename_input_ui};
use crate::mix_item::LoadState;
use crate::{MainStruct, idx_name::RenamePatternState};
use egui::{Color32, ColorImage, CornerRadius, Pos2, Rect, Response, pos2, vec2};
use image::RgbaImage;

use std::sync::mpsc::{Receiver, TryRecvError};
// app的主要内容，因为图片加载的方式，最终的app struct有两个，imagesorter和imagesorted

#[derive(Debug, PartialEq, Eq)]
pub enum AppState {
    Start,
    Sort,
    Finish,
}
impl AppState {
    pub fn is_sort(&self) -> bool {
        match self {
            AppState::Sort => true,
            _ => false,
        }
    }
}

pub struct ImageSorter {
    top_panel: [InputItem; 2],
    top_panel_rect: Option<([Rect; 4], [Rect; 5])>,
    main_content: MainStruct<LoadState>,
    rename_pattern: RenamePatternState,
    state: AppState,
    reciver: Option<Receiver<(String, Result<(RgbaImage, RgbaImage), String>)>>, // 考虑到如果在完全载入之前就已经出现位置移动，所以使用usize作为识别位置是不合适的，所以这里使用string作为判别标志。因为Loading的项不允许重命名，所以使用string还是安全的。
}
impl ImageSorter {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // 与之前的imagesorted基本一致
        let dir_input = InputItem::new_with_id_rect(
            "directory input",
            Rect::from_min_size(pos2(30., 15.), vec2(400., 30.)),
        );
        let rename_input = InputItem::new_with_id_rect(
            "rename pattern input",
            Rect::from_min_size(pos2(30., 15.), vec2(400., 30.)),
        );
        let top_panel = [dir_input, rename_input];
        let main_content = MainStruct::<LoadState>::new_with_asset(cc);
        let state = AppState::Start;
        let top_panel_rect = None;
        ImageSorter {
            main_content,
            top_panel,
            state,
            top_panel_rect,
            reciver: None,
            rename_pattern: Default::default(),
        }
    }
    fn if_reciver(&mut self, ui: &mut egui::Ui) {
        // 如果reciver字段有值，那么尝试接受图片信息，并将对应的item变更为loaddone或者loaderr
        if let Some(reciver) = &self.reciver {
            ui.request_repaint(); // 在reciver存活期间，持续重绘，
            match reciver.try_recv() {
                Ok((name, Ok(img_tuple))) => {
                    let (icon_fig, full_fig) = img_tuple;
                    let (width, height) = icon_fig.dimensions();
                    let icon_handle = ui.ctx().load_texture(
                        name.as_str(),
                        ColorImage::from_rgba_unmultiplied(
                            [width as usize, height as usize],
                            icon_fig.as_raw().as_slice(),
                        ),
                        Default::default(),
                    );
                    let (width, height) = full_fig.dimensions();
                    let full_handle = ui.ctx().load_texture(
                        name.as_str(),
                        ColorImage::from_rgba_unmultiplied(
                            [width as usize, height as usize],
                            full_fig.as_raw().as_slice(),
                        ),
                        Default::default(),
                    );
                    self.main_content
                        .find_loading_and_load_image(name, (icon_handle, full_handle));
                }
                Ok((name, Err(_))) => {
                    self.main_content.find_loading_and_err(name);
                }
                Err(TryRecvError::Empty) => (),
                Err(TryRecvError::Disconnected) => {
                    self.reciver.take();
                }
            }
        }
    }
    fn button_show(&mut self, ui: &mut egui::Ui) -> Response {
        // 用于绘制顶部的输入框以及对应的按钮
        let full_top_panel_rect = Rect::from_min_max(Pos2::ZERO, pos2(1300., 1030.));
        let top_panel_rect_tuple = if let Some(top_panel_rect_tuple) = self.top_panel_rect {
            top_panel_rect_tuple
        } else {
            let res = calculate_rect(&self.top_panel, &ui.ctx());
            self.top_panel_rect = Some(res.clone());
            res
        };
        ui.painter_at(full_top_panel_rect).rect_filled(
            full_top_panel_rect,
            CornerRadius::default(),
            Color32::GRAY,
        );
        dir_input_ui(&mut self.top_panel[0], &top_panel_rect_tuple.0, ui);
        rename_input_ui(
            &mut self.top_panel[1],
            &top_panel_rect_tuple.1,
            &mut self.rename_pattern,
            &self.state,
            ui,
        )
    }
    fn show(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let response = self.button_show(ui);
        // 除了sort阶段，其他阶段不响应response的clicked，因为只有在排序完成的阶段才能进行有效校检。
        match self.state {
            AppState::Start => {
                self.main_content.inactive_show(ui, frame);
                match self.top_panel[0].state {
                    InputState::Verified => {
                        let dir_path = self.top_panel[0].update_cache();
                        match self.main_content.load_figure(dir_path, &ui.ctx()) {
                            Ok(rx) => {
                                self.reciver = Some(rx);
                                self.state = AppState::Sort;
                                self.top_panel[0].state = InputState::InActive;
                            }
                            Err(_) => (),
                        };
                    }
                    InputState::InActive => (),
                    _ => (),
                }
            }
            AppState::Sort => {
                self.main_content.show(ui, frame);
                self.if_reciver(ui);
                // 首先响应rename的动作
                if response.clicked()
                    && let RenamePatternState::Iter(iter) = &self.rename_pattern
                {
                    let num = self.main_content.max_len();
                    // 二次校检 rename pattern 是否有效
                    if num < iter.max_size as usize {
                        let dir_str = self.top_panel[0].get_cache();
                        let rename_str = self.top_panel[1].get_cache();
                        //rename
                        match self.main_content.save(dir_str, rename_str, iter.clone()) {
                            Err(err) => {
                                self.rename_pattern =
                                    RenamePatternState::Err(format!("save error! {}", err));
                            }
                            Ok(()) => {
                                self.reciver.take();
                                self.state = AppState::Finish;
                            }
                        };
                    } else {
                        // 返回错误信息，并将main_content的内容重置
                        self.rename_pattern = RenamePatternState::Err(
                            "beyond max number of input pattern".to_string(),
                        );
                    }
                    self.top_panel[1].state = InputState::InActive;
                } else {
                    match self.top_panel[0].state {
                        InputState::Verified => {
                            // 重新输入后，verified，则清除信息，将state转为start
                            // 在下次计算的时候再处理重新加载的问题。
                            self.main_content.all_clear();
                            self.reciver.take();
                            self.state = AppState::Start;
                        }
                        _ => (),
                    }
                }
            }
            AppState::Finish => {
                self.main_content.inactive_show(ui, frame);
                match self.top_panel[0].state {
                    InputState::Verified => {
                        // 重新输入后，verified，则清除信息，将state转为start
                        // 在下次计算的时候再处理重新加载的问题。
                        self.main_content.all_clear();
                        self.state = AppState::Start;
                    }
                    _ => (),
                }
            }
        }
    }
}
impl eframe::App for ImageSorter {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.show(ui, frame);
    }
}
