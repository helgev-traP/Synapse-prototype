use std::sync::Arc;

use crate::{
    context::SharedContext,
    cosmic,
    device::keyboard,
    events::{ElementState, UiEvent},
    renderer::Renderer,
    types::{
        color::Color,
        size::{Size, StdSize},
    },
    ui::{Dom, DomComPareResult, Widget},
    vertex::uv_vertex::UvVertex,
};

pub struct TextDescriptor {
    pub label: Option<String>,
    pub size: [Size; 2],

    pub font_size: f32,
    pub font_color: Color,
    pub text: String,

    pub editable: bool,
}

impl Default for TextDescriptor {
    fn default() -> Self {
        Self {
            label: None,
            size: [Size::Pixel(100.0), Size::Pixel(100.0)],
            font_size: 16.0,
            font_color: Color::Rgb8USrgb {
                r: 255,
                g: 255,
                b: 255,
            },
            text: "".to_string(),
            editable: false,
        }
    }
}

pub struct Text {
    label: Option<String>,
    size: [Size; 2],

    font_size: f32,
    font_color: Color,
    text: String,
    editable: bool,
}

impl Text {
    pub fn new(disc: TextDescriptor) -> Box<Self> {
        Box::new(Self {
            label: disc.label,
            size: disc.size,
            font_size: disc.font_size,
            font_color: disc.font_color,
            text: disc.text,
            editable: disc.editable,
        })
    }
}

impl<T: Send + 'static> Dom<T> for Text {
    fn build_widget_tree(&self) -> Box<dyn Widget<T>> {
        Box::new(TextNode {
            label: self.label.clone(),
            size: self.size,
            font_size: self.font_size,
            font_color: self.font_color,
            text: self.text.clone(),
            editable: self.editable,
            text_cursor: self.text.len(),
            redraw_texture: true,
            texture: None,
            vertex: None,
            index: None,
        })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct TextNode {
    label: Option<String>,
    size: [Size; 2],

    font_size: f32,
    font_color: Color,
    text: String,

    editable: bool,

    text_cursor: usize,

    redraw_texture: bool,
    texture: Option<Arc<wgpu::Texture>>,
    vertex: Option<Arc<Vec<UvVertex>>>,
    index: Option<Arc<Vec<u16>>>,
}

impl<T: Send + 'static> Widget<T> for TextNode {
    fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    fn widget_event(
        &mut self,
        event: &UiEvent,
        parent_size: [StdSize; 2],
        context: &SharedContext,
    ) -> crate::events::UiEventResult<T> {
        todo!()
    }

    fn is_inside(&self, position: [f32; 2], parent_size: [StdSize; 2], context: &SharedContext) -> bool {
        todo!()
    }

    fn update_widget_tree(&mut self, dom: &dyn Dom<T>) -> Result<(), ()> {
        todo!()
    }

    fn compare(&self, dom: &dyn Dom<T>) -> DomComPareResult {
        todo!()
    }

    fn size(&self) -> [Size; 2] {
        self.size
    }

    fn px_size(&self, parent_size:[StdSize; 2], context: &SharedContext) -> [f32; 2] {
        todo!()
    }

    fn default_size(&self) -> [f32; 2] {
        todo!()
    }

    fn render(
        &mut self,
        // ui environment
        parent_size: [StdSize; 2],
        // context
        context: &SharedContext,
        renderer: &Renderer,
        frame: u64,
    ) -> Vec<(
        Arc<wgpu::Texture>,
        Arc<Vec<UvVertex>>,
        Arc<Vec<u16>>,
        nalgebra::Matrix4<f32>,
    )> {
        todo!()
    }
}
