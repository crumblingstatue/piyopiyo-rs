use {
    eframe::egui::{self, FontId},
    num_traits::AsPrimitive,
};

struct Out<T> {
    index: usize,
    value: T,
}

fn calc_waveform_out(pos: egui::Pos2, rect: egui::Rect) -> Out<i8> {
    let pos_min = pos - rect.min;
    let x = pos_min.x.clamp(0.0, 255.0);
    let y = 127.0 - pos_min.y.clamp(0.0, 255.0);
    Out {
        index: x as usize,
        value: y as i8,
    }
}

pub fn waveform_widget(ui: &mut egui::Ui, wave: &mut [i8; 256], prev_pos: &mut Option<egui::Pos2>) {
    let (rect, re) = ui.allocate_exact_size(egui::vec2(256.0, 256.0), egui::Sense::drag());
    let p = ui.painter_at(rect);
    p.rect(
        rect,
        1.0,
        egui::Color32::BLACK,
        egui::Stroke::new(1.0, egui::Color32::LIGHT_YELLOW),
        egui::StrokeKind::Inside,
    );
    p.text(
        rect.min + egui::vec2(2.0, 2.0),
        egui::Align2::LEFT_TOP,
        "Waveform",
        FontId::proportional(12.0),
        egui::Color32::WHITE,
    );
    p.line(
        wave.iter()
            .enumerate()
            .map(|(i, sample)| {
                let x = rect.min.x + i as f32;
                let y = rect.center().y - *sample as f32;
                egui::pos2(x, y)
            })
            .collect(),
        egui::Stroke::new(1.0, egui::Color32::LIGHT_BLUE),
    );
    if let Some(pos) = re.interact_pointer_pos() {
        let current = calc_waveform_out(pos, rect);
        if let Some(prev) = prev_pos {
            let prev = calc_waveform_out(*prev, rect);
            interpolate(&prev, &current, wave);
        }
        wave[current.index] = current.value;
        *prev_pos = Some(pos);
    } else {
        *prev_pos = None;
    }
}

fn calc_envelope_out(pos: egui::Pos2, rect: egui::Rect) -> Out<u8> {
    let pos = pos - rect.min;
    let x = pos.x.clamp(0.0, 63.0);
    let y = 128.0 - pos.y.clamp(0.0, 128.0);
    Out {
        index: x as usize,
        value: y as u8,
    }
}

pub fn envelope_widget(
    ui: &mut egui::Ui,
    envelope: &mut [u8; 64],
    prev_pos: &mut Option<egui::Pos2>,
) {
    let (rect, re) = ui.allocate_exact_size(egui::vec2(64.0, 128.0), egui::Sense::drag());
    let p = ui.painter_at(rect);
    p.rect(
        rect,
        1.0,
        egui::Color32::BLACK,
        egui::Stroke::new(1.0, egui::Color32::LIGHT_YELLOW),
        egui::StrokeKind::Inside,
    );
    p.text(
        rect.min + egui::vec2(2.0, 2.0),
        egui::Align2::LEFT_TOP,
        "Envelope",
        FontId::proportional(12.0),
        egui::Color32::WHITE,
    );
    p.line(
        envelope
            .iter()
            .enumerate()
            .map(|(i, vol)| {
                let x = rect.min.x + i as f32;
                let y = rect.max.y - (*vol as f32);
                egui::pos2(x, y)
            })
            .collect(),
        egui::Stroke::new(1.0, egui::Color32::LIGHT_BLUE),
    );
    if let Some(pos) = re.interact_pointer_pos() {
        let current = calc_envelope_out(pos, rect);
        if let Some(prev) = prev_pos {
            let prev = calc_envelope_out(*prev, rect);
            interpolate(&prev, &current, envelope);
        }
        envelope[current.index] = current.value;
        *prev_pos = Some(pos);
    } else {
        *prev_pos = None;
    }
}

fn interpolate<T, const N: usize>(prev: &Out<T>, current: &Out<T>, dest: &mut [T; N])
where
    T: AsPrimitive<f32>,
    f32: AsPrimitive<T>,
{
    let x = (current.index as isize - prev.index as isize).abs();
    let cur_f32: f32 = current.value.as_();
    let prev_f32: f32 = prev.value.as_();
    let y = cur_f32 - prev_f32;
    for i in 0..=x {
        let t = i as f32 / x as f32;
        let xi = if current.index > prev.index {
            prev.index + i as usize
        } else {
            prev.index - i as usize
        };
        let yi = prev_f32 + t * y;
        dest[xi] = yi.as_();
    }
}
