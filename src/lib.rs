pub mod app;
pub mod self_trait;

pub mod main_struct;
pub use main_struct::MainStruct;

pub mod mix_item;

pub mod related_struct;
use related_struct::Position;

pub mod scroll_drag_panel;
pub use scroll_drag_panel::FixedDispSizeScrollArea;

pub mod show_panel;
pub use show_panel::DataShow;

pub mod test_text;
pub use test_text::TextItemPos;

pub mod image_item;
pub use image_item::{ImageItem, rename_inner_layer};

pub mod asset;
pub use asset::{load_dir_icon, load_save_icon};

pub mod rename;
pub use rename::idx_name;

pub mod dir;
pub mod input_panel;

use egui::{
    Align, Align2, Color32, FontId, FontSelection, Id, Painter, Rect, RichText, Style, Vec2,
    text::LayoutJob,
};
use rand::distributions::Alphanumeric;
use rand::{Rng, thread_rng};
use self_trait::{DragItem, IntoContent, PosItem};

use std::path::PathBuf;
use std::{fs, mem};

// 一般函数

// 随机生成给定长度的ascii字符串，用于对新产生的group随机命名
pub fn random_alphanumeric_string(length: usize) -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

// 和ui有关，但不需要ui输入的函数

// 如果一个rect需要移动一个shift_vec，那么得到的rect为
fn rect_shift(rect: Rect, shift_vec: Vec2) -> Rect {
    Rect::from_center_size(rect.center() + shift_vec, rect.size())
}

// 需要ui运行的时候参与的函数

// 给定str，如果用default字体显示时，需要的宽高
fn text_display_size(string: &str, ctx: &egui::Context) -> (f32, f32) {
    let disp_size = display_size_with_special_fontid(string, FontId::default(), ctx);
    (disp_size.x, disp_size.y)
}
// 同上，但是需要指定使用的字体，且返回值为size
fn display_size_with_special_fontid(string: &str, font_id: FontId, ctx: &egui::Context) -> Vec2 {
    ctx.fonts_mut(|fonts| fonts.layout_no_wrap(string.to_string(), font_id, Color32::BLACK))
        .rect
        .size()
}
// 同上，但是仅返回width
fn display_len_with_special_fontid(string: &str, font_id: FontId, ctx: &egui::Context) -> f32 {
    display_size_with_special_fontid(string, font_id, ctx).x
}
// 同text_display_size，但是仅返回width
fn display_len(string: &str, ctx: &egui::Context) -> f32 {
    text_display_size(string, ctx).0
}

// 在给定rect居中显示rich_text
fn rect_richtext(
    ui: &mut egui::Ui,
    rect: Rect,
    rich_text: RichText,
    font: FontSelection,
    color: Color32,
) {
    let painter = ui.painter_at(rect);
    painter_richtext(painter, rect, rich_text, font, color);
}
// 在给定painter中，居中显示rich_text
fn painter_richtext(
    painter: Painter,
    rect: Rect,
    rich_text: RichText,
    font: FontSelection,
    color: Color32,
) {
    let mut layout_job = LayoutJob::default();
    rich_text.append_to(&mut layout_job, &Style::default(), font, Align::Center);
    let galley = painter.layout_job(layout_job);
    let rect = Align2::CENTER_CENTER.anchor_size(rect.center(), galley.size());
    painter.galley(rect.min, galley, color);
}

// 泛型方法

