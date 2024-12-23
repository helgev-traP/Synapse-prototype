use super::{component::Component, types::color::Color, window::Window};

pub struct App<'a, Model: Send + 'static, Message: 'static> {
    window: Window<'a, Model, Message>,

    // dummy field to store channel
    rx: Option<std::sync::channel::Receiver<Message>>,
    tx: Option<std::sync::mpsc::Sender<Message>>,
}

impl<Model: Send + 'static, Message: 'static> App<'_, Model, Message> {
    pub fn new(component: Component<Model, Message, Message, Message>) -> Self {
        Self {
            window: Window::new(component),
        }
    }

    pub fn title(mut self, title: &str) -> Self {
        self.window.title(title);
        self
    }

    pub fn base_color(mut self, color: Color) -> Self {
        self.window.base_color(color);
        self
    }

    pub fn communicate(mut self, rx: std::sync::channel::Receiver<Message>, tx: std::sync::mpsc::Sender<Message>) -> Self {
        self
        self
    }

    pub fn run(&mut self) {
        // let event_loop = winit::event_loop::EventLoop::with_user_event().build().unwrap();
        // event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
        // let _ = event_loop.run_app(&mut self.window);

        // simulate messages from the GUI.

        // self.rx.as_ref().unwrap().send(
        //     {some message},
        // ).unwrap();
    }
}
