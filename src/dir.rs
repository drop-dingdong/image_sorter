use eframe::egui::Modifiers;
use eframe::egui::text::CCursorRange;
use eframe::{egui, egui::text::LayoutJob};
use egui::epaint::MarginF32;
use egui::text::CharIndex;
use egui::{
    Align, Color32, Context, FontId, FontSelection, Frame, Id, Key, Pos2, Rangef, Rect, Response,
    ScrollArea, Sense, Style, TextBuffer, TextEdit, Vec2, Vec2b, Widget, pos2, vec2,
    widget_text::RichText,
};

use crate::{display_len_with_special_fontid, rect_richtext};

use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
// 主要包含两部分，带有提示的单行地址输入框 以及 一般的 带有警告信息的输入框。
#[derive(Debug, Default)]
// 假定输入存在四种状态，waitingverify基础检验通过，需要额外的外部检验，
pub enum InputState {
    #[default]
    Active, // 可输入状态
    WaitingVerify, // 内部检查通过，需要外部检查。
    Verified,      // 外部检查通过，等待读取
    InActive,      // 被读取后的不可输入状态。
}
impl InputState {
    pub fn is_active(&self) -> bool {
        match self {
            InputState::Active => true,
            _ => false,
        }
    }
    pub fn is_inactive(&self) -> bool {
        // 不可输入态
        !self.is_active()
    }
    pub fn is_waiting(&self) -> bool {
        match self {
            InputState::WaitingVerify => true,
            _ => false,
        }
    }
    pub fn is_verified(&self) -> bool {
        match self {
            InputState::Verified => true,
            _ => false,
        }
    }
}
const TEXT_RATIO: f32 = 1.125; // fontid中的size和实际显示高度之间的比值
// 包裹hintinputstring，提供具体的显示位置
pub struct RectHintInput<'a> {
    hint_input: HintInputString<'a>,
    rect: Rect,
}
impl<'a> RectHintInput<'a> {
    pub fn new_with_default_font_size(
        text_buffer: &'a mut dyn TextBuffer,
        state: &'a mut InputState,
        id: impl Into<Id>,
        rect: Rect,
    ) -> Self {
        // rect，由于这里仅仅是单行文本输入，所以无论高度如何变换，可视区域的高度基本不会有什么变化。
        // rect中主要起到的作用的是宽度和左上或者右上的坐标，或者说最上面的边的情况。
        let hint_input = HintInputString::with_default_font_size_and_width(text_buffer, state, id);
        RectHintInput { hint_input, rect }
    }
    pub fn new_with_specifal_font_size(
        text_buffer: &'a mut dyn TextBuffer,
        state: &'a mut InputState,
        id: impl Into<Id>,
        font_size: f32,
        rect: Rect,
    ) -> Self {
        // 根据输入的font_size来设定margin和内部字体大小，font_size为fontid.size
        let margin = MarginF32::symmetric(0., (rect.height() - font_size * TEXT_RATIO) / 2.);
        let hint_input =
            HintInputString::new(text_buffer, state, margin, font_size, rect.width(), id);
        RectHintInput { hint_input, rect }
    }
    pub fn show_with_action(&mut self, ui: &mut egui::Ui, action: Option<&str>) {
        // 通过外部的动作action，对内部的hint_input进行操作，现有操作为确定当前输入。
        // 现在的ui显示情况，调整字体大小和rect的高度，发现textedit的高度和设定的rect的高度之间总是存在一定差距，差距或正或负
        ui.scope_builder(egui::UiBuilder::new().max_rect(self.rect), |ui| {
            self.hint_input.show_with_action(ui, action);
        });
    }
}
// 当前实现的功能是，输入一个字符串，如果为可及的目录的前缀，那么显示所有的备选目录，如果没有可备选的的目录，会直接给出No such diercotry的警告。当前现在仅仅对unix下的文件系统进行了测试
// 当存在备选的时候即会给出备选提示。如果光标处在最右侧，再次按下右键，那么将会填充。
// 这也就意味着如果出现备选提示，但是将光标移到其他地方，按住右键只会在文本输入框内进行移动，不会发生填充。
// 备选框，只能通过上下移动来更改填充的具体字段，不可通过点击或者其他操作触发填充操作。
// 默认备选项会直接以亮灰色显示在输入框内部。
// 更改具体选择项的时候，会自动滚动备选框，使得当选择的序号大于0时，自动将选项置顶。这与我没搞明白scroll_area的排版问题有关。
struct HintInputString<'a> {
    path_string: &'a mut dyn TextBuffer,
    state: &'a mut InputState,
    font_id: FontId,
    margin: MarginF32,
    width: f32,
    id: Id,
}
impl<'a> Widget for HintInputString<'a> {
    fn ui(mut self, ui: &mut egui::Ui) -> Response {
        self.show_with_action(ui, None)
    }
}
impl<'a> HintInputString<'a> {
    fn new(
        text_buffer: &'a mut dyn TextBuffer,
        state: &'a mut InputState,
        margin: MarginF32,
        font_size: f32,
        width: f32,
        id: impl Into<Id>,
    ) -> Self {
        let mut font_id = FontId::default();
        font_id.size = font_size;
        HintInputString {
            path_string: text_buffer,
            state,
            margin,
            width,
            font_id,
            id: id.into(),
        }
    }
    fn with_default_font_size_and_width(
        text_buffer: &'a mut dyn TextBuffer,
        state: &'a mut InputState,
        id: impl Into<Id>,
    ) -> Self {
        let font_id = FontId::default();
        let margin = MarginF32::symmetric(4., 2.); //textedit的基本margin
        let width = 400.;
        HintInputString {
            path_string: text_buffer,
            state,
            margin,
            width,
            font_id,
            id: id.into(),
        }
    }
    fn show_with_action(&mut self, ui: &mut egui::Ui, action: Option<&str>) -> Response {
        // 过程，读取state, 补全，绘制text_edit_ui，根据现在的结果更新状态，绘制 warngin和hint，并储存state
        let mut state = HintState::load(ui.ctx(), self.id).unwrap_or_default();
        match self.state {
            InputState::Active => {
                state.warning_index = 0;
            }
            _ => (),
        }
        let font_id = self.font_id.clone();
        self.hint_key_process(&mut state, ui); // hint_key_process将优先消耗 rightarrow 和 enter
        let response = self.text_edit_ui(&mut state, ui);
        self.update_state(&mut state, ui);
        if (state.content_state.warning && state.warning_index == 0) || state.warning_index != 0 {
            state.warning_display(font_id.clone(), ui);
        }
        state.hint_ui(font_id, ui);
        let content_state = &mut state.content_state;
        if self.state.is_active() {
            match action {
                None => (),
                Some("get") => {
                    if content_state.is_directory() {
                        self.path_string.replace_with(&content_state.path_cache);
                        *self.state = InputState::Verified;
                    } else {
                        state.warning_index = 1;
                        *self.state = InputState::InActive;
                    }
                }
                _ => (),
            };
        };
        state.store(ui.ctx(), self.id);
        response
    }
}
fn warning_display(warning: &&str, rem_rect: Rect, font_id: FontId, ui: &mut egui::Ui) {
    // 在rem_rect的右侧打印warning
    let y_min = rem_rect.top();
    let max = rem_rect.right_bottom();
    let err_len = display_len_with_special_fontid(*warning, font_id.clone(), ui.ctx());
    let rect = Rect::from_min_max(Pos2::new(max[0] - err_len, y_min), max);
    rect_richtext(
        ui,
        rect,
        RichText::new(*warning)
            .color(Color32::RED)
            .font(font_id.clone())
            .background_color(Color32::WHITE),
        font_id.into(),
        Color32::RED,
    );
}
#[derive(Debug, Clone, Default)]
// hintinputstring的input内容相关状态
struct HintInputState {
    path_cache: String,
    hint_indexes: Vec<usize>, // 当前输入下，初始段完全匹配情况下的备选index
    split_char: char,         // 路径分隔符
    prefix_path: PathBuf,     // 父目录
    prefix_path_legacy: bool, // 是否为合法父目录
    content_pos: usize,       //父目录后的起始位置
    split_len: usize,         // split_char的长度
    flag: bool,               // 表示父级路径对应字符串是否发生变化。
    subdir: Vec<OsString>,    // 父目录下的目录信息
    warning: bool,            // 是否对当前输入内容触发warning信息
}

