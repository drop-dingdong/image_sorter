use crate::app::AppState;
use crate::dir::{FixedSizedText, InputState, RectHintInput, WarningInput};
use crate::display_size_with_special_fontid;
use crate::idx_name::{IndexName, RenamePatternState};
use egui::{
    Align, Button, Color32, CornerRadius, FontId, Id, Rect, Response, RichText, Tooltip, Vec2,
    pos2, vec2,
};
use once_cell::sync::Lazy;
use regex::Regex;
// 该文件主要包含顶部两个输入框相关内容ui的计算部分，以及对应的结构体，本质上这个mod是dir.rs内容物的包装
pub struct InputItem {
    input: String,
    input_cache: String,
    pub state: InputState,
    id: String,
    rect: Rect,
}
impl InputItem {
    pub fn new_with_id_rect(id: impl Into<String>, rect: Rect) -> Self {
        let input = String::new();
        let input_cache = String::new();
        let state = Default::default();
        InputItem {
            input,
            input_cache,
            state,
            id: id.into(),
            rect,
        }
    }
    pub fn update_cache(&mut self) -> &str {
        self.input_cache = self.input.clone();
        self.input_cache.as_str()
    }
    pub fn get_cache(&self) -> &str {
        &self.input_cache
    }
}

pub fn dir_input_ui(input_item: &mut InputItem, rect_lict: &[Rect; 4], ui: &mut egui::Ui) {
    let mut font_id = FontId::default();
    font_id.size = 25.;
    let dir_title_str = "images dir: ";

    let dir_pos_list = *rect_lict;
    let dir_input = input_item;
    let [
        dir_title_back_rect,
        dir_title_rect,
        dir_input_rect,
        dir_button_rect,
    ] = dir_pos_list;

    ui.painter_at(dir_title_back_rect).rect_filled(
        dir_title_back_rect,
        CornerRadius {
            nw: 20,
            sw: 20,
            ..CornerRadius::same(0)
        },
        Color32::LIGHT_GRAY.gamma_multiply(0.5),
    );

    ui.put(
        dir_title_rect,
        FixedSizedText::new(
            dir_title_str,
            Color32::GOLD,
            Color32::TRANSPARENT,
            font_id.clone().into(),
            dir_title_rect.size(),
            Align::Center,
        ),
    );

    let id = Id::new(dir_input.id.clone());

    let dir_button_response = ui.put(
        dir_button_rect,
        Button::new(
            RichText::new(if dir_input.state.is_active() {
                "✅"
            } else {
                "↩"
            })
            .color(Color32::BLACK),
        ) // 2705, 21a9
        .fill(Color32::GOLD)
        .corner_radius(CornerRadius {
            ne: 20,
            se: 20,
            ..CornerRadius::same(0)
        }),
    );

    let action = if dir_button_response.clicked() && dir_input.state.is_active() {
        Some("get")
    } else {
        None
    };

    let re_active = dir_input.state.is_inactive() && dir_button_response.clicked();
    if re_active {
        // input.state 在textedit内部判断是否变为verified的，在外部只要是处于inactive的（包含除了active之外所有的态），clicked将会置为active
        dir_input.state = InputState::Active;
    }

    Tooltip::for_enabled(&dir_button_response).show(|ui| {
        ui.label(if dir_input.state.is_active() {
            "verify path and load images"
        } else {
            "reset path input"
        })
    });

    let mut rect_dir_input = RectHintInput::new_with_specifal_font_size(
        &mut dir_input.input,
        &mut dir_input.state,
        id,
        font_id.size,
        dir_input_rect,
    );
    rect_dir_input.show_with_action(ui, action);
}
static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^a(\d+)b(\d+)$").unwrap());
pub fn rename_input_ui(
    input_item: &mut InputItem,
    rect_lict: &[Rect; 5],
    rename_pattern: &mut RenamePatternState,
    app_state: &AppState, // 重命名按钮只在appstate::sort的时候才会激活，
    ui: &mut egui::Ui,
) -> Response {
    let mut font_id = FontId::default();
    font_id.size = 25.;
    let rename_title_str = "rename pattern: ";

    let rename_pos_list = *rect_lict;
    let rename_input = input_item;

    // 基础的校检
    if rename_input.state.is_waiting() && !rename_pattern.is_iter() {
        match IndexName::new(&rename_input.input) {
            None => {
                *rename_pattern =
                    RenamePatternState::Err("producing rename iter failed".to_string());
                rename_input.state = InputState::InActive; // 初次校检未过，需要重新输入
            }
            Some(iter) => {
                rename_input.input_cache = rename_input.input.clone();
                *rename_pattern = RenamePatternState::Iter(iter); // 生成迭代器，供下次校检
            }
        }
    }

    let [
        rename_title_back_rect,
        rename_title_rect,
        rename_input_rect,
        rename_button_1_rect,
        rename_button_2_rect,
    ] = rename_pos_list;

    ui.painter_at(rename_title_back_rect).rect_filled(
        rename_title_back_rect,
        CornerRadius {
            nw: 20,
            sw: 20,
            ..CornerRadius::same(0)
        },
        Color32::LIGHT_GRAY.gamma_multiply(0.3),
    );
    ui.put(
        rename_title_rect,
        FixedSizedText::new(
            rename_title_str,
            Color32::GOLD,
            Color32::TRANSPARENT,
            font_id.clone().into(),
            rename_title_rect.size(),
            Align::Center,
        ),
    );
    let verify_response = ui.put(
        rename_button_1_rect,
        Button::new(
            RichText::new(if rename_input.state.is_active() {
                "✅"
            } else {
                "↩"
            })
            .color(Color32::BLACK),
        ) // 2705, 21a9
        .fill(Color32::GOLD)
        .corner_radius(CornerRadius::same(0)),
    );

    Tooltip::for_enabled(&verify_response).show(|ui| {
        ui.label(if rename_input.state.is_active() {
            "verify rename pattern"
        } else {
            "reset rename pattern"
        })
    });
    let action = if verify_response.clicked() && rename_input.state.is_active() {
        Some("get")
    } else {
        None
    };
    if rename_input.state.is_inactive() && verify_response.clicked() {
        // 重新输入pattern的话，会将所有的renamepatternstate信息删除。
        *rename_pattern = RenamePatternState::Empty;
        rename_input.state = InputState::Active;
    }

    let enable =
        rename_input.state.is_inactive() && rename_pattern.is_iter() && app_state.is_sort();

    let rename_response = ui
        .scope_builder(
            egui::UiBuilder::new().max_rect(rename_button_2_rect),
            |ui| {
                ui.add_enabled(
                    enable,
                    Button::new(RichText::new("💾").color(Color32::BLACK)) // 1f4be
                        .fill(Color32::GOLD)
                        .min_size(rename_button_2_rect.size()) // 如果不额外限制这个地方，button的size将变小
                        .corner_radius(CornerRadius {
                            ne: 20,
                            se: 20,
                            ..CornerRadius::same(0)
                        }),
                )
            },
        )
        .inner;
    Tooltip::for_enabled(&rename_response)
        .show(|ui| ui.label("use the input pattern to rename images"));
    Tooltip::for_disabled(&rename_response)
        .show(|ui| ui.label("before rename, you must input your rename pattern"));

    let id = Id::new(rename_input.id.clone());
    let mut rename_warning_input = WarningInput::new(
        None,
        &mut rename_input.input,
        &mut rename_input.state,
        id,
        font_id.size,
        rename_input_rect,
        "axby, where x y is digit",
    );
    let pattern = Box::new(|s: &String| RE.is_match(s));
    rename_warning_input.insert_pattern(pattern);
    // renamepatternstate::err携带的警告信息会一并显示。
    if let RenamePatternState::Err(warning) = &rename_pattern {
        rename_warning_input.insert_extern_warning(warning);
    }
    rename_warning_input.show_with_action(ui, action);

    rename_response
}
pub fn calculate_rect(top_panel: &[InputItem; 2], ctx: &egui::Context) -> ([Rect; 4], [Rect; 5]) {
    // 根据inputitem的情况计算需要用到的rect信息
    // 主要为以下四种
    // 输入框的标题所用rect
    // 输入框背景所有rect
    // 输入框所用rect
    // 搭配的按钮所用的rect
    let mut font_id = FontId::default();
    font_id.size = 25.;
    let default_height = 30.;
    let default_button_size = Vec2::splat(30.);
    let dir_title_str = "images dir: ";
    let rename_title_str = "rename pattern: ";
    let y_center = 30.;
    let dir_input_size = top_panel[0].rect.size();
    let rename_input_size = top_panel[1].rect.size();

    let left_bias = 20.;
    let dir_title_size = display_size_with_special_fontid(dir_title_str, font_id.clone(), ctx);
    let dir_title_size = vec2(dir_title_size.x, default_height);
    let dir_back_size = dir_title_size + vec2(left_bias, 0.);
    let dir_title_back_rect = Rect::from_center_size(
        pos2(left_bias / 2. + dir_title_size.x / 2., y_center),
        dir_back_size,
    );
    let dir_title_rect = Rect::from_center_size(
        pos2(left_bias + dir_title_size.x / 2., y_center),
        dir_title_size,
    );
    let dir_input_center_x = dir_title_rect.right() + dir_input_size.x / 2.;
    let dir_input_rect = Rect::from_center_size(pos2(dir_input_center_x, y_center), dir_input_size);
    let dir_button_rect = Rect::from_min_size(dir_input_rect.right_top(), default_button_size);

    let empty_width = 50.;
    let end_x = dir_button_rect.right() + empty_width;
    let rename_title_size =
        display_size_with_special_fontid(rename_title_str, font_id.clone(), ctx);
    let rename_title_size = vec2(rename_title_size.x, default_height);
    let rename_title_back_rect = Rect::from_center_size(
        pos2(end_x - 10. + rename_title_size.x / 2., y_center),
        rename_title_size + vec2(20., 0.),
    );
    let rename_title_rect = Rect::from_center_size(
        pos2(end_x + rename_title_size.x / 2., y_center),
        rename_title_size,
    );
    let rename_input_center_x = rename_title_rect.right() + rename_input_size.x / 2.;
    let rename_input_rect =
        Rect::from_center_size(pos2(rename_input_center_x, y_center), rename_input_size);
    let rename_button_1_rect =
        Rect::from_min_size(rename_input_rect.right_top(), default_button_size);
    let rename_button_2_rect =
        (Rect::from_min_size(rename_button_1_rect.right_top(), default_button_size));
    let res: ([Rect; 4], [Rect; 5]) = (
        [
            dir_title_back_rect,
            dir_title_rect,
            dir_input_rect,
            dir_button_rect,
        ],
        [
            rename_title_back_rect,
            rename_title_rect,
            rename_input_rect,
            rename_button_1_rect,
            rename_button_2_rect,
        ],
    );
    res
}
