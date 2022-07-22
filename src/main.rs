use byte_slice_cast::*;
use eframe::{egui, NativeOptions};
use egui::plot::{Line, Plot, Value, Values};
use egui::style::Visuals;
use std::io::prelude::*;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;

const PAYLOAD_SIZE: usize = 8192;
const CHANNELS: usize = 2048;

struct Measurements {
    stokes: [f32; CHANNELS],
}

impl Measurements {
    fn new() -> Self {
        Self {
            stokes: [0f32; CHANNELS],
        }
    }
    fn values(&self) -> Values {
        Values::from_values_iter(self.stokes.iter().enumerate().map(|(i, v)| {
            Value::new(
                1530f32 - (i as f32 * (250f32 / 2048f32)),
                10f32 * (*v).log10(),
            )
        }))
    }
    fn update_from_buf(&mut self, buf: &[u8]) {
        self.stokes = buf
            .as_slice_of::<f32>()
            .expect("TCP Payload should always be packed f32s")
            .try_into()
            .expect("We should have received exactly 2048 channels");
    }
}

struct App {
    values: Arc<Mutex<Measurements>>,
}

fn main() {
    let native_options = NativeOptions::default();
    // This starts up the UI thread
    eframe::run_native(
        "GReX Spectrum Monitor",
        native_options,
        Box::new(|cc| Box::new(App::new(cc))),
    );
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(Visuals::dark());
        let values = Arc::new(Mutex::new(Measurements::new()));
        // Clone the values for the thread
        let thread_values = values.clone();
        // Spawn thread to deal with TCP connection
        thread::spawn(move || {
            let mut stream = TcpStream::connect("127.0.0.1:4242").unwrap();
            let mut buf = [0u8; PAYLOAD_SIZE];
            loop {
                let n = stream.read(&mut buf).unwrap();
                if n < PAYLOAD_SIZE {
                    return;
                }
                thread_values
                    .lock()
                    .expect("Lock shouldn't fail")
                    .update_from_buf(&buf);
            }
        });
        Self { values }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Build line and plot
            let line = Line::new(self.values.lock().expect("Lock shouldn't fail").values());
            Plot::new("spectra")
                .allow_drag(true)
                .include_y(45)
                .include_y(55)
                .allow_boxed_zoom(true)
                .show(ui, |plot_ui| plot_ui.line(line));
            // Force redraw
            ctx.request_repaint();
        });
    }
}
