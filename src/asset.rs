use egui::{ColorImage, TextureHandle};

// chatgpt 生成
static DIR_ICON: &[u8] = include_bytes!("../asset/dir_chatgpt_img.png");
static SAVE_ICON: &[u8] = include_bytes!("../asset/save_icon_chatgpt.png");

fn load_icon(icon: &[u8], name: impl Into<String>, ctx: &egui::Context) -> TextureHandle {
    let img = image::load_from_memory_with_format(icon, image::ImageFormat::Png)
        .expect("无法加载图标")
        .to_rgba8();
    let size = img.dimensions();
    let size = [size.0 as usize, size.1 as usize];
    let img_data = img.into_raw();
    let img_data = img_data.as_slice();

    ctx.load_texture(
        name,
        ColorImage::from_rgba_unmultiplied(size, img_data),
        Default::default(),
    )
}

pub fn load_dir_icon(ctx: &egui::Context) -> TextureHandle {
    load_icon(DIR_ICON, "dir_icon", ctx)
}
pub fn load_save_icon(ctx: &egui::Context) -> TextureHandle {
    load_icon(SAVE_ICON, "save_icon", ctx)
}