// vec: ..., (beg_item. beg_pos), (beg_next_item, beg_next_pos), ..., (last_end_item, last_end_pos), (end_item, end_pos), ...
// 作用后
// vec: ..., (beg_item. end_pos), (beg_next_item, beg_pos), ..., (last_end_item, llast_end_pos), (end_item, last_end_pos), ...
fn rotate_one_pos<T: PosItem>(vec: &mut Vec<T>, beg_idx: usize, end_idx: usize) {
    let end_rect = shift_one_pos(vec, beg_idx, end_idx);
    vec[beg_idx].update_item_rect(end_rect);
}
// vec: ..., (beg_item. beg_pos), (beg_next_item, beg_next_pos), ..., (last_end_item, last_end_pos), (end_item, end_pos), ...
// 作用后
// vec: ..., (beg_item. beg_pos), (beg_next_item, beg_pos), ..., (last_end_item, llast_end_pos), (end_item, last_end_pos), ...
// 同时返回end_pos
fn shift_one_pos<T: PosItem>(vec: &mut Vec<T>, mut beg_idx: usize, mut end_idx: usize) -> Rect {
    // 实现的目标，beg_idx 的rect移动到下一个指标的rect上。 下一个指标的方向依赖于end_idx和beg_idx的关系。并最终将end_idx的item_rect返回
    let len = vec.len();
    if len == 0 {
        panic!("显然，如果使用该函数，必须要求输入vec长度非零");
    }
    beg_idx = beg_idx.min(len - 1);
    end_idx = end_idx.min(len - 1);
    let res = vec[end_idx].item_rect();
    if beg_idx < end_idx {
        for i in ((beg_idx + 1)..=end_idx).rev() {
            let rect = vec[i - 1].item_rect();
            vec[i].update_item_rect(rect);
        }
    } else if beg_idx > end_idx {
        for i in end_idx..=(beg_idx - 1) {
            let rect = vec[i + 1].item_rect();
            vec[i].update_item_rect(rect);
        }
    };
    res
}
// Vec<impl PosItem>中移除一个item，同时保证余下的item位置合规。
fn vec_remove_item<T: PosItem>(vec: &mut Vec<T>, remove_idx: usize) -> T {
    let len = vec.len();
    if len == 0 {
        panic!("试图从长度为零的vec中删除一个元素");
    }
    if remove_idx >= len {
        panic!("试图删去一个index过大的item");
    }
    shift_one_pos(vec, remove_idx, len - 1);
    vec.remove(remove_idx)
}
// 拖拽指标为drag_idx的item，释放时探测对应的item指标为target，释放时drag的rect与target rect的关系为position
// drag_idx一定合法，target_idx和position由于是计算出的，不一定合法
fn vec_process_with_id_output<T: DragItem + Default + IntoContent>(
    vec: &mut Vec<T>,
    drag_idx: usize,
    mut target_idx: usize,
    mut position: Position,
    merge_used: &T::MergeUsed,
) -> Option<(Id, Id)> {
    let len = vec.len();
    if len == 0 {
        panic!("禁止对空数组进行操作");
    }
    if target_idx >= len {
        target_idx = len - 1;
        position = Position::Behind;
    }
    if position == Position::Before && drag_idx < target_idx {
        target_idx -= 1;
        position = Position::Behind;
    }
    if position == Position::Behind && drag_idx > target_idx {
        target_idx += 1;
        position = Position::Before;
    }
    if position == Position::Middle
        && !<T as IntoContent>::merge_able(&vec[drag_idx], &vec[target_idx])
    {
        if drag_idx > target_idx {
            position = Position::Behind
        } else {
            position = Position::Before
        }
    }
    if drag_idx == target_idx {
        return None;
    }
    match position {
        Position::Middle => {
            shift_one_pos(vec, drag_idx, len - 1);
            let drag_item = vec.remove(drag_idx);
            if target_idx > drag_idx {
                target_idx -= 1;
            }
            let target_item = mem::take(&mut vec[target_idx]);
            let id0 = drag_item.id();
            let id1 = target_item.id();
            vec[target_idx] = target_item.merge(drag_item, merge_used);
            Some((id0, id1))
        }
        _ => {
            rotate_one_pos(vec, drag_idx, target_idx);
            if drag_idx < target_idx {
                vec[drag_idx..=target_idx].rotate_left(1);
            } else {
                vec[target_idx..=drag_idx].rotate_right(1);
            }
            None
        }
    }
}

