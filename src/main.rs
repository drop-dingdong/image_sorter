use eframe;
use eframe::Error;
use egui::Color32;
use image_sorter::app::ImageSorter;
fn main() -> Result<(), Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1300.0, 1040.0])
            .with_max_inner_size([1300.0, 1040.0])
            .with_min_inner_size([1300.0, 1040.0])
            .with_resizable(false),
        ..Default::default()
    };
    eframe::run_native(
        "Image Sorter",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);
            // 下面的设置没用
            let mut visuals = egui::Visuals::light();
            visuals.extreme_bg_color = Color32::GRAY;
            cc.egui_ctx.set_visuals(visuals);

            Ok(Box::new(ImageSorter::new(cc)))
        }),
    )
}
