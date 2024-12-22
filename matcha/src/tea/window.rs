use std::sync::Arc;

use super::{component::Component, context, renderer::Renderer, types::color::Color, ui::Widget};

mod benchmark;
mod gpu_state;
mod keyboard_state;
mod mouse_state;

pub struct Window<'a, Model: Send + 'static, Message: 'static> {
    // --- rendering context ---

    // boot status
    performance: wgpu::PowerPreference,
    title: String,
    init_size: [u32; 2],
    maximized: bool,
    full_screen: bool,
    font_context: Option<crate::cosmic::FontContext>,
    base_color: Color,

    // rendering
    winit_window: Option<Arc<winit::window::Window>>,
    gpu_state: Option<gpu_state::GpuState<'a>>,
    context: Option<context::SharedContext>,
    renderer: Option<Renderer>,

    // root component
    root_component: Component<Model, Message, Message, Message>,
    root_widget: Option<Box<dyn Widget<Message>>>,

    // frame
    frame: u64,

    // --- input and event handling ---

    // mouse
    mouse_state: Option<mouse_state::MouseState>,
    mouse_primary_button: winit::event::MouseButton,
    scroll_pixel_per_line: f32,

    // keyboard
    keyboard_state: Option<keyboard_state::KeyboardState>,

    // --- benchmark ---
    benchmark: Option<benchmark::Benchmark>,
}

// build chain
impl<Model: Send, Message: 'static> Window<'_, Model, Message> {
    pub fn new(component: Component<Model, Message, Message, Message>) -> Self {
        Self {
            performance: wgpu::PowerPreference::default(),
            title: "Tea".to_string(),
            init_size: [800, 600],
            maximized: false,
            full_screen: false,
            font_context: None,
            base_color: Color::Rgb8USrgb { r: 0, g: 0, b: 0 },
            winit_window: None,
            gpu_state: None,
            context: None,
            renderer: None,
            root_component: component,
            root_widget: None,
            frame: 0,
            mouse_state: None,
            mouse_primary_button: winit::event::MouseButton::Left,
            scroll_pixel_per_line: 40.0,
            keyboard_state: None,
            benchmark: None,
        }
    }

    // design

    pub fn base_color(&mut self, color: Color) {}

    pub fn performance(&mut self, performance: wgpu::PowerPreference) {}

    pub fn title(&mut self, title: &str) {}

    pub fn init_size(&mut self, size: [u32; 2]) {}

    pub fn maximized(&mut self, maximized: bool) {}

    pub fn full_screen(&mut self, full_screen: bool) {}

    pub fn font_context(&mut self, font_context: crate::cosmic::FontContext) {}

    // input

    pub fn mouse_primary_button(&mut self, button: crate::device::mouse::MousePhysicalButton) {
        match button {
            crate::device::mouse::MousePhysicalButton::Left => {
                self.mouse_primary_button = winit::event::MouseButton::Left;
            }
            crate::device::mouse::MousePhysicalButton::Right => {
                self.mouse_primary_button = winit::event::MouseButton::Right;
            }
            crate::device::mouse::MousePhysicalButton::Middle => {
                self.mouse_primary_button = winit::event::MouseButton::Middle;
            }
        }
    }

    pub fn scroll_pixel_per_line(&mut self, pixel: f32) {
        self.scroll_pixel_per_line = pixel;
    }
}

// winit event handler
impl<Model: Send, Message: 'static> Window<'_, Model, Message> {
    fn render(&mut self) {}
}

// winit event handler
impl<Model: Send, Message: 'static> winit::application::ApplicationHandler<Message>
    for Window<'_, Model, Message>
{
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let _ = event_loop;
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let _ = (event_loop, window_id, event);
    }

    fn new_events(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        cause: winit::event::StartCause,
    ) {
        let _ = (event_loop, cause);
    }

    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: Message) {
        let _ = (event_loop, event);
    }

    fn device_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        let _ = (event_loop, device_id, event);
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let _ = event_loop;
    }

    fn suspended(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let _ = event_loop;
    }

    fn exiting(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let _ = event_loop;
    }

    fn memory_warning(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let _ = event_loop;
    }
}
