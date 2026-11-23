use rust_flutter::{
    egui,
    element::{BuildContext, ElementTree, Widget},
    layout::BoxConstraints,
    state::{StreamState, StreamStatus, TaskStream, spawn_stream},
};
const STEPS: u32 = 20;

pub struct StreamBuilderDemo {
    tree: ElementTree,
    stream: TaskStream<u32, String>,
}

impl StreamBuilderDemo {
    pub fn new(ctx: &egui::Context) -> Self {
        let stream = start_stream(ctx);
        let root = stream.builder(render_snapshot);
        let tree = ElementTree::new(ctx, root).expect("valid retained stream tree");
        Self { tree, stream }
    }

    fn restart(&mut self, ctx: &egui::Context) {
        self.stream.cancel();
        self.stream = start_stream(ctx);
        self.tree
            .update(self.stream.builder(render_snapshot))
            .expect("valid retained stream source update");
    }

    pub fn cancel(&self) {
        self.stream.cancel();
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Restart stream").clicked() {
                self.restart(ui.ctx());
            }
            if ui.button("Cancel").clicked() {
                self.stream.cancel();
            }
        });
        let constraints = BoxConstraints::tight(egui::vec2(ui.available_width(), 74.0));
        match constraints
            .map_err(rust_flutter::element::ElementError::from)
            .and_then(|constraints| self.tree.show(ui, constraints))
        {
            Ok(_) => {}
            Err(error) => {
                ui.colored_label(egui::Color32::LIGHT_RED, error.to_string());
            }
        }
    }
}

fn start_stream(ctx: &egui::Context) -> TaskStream<u32, String> {
    spawn_stream(ctx, |sink| {
        for value in 1..=STEPS {
            if sink.is_cancelled() {
                return Ok(());
            }
            sink.emit(value);
            std::thread::sleep(std::time::Duration::from_millis(120));
        }
        Ok(())
    })
}

fn render_snapshot(_: &BuildContext, snapshot: StreamState<u32, String>) -> Widget {
    let value = snapshot.data.as_deref().copied().unwrap_or(0);
    let progress = value as f32 / STEPS as f32;
    let status = match snapshot.status {
        StreamStatus::Pending => "Waiting for the first update".to_owned(),
        StreamStatus::Active => format!("Latest update: {value}/{STEPS}"),
        StreamStatus::Done => format!("Complete: {value}/{STEPS}"),
        StreamStatus::Panicked => "The producer panicked".to_owned(),
        StreamStatus::Cancelled => "The producer was cancelled".to_owned(),
    };
    let error = snapshot.error.as_deref().cloned();
    Widget::leaf(egui::vec2(320.0, 74.0), move |ui| {
        ui.label(&status);
        if let Some(error) = &error {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
        ui.add(egui::ProgressBar::new(progress).desired_width(ui.available_width()));
    })
}