impl HintInputState {
    fn new() -> Self {
        let mut initstate = HintInputState {
            path_cache: String::new(),
            hint_indexes: vec![],
            split_char: '/',
            split_len: '/'.len_utf8(),
            prefix_path: PathBuf::from("./"),
            content_pos: 0,
            flag: true,
            subdir: Vec::new(),
            warning: false,
            prefix_path_legacy: true,
        };
        initstate.load_dir_content();
        initstate
    }
    fn hint_str(&self) -> Vec<&str> {
        let mut vec = Vec::new();
        for i in &self.hint_indexes {
            vec.push(self.subdir[*i].to_str().unwrap());
        }
        vec
    }
    fn hint_str_index(&self, idx: usize) -> &str {
        self.subdir[self.hint_indexes[idx]].to_str().unwrap()
    }
    fn content(&self) -> &str {
        &self.path_cache[(self.content_pos)..]
    }
    fn path_cache_refresh<'a>(&mut self, input: &HintInputString<'a>) {
        let str = input.path_string.as_str();
        if self.path_cache != str {
            self.path_cache = str.to_owned();
        }
    }
    fn parse_path(&mut self) {
        if self.path_cache == "." || self.path_cache == ".." {
            self.prefix_path.clear();
            self.prefix_path.push(&self.path_cache);
            self.content_pos = 0;
            self.flag = true;
            return;
        }
        match self.path_cache.rfind(self.split_char) {
            None => {
                self.prefix_path.clear();
                self.prefix_path.push("./");
                self.content_pos = 0;
                self.flag = true;
            }
            Some(pos) => {
                if let Some(slice) = self.prefix_path.to_str()
                    && pos + self.split_len == self.content_pos
                    && slice == &self.path_cache[..self.content_pos]
                {
                    self.flag = false;
                } else {
                    self.content_pos = pos + self.split_len;
                    self.flag = true;
                    self.prefix_path.clear();
                    self.prefix_path.push(&self.path_cache[..self.content_pos]);
                }
            }
        };
    }
    // 根据父路径获取当前父路径下所有的目录名
    fn load_dir_content(&mut self) {
        match std::fs::read_dir(&self.prefix_path) {
            Err(err) => {
                println!("read path {:?} failed {:?}", &self.prefix_path, err);
                self.prefix_path_legacy = false;
                self.subdir.clear();
            }
            Ok(dir) => {
                self.prefix_path_legacy = true;
                self.subdir.clear();
                dir.map(|u| match u {
                    Err(_) => None,
                    Ok(dir_entiy) => {
                        if dir_entiy.file_type().map_or(false, |u| u.is_dir()) {
                            Some(dir_entiy.file_name())
                        } else {
                            None
                        }
                    }
                })
                .filter_map(|u| u)
                .fold((), |_, item| {
                    self.subdir.push(item);
                    ()
                });
            }
        };
    }
    // 筛选匹配的目录序号
    fn get_hint(&mut self) {
        self.hint_indexes.clear();
        self.subdir
            .iter()
            .enumerate()
            .filter(|(_, u)| {
                u.to_string_lossy()
                    .starts_with(&self.path_cache[(self.content_pos)..])
            })
            .fold((), |_, (id, _)| {
                self.hint_indexes.push(id);
                ()
            });
        if self.path_cache.len() == self.content_pos {
            // 即以split_char在path_cache的末尾，
            self.warning = !self.prefix_path_legacy;
        } else if self.path_cache == ".."
            || self.path_cache == "."
            || &self.path_cache[(self.content_pos)..] == "."
            || &self.path_cache[(self.content_pos)..] == ".."
            || !(self.hint_indexes.len() == 0)
        {
            self.warning = false;
        } else {
            self.warning = true;
        }
    }
    fn parse_and_hint<'a>(&mut self, input: &HintInputString<'a>) {
        self.path_cache_refresh(input);
        self.parse_path();
        if self.flag {
            self.load_dir_content();
        }
        self.get_hint();
    }
    fn is_directory(&self) -> bool {
        if self.warning {
            false
        } else if let Ok(_) = fs::read_dir(&self.path_cache) {
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
// 与edit相关的state，
struct TextEditState {
    content_disp_len: f32,     //父路径之后的内容的显示长度
    at_end: (bool, CharIndex), // 当前光标是否位于末尾，以及末尾的char index
    rem_rect: Rect,            //除去输入内容外的剩余空间
    changed: bool,             // 是否发生变化
    focused: bool,             // 是否focused
}
impl Default for TextEditState {
    fn default() -> Self {
        Self::new()
    }
}
impl TextEditState {
    fn new() -> Self {
        TextEditState {
            content_disp_len: 0.,
            at_end: (false, Default::default()),
            rem_rect: Rect::ZERO,
            changed: false,
            focused: false,
        }
    }
    fn parent_path_x_pos(&self) -> f32 {
        // 父路径结束时的x坐标
        self.rem_rect.left() - self.content_disp_len
    }
}
#[derive(Debug, Clone)]
// 与hint 选择相关的内容
struct HintSelect {
    // 目前里面除了hint_select，其他都没什么用
    hint_select: usize,
    offset: f32,
    disp_range: Rangef,
}
impl Default for HintSelect {
    fn default() -> Self {
        let hint_select = 0;
        let offset = 0.;
        let disp_range = Rangef { min: 0., max: 0. };
        HintSelect {
            hint_select,
            offset,
            disp_range,
        }
    }
}
impl HintSelect {
    fn reset_scroll_set(&mut self) {
        self.offset = 0.;
        self.hint_select = 0;
    }
}
#[derive(Debug, Clone)]
struct HintState {
    content_state: HintInputState,
    warning_text: Vec<String>, // warning_text为可选的warning文本，
    warning_index: usize,      // 实际显示的warning 的index
    select_state: HintSelect,
    text_edit_state: TextEditState,
}
impl Default for HintState {
    fn default() -> Self {
        Self::new()
    }
}

impl HintState {
    fn new() -> Self {
        HintState {
            content_state: HintInputState::new(),
            warning_text: vec![
                // 父路径下不存在可匹配的目录名
                "No such directory".to_string(),
                // 确认时，输入文本不是完整目录路径
                "Not right full dir path".to_string(),
            ],
            warning_index: 0,
            select_state: Default::default(),
            text_edit_state: TextEditState::new(),
        }
    }
}

#[cfg(test)]
mod test_hintdirct {
    use crate::dir::{HintInputState, HintInputString, InputState};

    #[test]
    fn test_hintdirct() {
        let mut string = String::new();
        let mut state = InputState::default();
        let input = HintInputString::with_default_font_size_and_width(
            &mut string,
            &mut state,
            "zero input",
        );
        // 空输入传返回当前目录下的情况
        let mut input_state = HintInputState::new();
        input_state.path_cache_refresh(&input);
        input_state.parse_path();
        assert_eq!(input.path_string.as_str(), "");
        assert_eq!(
            input_state
                .prefix_path
                .to_string_lossy()
                .clone()
                .into_owned(),
            "./"
        );
        assert_eq!(input_state.flag, true);
        input_state.load_dir_content();
        input_state.get_hint();
        assert_eq!(input_state.warning, false);
        let mut aim = vec![
            "test_dir",
            "target",
            "src",
            "asset",
            ".git",
            "_test_dir1",
            "_test_dir2",
        ];
        aim.sort();
        let hint_str = input_state.hint_str();
        for i in &aim {
            assert!(hint_str.contains(&i));
        }
        for i in &hint_str {
            assert!(aim.contains(&i));
        }

        // 当前目录，结果应与上结果相同
        string = "./".to_string();
        let input = HintInputString::with_default_font_size_and_width(
            &mut string,
            &mut state,
            "zero input",
        );
        // let mut input_state = HintInputState::new();

        input_state.path_cache_refresh(&input);
        input_state.parse_path();
        println!("input state {:?}", input_state);
        assert_eq!(input_state.content(), "");
        assert_eq!(
            input_state
                .prefix_path
                .to_string_lossy()
                .clone()
                .into_owned(),
            "./"
        );
        assert_eq!(input_state.flag, true);
        input_state.load_dir_content();
        input_state.get_hint();
        assert_eq!(input_state.warning, false);
        let hint_str = input_state.hint_str();
        for i in &aim {
            assert!(hint_str.contains(&i));
        }
        for i in &hint_str {
            assert!(aim.contains(&i));
        }

        // 当前目录下空头局部匹配
        string = "test".to_string();
        let input = HintInputString::with_default_font_size_and_width(
            &mut string,
            &mut state,
            "zero input",
        );
        // let mut input_state = HintInputState::new();
        input_state.path_cache_refresh(&input);
        input_state.parse_path();
        assert_eq!(input_state.content(), "test");
        assert_eq!(
            input_state
                .prefix_path
                .to_string_lossy()
                .clone()
                .into_owned(),
            "./"
        );
        assert_eq!(input_state.flag, true);
        input_state.load_dir_content();
        input_state.get_hint();
        assert_eq!(input_state.warning, false);
        assert_eq!(input_state.hint_str(), vec!["test_dir"]);

        // 当前目录头下部分匹配
        string = "./test".to_string();
        let input = HintInputString::with_default_font_size_and_width(
            &mut string,
            &mut state,
            "zero input",
        );
        // let mut input_state = HintInputState::new();
        input_state.path_cache_refresh(&input);
        input_state.parse_path();
        assert_eq!(input_state.content(), "test");
        assert_eq!(
            input_state
                .prefix_path
                .to_string_lossy()
                .clone()
                .into_owned(),
            "./"
        );
        assert_eq!(input_state.flag, true);
        input_state.load_dir_content();
        input_state.get_hint();
        assert_eq!(input_state.warning, false);
        assert_eq!(input_state.hint_str(), vec!["test_dir"]);

        //空目录头下匹配下一级目录
        string = "test_dir/".to_string();
        let input = HintInputString::with_default_font_size_and_width(
            &mut string,
            &mut state,
            "zero input",
        );
        // let mut input_state = HintInputState::new();
        input_state.path_cache_refresh(&input);
        input_state.parse_path();
        assert_eq!(input_state.content(), "");
        assert_eq!(
            input_state
                .prefix_path
                .to_string_lossy()
                .clone()
                .into_owned(),
            "test_dir/"
        );
        assert_eq!(input_state.flag, true);
        input_state.load_dir_content();
        input_state.get_hint();
        assert_eq!(input_state.warning, false);
        let vec1 = vec![
            "dir1", "dir2", "dir3", "dir4", "dir5", "dir6", "dir7", "dir8", "dir9", "dir10",
            "dir11", "dir12", "dir13",
        ];
        let hint_str = input_state.hint_str();
        for i in &vec1 {
            assert!(hint_str.contains(&i));
        }
        for i in &hint_str {
            assert!(vec1.contains(&i));
        }

        // 同头下复用之前的结果
        string = "test_dir/dir".to_string();
        let input = HintInputString::with_default_font_size_and_width(
            &mut string,
            &mut state,
            "zero input",
        );
        // let mut input_state = HintInputState::new();
        input_state.path_cache_refresh(&input);
        input_state.parse_path();
        assert_eq!(input_state.content(), "dir");
        assert_eq!(
            input_state
                .prefix_path
                .to_string_lossy()
                .clone()
                .into_owned(),
            "test_dir/"
        );
        assert_eq!(input_state.flag, false);
        input_state.get_hint();
        assert_eq!(input_state.warning, false);
        assert_eq!(input_state.hint_str(), vec1);

        //当前目录头下匹配下一级目录
        string = "./test_dir/".to_string();
        let input = HintInputString::with_default_font_size_and_width(
            &mut string,
            &mut state,
            "zero input",
        );
        // let mut input_state = HintInputState::new();
        input_state.path_cache_refresh(&input);
        input_state.parse_path();
        assert_eq!(input_state.content(), "");
        assert_eq!(
            input_state
                .prefix_path
                .to_string_lossy()
                .clone()
                .into_owned(),
            "./test_dir/"
        );
        assert_eq!(input_state.flag, true);
        input_state.load_dir_content();
        input_state.get_hint();
        assert_eq!(input_state.warning, false);
        assert_eq!(input_state.hint_str(), vec1);

        //当尾部分为 .或者..的时候不提供匹配选项
        string = "./test_dir/..".to_string();
        let input = HintInputString::with_default_font_size_and_width(
            &mut string,
            &mut state,
            "zero input",
        );
        // let mut input_state = HintInputState::new();
        input_state.path_cache_refresh(&input);
        input_state.parse_path();
        assert_eq!(input_state.content(), "..");
        assert_eq!(
            input_state
                .prefix_path
                .to_string_lossy()
                .clone()
                .into_owned(),
            "./test_dir/"
        );
        assert_eq!(input_state.flag, false);
        input_state.get_hint();
        assert_eq!(input_state.warning, false);
        assert_eq!(input_state.hint_str(), Vec::<String>::new());

        //多级匹配测试
        string = "./test_dir/empty".to_string();
        let input = HintInputString::with_default_font_size_and_width(
            &mut string,
            &mut state,
            "zero input",
        );
        // let mut input_state = HintInputState::new();
        input_state.path_cache_refresh(&input);
        input_state.parse_path();
        assert_eq!(input_state.content(), "empty");
        assert_eq!(
            input_state
                .prefix_path
                .to_string_lossy()
                .clone()
                .into_owned(),
            "./test_dir/"
        );
        assert_eq!(input_state.flag, false);
        input_state.get_hint();
        assert_eq!(input_state.warning, true);
        assert_eq!(input_state.hint_str(), Vec::<String>::new());

        // 多级匹配测试
        string = "./test_dir/dir1".to_string();
        let input = HintInputString::with_default_font_size_and_width(
            &mut string,
            &mut state,
            "zero input",
        );
        // let mut input_state = HintInputState::new();
        input_state.path_cache_refresh(&input);
        input_state.parse_path();
        assert_eq!(input_state.content(), "dir1");
        assert_eq!(
            input_state
                .prefix_path
                .to_string_lossy()
                .clone()
                .into_owned(),
            "./test_dir/"
        );
        assert_eq!(input_state.flag, false);
        input_state.get_hint();
        assert_eq!(input_state.warning, false);
        assert_eq!(
            input_state.hint_str(),
            vec!["dir1", "dir10", "dir11", "dir12", "dir13"]
        );

        // 多级匹配测试
        string = "./test_dir/dir2".to_string();
        let input = HintInputString::with_default_font_size_and_width(
            &mut string,
            &mut state,
            "zero input",
        );
        // let mut input_state = HintInputState::new();
        input_state.path_cache_refresh(&input);
        input_state.parse_path();
        assert_eq!(input_state.content(), "dir2");
        assert_eq!(
            input_state
                .prefix_path
                .to_string_lossy()
                .clone()
                .into_owned(),
            "./test_dir/"
        );
        assert_eq!(input_state.flag, false);
        input_state.get_hint();
        assert_eq!(input_state.warning, false);
        assert_eq!(input_state.hint_str(), vec!["dir2"]);

        //多级匹配测试
        string = "./test_dir/dir1/".to_string();
        let input = HintInputString::with_default_font_size_and_width(
            &mut string,
            &mut state,
            "zero input",
        );
        // let mut input_state = HintInputState::new();
        input_state.path_cache_refresh(&input);
        input_state.parse_path();
        assert_eq!(input_state.content(), "");
        assert_eq!(
            input_state
                .prefix_path
                .to_string_lossy()
                .clone()
                .into_owned(),
            "./test_dir/dir1/"
        );
        assert_eq!(input_state.flag, true);
        input_state.load_dir_content();
        input_state.get_hint();
        assert_eq!(input_state.warning, false);
        assert_eq!(input_state.hint_str(), vec!["dir2"]);
    }
}

impl<'a> HintInputString<'a> {
    fn update_state(self: &Self, state: &mut HintState, ui: &mut egui::Ui) {
        let text_edit_state = &mut state.text_edit_state;
        let content_state = &mut state.content_state;
        let select_state = &mut state.select_state;
        if text_edit_state.changed {
            // 只有当输入文本发生变化的时候，发生的更新操作
            select_state.reset_scroll_set();
            content_state.parse_and_hint(self);
            text_edit_state.content_disp_len = display_len_with_special_fontid(
                content_state.content(),
                self.font_id.clone(),
                ui.ctx(),
            );
            text_edit_state.changed = false;
        }
    }
    fn text_edit_ui(self: &mut Self, state: &mut HintState, ui: &mut egui::Ui) -> Response {
        // 显示文本输入框，并且根据输入的变化更新text_edit_state信息
        let text_edit_state = &mut state.text_edit_state;
        let x_margin = self.margin.left;
        let x_bias = 2.;
        let font_id = self.font_id.clone();

        let input_id: Id = "input".into();

        let textedit = TextEdit::singleline(self.path_string)
            .font(font_id)
            .desired_rows(1)
            .clip_text(true)
            .interactive(self.state.is_active())
            .background_color(Color32::WHITE)
            .cursor_at_end(true)
            .margin(self.margin)
            .id(input_id);
        let output = textedit.desired_width(self.width).show(ui);

        let galley = &output.galley;
        // 如果触发自动补全，自动移动光标
        if text_edit_state.changed
            && let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), input_id)
        {
            state
                .cursor
                .set_char_range(Some(CCursorRange::one(galley.end())));
            egui::TextEdit::store_state(ui.ctx(), input_id, state);
            // 每次update_state后changed自动为false, 这里的changed只探测自动补全，因此在该函数调用之前，调用hint_key_process
        }
        let lt = output.response.rect.min.to_vec2();
        let response = output.response;
        let rect = response.rect;
        let end_x = (galley.pos_from_cursor(galley.end()).min.x + x_margin + lt.x)
            .max(rect.left() + x_bias);
        let rem_rect = Rect::from_min_max(
            pos2(end_x, rect.top()),
            rect.right_bottom() - vec2(x_bias, 0.),
        );
        text_edit_state.rem_rect = rem_rect;
        text_edit_state.changed |= response.changed();
        text_edit_state.focused = response.has_focus();
        text_edit_state.at_end = {
            if let Some(state) = output.cursor_range {
                let c_idx = galley.end().index;
                (state.primary.index == c_idx, c_idx)
            } else {
                (false, Default::default())
            }
        };
        response.response
    }
    fn hint_key_process(self: &mut Self, state: &mut HintState, ui: &mut egui::Ui) {
        // arrowdown arrowup更改所选的hint，光标在最右侧时，arronright和enter可以利用选择的hint自动填充
        let path_string = &mut self.path_string;
        let input_state = &state.content_state;
        let hint_indexes = &input_state.hint_indexes;
        let subdir = &input_state.subdir;
        let hint_select = &mut state.select_state.hint_select;
        let hint_num = input_state.hint_indexes.len();
        let text_edit_state = &mut state.text_edit_state;
        if hint_num > 0 {
            let content_byte_len = input_state.content().len();
            // 现在的按键状态是，当光标位于输入末端时，根据hint_select来更换选择的hint_string。如果按下->，那么自动将对应字段填充。
            if hint_num > 1 && *hint_select < hint_num - 1 {
                ui.input_mut(|i| {
                    if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
                        *hint_select += 1;
                    }
                })
            }
            if hint_num > 1 && *hint_select > 0 {
                ui.input_mut(|i| {
                    if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
                        *hint_select -= 1;
                    }
                })
            }
            let str0 = &subdir[hint_indexes[*hint_select]].to_str().unwrap()[content_byte_len..];

            let (at_end, c_idx) = text_edit_state.at_end;
            if at_end {
                ui.input_mut(|i| {
                    if i.key_pressed(Key::ArrowRight) {
                        path_string.insert_text(str0, c_idx); // 这里是否存在数据竞争的问题？
                        i.consume_key(Modifiers::NONE, Key::ArrowRight);
                        text_edit_state.changed = true;
                    } else if i.key_pressed(Key::Enter) {
                        path_string.insert_text(str0, c_idx);
                        i.consume_key(Modifiers::NONE, Key::Enter);
                        text_edit_state.changed = true;
                    }
                });
            }
        }
    }
}
impl HintState {
    fn load(ctx: &Context, id: Id) -> Option<Self> {
        ctx.data_mut(|d| d.get_temp(id))
    }
    fn store(self, ctx: &Context, id: Id) {
        ctx.data_mut(|d| d.insert_temp(id, self));
    }
    fn warning_display(self: &mut Self, font_id: FontId, ui: &mut egui::Ui) {
        let rem_rect = self.text_edit_state.rem_rect;
        warning_display(
            &self.warning_text[self.warning_index].as_str(),
            rem_rect,
            font_id,
            ui,
        );
    }
    fn hint_ui(self: &Self, font_id: FontId, ui: &mut egui::Ui) {
        let content_state = &self.content_state;
        let content_len = content_state.content().len();
        let text_edit_state = &self.text_edit_state;

        // 绘制hint_string
        let hint_num = content_state.hint_indexes.len();
        let hint_select = self.select_state.hint_select;
        let rem_rect = text_edit_state.rem_rect;

        if hint_num > 0 {
            let tot_x = rem_rect.right();
            let y_max = rem_rect.bottom();
            let text_height = font_id.size * TEXT_RATIO;
            // 绘制首个hint_string
            let hint0 = content_state.hint_str_index(0);
            let hint_len = display_len_with_special_fontid(hint0, font_id.clone(), ui.ctx());
            let font_selection: FontSelection = font_id.clone().into();
            rect_richtext(
                ui,
                Rect::from_min_max(
                    rem_rect.left_top(),
                    Pos2::new(
                        (text_edit_state.parent_path_x_pos() + hint_len).min(tot_x),
                        y_max,
                    ),
                ),
                RichText::new(hint0[content_len..].to_string())
                    .color(Color32::LIGHT_GRAY)
                    .background_color(Color32::WHITE),
                font_selection.clone(),
                Color32::LIGHT_GRAY,
            );

            let min_pos = Pos2::new(text_edit_state.parent_path_x_pos().max(0.), y_max);
            if text_edit_state.focused && hint_num > 1 {
                let max_len = (0..hint_num)
                    .into_iter()
                    .map(|u| {
                        display_len_with_special_fontid(
                            content_state.hint_str_index(u),
                            font_id.clone(),
                            ui.ctx(),
                        )
                    })
                    .fold(0., |acc, item| {
                        if (!f32::is_nan(item)) && item > acc {
                            item
                        } else {
                            acc
                        }
                    });
                let line_space = ui.spacing().item_spacing.y;
                let line_height = text_height + line_space * 1.4; // 涉及一些行间距修正
                let item_size = Vec2::new(max_len, text_height); // 如果使用font_id.size作为显示字符串的item大小的话，实际显示的字符串是经过裁剪的，比如下划线将完全无法看见。
                let empty_size = Vec2::new(1., text_height);
                let max_hint_disp_num = (hint_num - 1).min(10);
                let output = egui::Area::new("ScrollArea".into())
                    .fixed_pos(min_pos)
                    .order(egui::Order::Foreground)
                    .default_width(max_len + 0.)
                    .default_height(line_height * max_hint_disp_num as f32) // 按理来说这么设置没什么问题，但是scroll_area行间距，如果使用ui.horizontal的话，行间距相较于只显示文本的时候差了一些，推测可能是因为ui.horizontal和ui.add叠加了两份行间距。但两份line_space显然也不对。也因此自动scroll还是按照目前的样式，自动将当前选择的处于顶部。
                    // 不知为何，这里的default_height设置是失败的，原本可以同时显示多个hint dir，但调整self.font_id.size后，同时显示的hint_dir的个数依赖于size
                    .show(ui.ctx(), |ui| {
                        Frame::new().fill(Color32::WHITE).show(ui, |ui| {
                            ScrollArea::vertical()
                                .auto_shrink(Vec2b::new(false, false))
                                .vertical_scroll_offset(
                                    (line_height) * (hint_select.max(1) - 1) as f32,
                                )
                                .show_rows(ui, item_size.y, hint_num - 1, |ui, row_range| {
                                    ui.spacing_mut().item_spacing.x = 0.;
                                    // ui.spacing_mut().extra_text_line_spacing = x_margin;
                                    // 更改了一下project，为什么这行代码有问题？查看了一下返回类型均一致为 egui::style::Spacing
                                    ui.spacing_mut().item_spacing.y = line_space;
                                    for row_index in row_range {
                                        let background_color = if hint_select == row_index + 1 {
                                            Color32::LIGHT_BLUE
                                        } else {
                                            Color32::WHITE
                                        };
                                        ui.horizontal(|ui| {
                                            ui.add(exact_empty_space(empty_size, background_color));
                                            ui.add(FixedSizedText::new(
                                                content_state.hint_str_index(row_index + 1),
                                                Color32::BLACK,
                                                background_color,
                                                font_selection.clone(),
                                                item_size,
                                                Align::Min,
                                            ));
                                        });
                                    }
                                });
                        });
                    });
            }
        }
    }
}
// 一个给定大小的文本框，内部显示文字
pub struct FixedSizedText<T: Into<String>> {
    string: T,
    text_color: Color32,
    background_color: Color32,
    font: FontSelection,
    size: Vec2,
    align: Align,
}
impl<T: Into<String>> FixedSizedText<T> {
    pub fn new(
        string: T,
        text_color: Color32,
        background_color: Color32,
        font: FontSelection,
        size: Vec2,
        align: Align,
    ) -> Self {
        FixedSizedText {
            string,
            text_color,
            background_color,
            font,
            size,
            align,
        }
    }
}
// 单独占据给定size方形面积的空白空间，该区域的颜色由background_color填充
pub fn exact_empty_space(size: Vec2, background_color: Color32) -> FixedSizedText<&'static str> {
    FixedSizedText {
        string: "",
        text_color: background_color,
        background_color,
        font: FontSelection::Default,
        size,
        align: Align::Center,
    }
}
impl<T: Into<String>> Widget for FixedSizedText<T> {
    fn ui(self, ui: &mut egui::Ui) -> Response {
        let (rect, response) = ui.allocate_exact_size(self.size, Sense::empty());
        let painter = ui.painter_at(rect);
        painter.galley(
            rect.left_top(),
            painter.layout_job({
                let rich_text = RichText::new(self.string)
                    .background_color(self.background_color)
                    .color(self.text_color);
                let mut layout_job = LayoutJob::default();
                rich_text.append_to(&mut layout_job, &Style::default(), self.font, self.align);
                layout_job
            }),
            self.text_color,
        );
        response
    }
}
// 给定位置设置一个输入框，可选hint_str以及warning str，如果需要warning str起作用，那么需要输入一个pattern,当pattern(input) == false的时候会在输入框末端显示warning提示词。
// 现在通过设置font_size以及调整margin，基本可以实现较好的控制，但是仍有以下问题。
// 检验了texteditoutput.rect确实和零输入时的rem_rect横向相同。但显示效果上，可以明显的看到hint_str和warning的超过了输入框的范围。
pub struct WarningInput<'a> {
    input: &'a mut String,
    state: &'a mut InputState,
    id: Id,
    pos: Pos2,
    margin: MarginF32,
    width: f32,
    font_size: f32,
    warning: &'a str,
    extern_warning: Option<&'a str>, // 可供选择的外部warning，
    hint_str: Option<&'a str>,
    pattern: Option<Box<dyn for<'c> Fn(&'c String) -> bool + 'a>>, // 用于决定是否显示内在的warning
}
impl<'a> WarningInput<'a> {
    pub fn new(
        hint_str: Option<&'a str>,
        input: &'a mut String,
        state: &'a mut InputState,
        id: impl Into<Id>,
        font_size: f32,
        rect: Rect,
        warn_text: &'a str,
    ) -> Self {
        let y_margin = (rect.height() - font_size * TEXT_RATIO) / 2.;

        WarningInput {
            hint_str,
            input,
            state,
            id: id.into(),
            pos: rect.left_top(),
            width: rect.width(),
            extern_warning: None,
            font_size,
            margin: MarginF32::symmetric(0., y_margin),
            warning: warn_text,
            pattern: None,
        }
    }
    pub fn insert_pattern(&mut self, pattern: Box<dyn for<'c> Fn(&'c String) -> bool + 'a>) {
        self.pattern = Some(pattern);
    }
    pub fn insert_extern_warning(&mut self, warning: &'a str) {
        self.extern_warning = Some(warning);
    }
    pub fn clear_extern_warning(&mut self) {
        self.extern_warning = None;
    }
    pub fn show_with_action(&mut self, ui: &mut egui::Ui, action: Option<&str>) {
        self.text_edit_ui(ui);
        self.action_result(action);
    }
    fn action_result(&mut self, action: Option<&str>) {
        match action {
            None => (),
            Some(_) => {
                let right1 = self.state.is_active();
                let right2 = if let Some(p) = &self.pattern {
                    p(self.input)
                } else {
                    true
                };
                if right1 && right2 {
                    *self.state = InputState::WaitingVerify;
                }
            }
        }
    }
    fn text_edit_ui(&mut self, ui: &mut egui::Ui) {
        let x_margin = self.margin.left;
        let y_margin = self.margin.top;
        let x_bias = 2.;
        let mut font_id = FontId::default();
        font_id.size = self.font_size;
        let textedit = TextEdit::singleline(self.input)
            .font(font_id.clone())
            .desired_rows(1)
            .clip_text(true)
            .interactive(self.state.is_active())
            .text_color(Color32::BLACK)
            .background_color(Color32::WHITE)
            .margin(self.margin)
            .cursor_at_end(true)
            .desired_width(self.width);
        let rect = Rect::from_min_size(
            self.pos,
            vec2(self.width, (font_id.size * TEXT_RATIO + y_margin * 2.)),
        );
        let output = ui
            .scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                textedit.show(ui)
            })
            .inner; // 按照现在的设置结果来看，在视觉上textedit输入框会超过rect的大小。
        // let output = textedit.show(ui);
        let galley = &output.galley;
        let response = output.response;

        let lt = response.rect.min.to_vec2();
        let end_x = (galley.pos_from_cursor(galley.end()).min.x + x_margin + lt.x)
            .max(rect.left() + x_bias);

        let rem_rect = Rect::from_min_max(
            pos2(end_x, rect.top()),
            rect.right_bottom() - vec2(x_bias, 0.),
        );

        let font_selection: FontSelection = font_id.clone().into();

        match self.hint_str {
            None => (),
            Some(hint) => {
                let tot_x = rem_rect.right();
                let y_max = rem_rect.bottom();
                let hint_len = display_len_with_special_fontid(hint, font_id.clone(), ui.ctx());
                rect_richtext(
                    ui,
                    Rect::from_min_max(
                        rem_rect.left_top(),
                        pos2((rem_rect.left() + hint_len).min(tot_x), y_max),
                    ),
                    RichText::new(hint).color(Color32::LIGHT_GRAY),
                    font_selection.clone(),
                    Color32::LIGHT_GRAY,
                );
            }
        }

        match (&self.pattern, &self.extern_warning) {
            (None, None) => (),
            (_, Some(warning)) => {
                warning_display(warning, rem_rect, font_id.clone(), ui);
            }
            (Some(p), None) => {
                if !p(self.input) {
                    warning_display(&self.warning, rem_rect, font_id, ui);
                }
            }
        }
    }
}