// 从给定路径读取扩展名在ext_list中的所有文件的完整文件名，扩展名，文件句柄
fn filter_file(
    dir_path: PathBuf,
    ext_list: &[&str],
) -> Result<Vec<(String, String, fs::File)>, String> {
    let entries = fs::read_dir(dir_path);
    match entries {
        Err(err) => Err(format!("无法打开该目录，错误 {}", err)),
        Ok(entries) => {
            let res = entries
                .map(|entry| match entry {
                    Err(f) => {
                        println!("file failed 5 {:?}", f);
                        None
                    }
                    Ok(dir_entry) => {
                        let path = dir_entry.path();
                        match (dir_entry.file_name().to_str(), path.extension()) {
                            (None, f) => {
                                println!("file filed 1 {:?}", f);
                                None
                            }
                            (f, None) => {
                                println!("file filed 2 {:?}", f);
                                None
                            }
                            (Some(filename), Some(ext)) => {
                                match ext.to_ascii_lowercase().to_str() {
                                    None => {
                                        println!("file filed 3 {:?}", filename);
                                        None
                                    }
                                    Some(ext) => {
                                        if ext_list.contains(&ext)
                                            && let Ok(file) = fs::File::open(path.clone())
                                        {
                                            Some((filename.to_string(), ext.to_owned(), file))
                                        } else {
                                            println!("file filed 4 {:?}", filename);
                                            None
                                        }
                                    }
                                }
                            }
                        }
                    }
                })
                .filter_map(|f| f)
                .collect();
            Ok(res)
        }
    }
}

use fs::File;
use image::{ImageFormat, RgbaImage};
use mime::Mime;
use std::io::{BufReader, Cursor, Seek, SeekFrom};
use thumbnailer::{ThumbnailSize, create_thumbnails};
// 获取图片对应的缩略图
fn thumbnail_get(
    reader: BufReader<File>,
    mime_type: Mime,
    size: ThumbnailSize,
) -> Result<RgbaImage, String> {
    let thumbnail = create_thumbnails(reader, mime_type, [size])
        .map_err(|_| format!("缩略图生成失败",))?
        .pop()
        .unwrap();
    let mut buf = Cursor::new(Vec::new());
    thumbnail
        .write_png(&mut buf)
        .map_err(|_| "缩略图转存失败")?;
    let img =
        image::load_from_memory_with_format(buf.get_ref().as_slice(), image::ImageFormat::Png)
            .map_err(|_| "内存内转存缩略图为png时失败")?
            .to_rgba8();
    Ok(img)
}
// 在单一线程中读取图片数据，以及获取缩略图数据
pub fn thread_load_image(
    name: &str,
    ext: &str,
    file: File,
) -> Result<(RgbaImage, RgbaImage), String> {
    let mut reader = BufReader::new(file);
    let (image_format, mime_type) = match ext {
        "jpg" => (ImageFormat::Jpeg, mime::IMAGE_JPEG),
        "jpeg" => (ImageFormat::Jpeg, mime::IMAGE_JPEG),
        "png" => (ImageFormat::Png, mime::IMAGE_PNG),
        _ => panic!(),
    };
    let full_fig = image::load(&mut reader, image_format).map_err(|err| format!("{:?}", err))?;
    let full_fig = RgbaImage::from(full_fig);
    reader
        .seek(SeekFrom::Start(0))
        .map_err(|e| format!("{}", e))?;
    let icon_fig = thumbnail_get(reader, mime_type, thumbnailer::ThumbnailSize::Small)
        .map_err(|err| format!("filename {}, {:?}", name, err))?;
    Ok((icon_fig, full_fig))
}
use rayon::prelude::*;
// 并行获取图片数据及缩略图数据
pub fn load_image(path: PathBuf) -> Result<Vec<(String, String, (RgbaImage, RgbaImage))>, String> {
    let file_vec = filter_file(path, &["png", "jpeg", "jpg"])?;
    println!("filter file length {}", file_vec.len());
    let image_item_vec: Result<Vec<(String, String, (RgbaImage, RgbaImage))>, String> = file_vec
        .into_par_iter()
        .map(|(name, ext, file)| {
            let (icon_fig, full_fig) = thread_load_image(&name, &ext, file)?;
            let res: Result<
                (
                    String,
                    String,
                    (
                        image::ImageBuffer<image::Rgba<u8>, Vec<u8>>,
                        image::ImageBuffer<image::Rgba<u8>, Vec<u8>>,
                    ),
                ),
                String,
            > = Ok((name, ext, (icon_fig, full_fig)));
            res
        })
        .collect();
    image_item_vec
}
