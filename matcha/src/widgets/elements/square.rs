use std::sync::Arc;

use crate::{
    context::SharedContext,
    events::UiEvent,
    renderer::Renderer,
    types::{
        color::Color,
        size::Size,
    },
    ui::{Dom, DomComPareResult, Widget},
    vertex::{
        colored_vertex::ColoredVertex,
        uv_vertex::UvVertex,
        vertex_generator::{BorderDescriptor, RectangleDescriptor},
    },
};

pub struct SquareDescriptor {
    pub label: Option<String>,
    pub size: [Size; 2],
    pub radius: f32,
    pub background_color: Color,

    pub border_width: f32,
    pub border_color: Color,
}

impl Default for SquareDescriptor {
    fn default() -> Self {
        Self {
            label: None,
            size: [Size::Pixel(100.0), Size::Pixel(100.0)],
            radius: 0.0,
            background_color: Color::Rgb8USrgb { r: 0, g: 0, b: 0 },
            border_width: 0.0,
            border_color: Color::Rgb8USrgb { r: 0, g: 0, b: 0 },
        }
    }
}

pub struct Square {
    label: Option<String>,
    size: [Size; 2],
    radius: f32,

    background_color: Color,

    border_width: f32,
    border_color: Color,
}

impl Square {
    pub fn new(disc: SquareDescriptor) -> Box<Self> {
        Box::new(Self {
            label: disc.label,
            size: disc.size,
            radius: disc.radius,
            background_color: disc.background_color,
            border_width: disc.border_width,
            border_color: disc.border_color,
        })
    }
}

impl<R: Copy + Send + 'static> Dom<R> for Square {
    fn build_widget_tree(&self) -> Box<dyn Widget<R>> {
        Box::new(SquareWidget {
            label: self.label.clone(),
            size: self.size,
            radius: self.radius,
            background_color: self.background_color,
            border_width: self.border_width,
            border_color: self.border_color,
            scene: vello::Scene::new(),
            texture: None,
            vertex: None,
            index: Arc::new(vec![0, 1, 2, 0, 2, 3]),
        })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct SquareWidget {
    label: Option<String>,

    size: [Size; 2],
    radius: f32,
    background_color: Color,
    border_width: f32,
    border_color: Color,

    // rendering
    scene: vello::Scene,
    texture: Option<Arc<wgpu::Texture>>,
    vertex: Option<Arc<Vec<UvVertex>>>,
    index: Arc<Vec<u16>>,
}

impl<R: Copy + Send + 'static> Widget<R> for SquareWidget {
    fn label(&self) -> Option<&str> {
        todo!()
    }

    fn update_widget_tree(&mut self, dom: &dyn Dom<R>) -> Result<(), ()> {
        todo!()
    }

    fn compare(&self, dom: &dyn Dom<R>) -> DomComPareResult {
        todo!()
    }

    fn widget_event(
        &mut self,
        event: &UiEvent,
        parent_size: [crate::types::size::StdSize; 2],
        context: &SharedContext,
    ) -> crate::events::UiEventResult<R> {
        todo!()
    }

    fn size(&self) -> [Size; 2] {
        todo!()
    }

    fn px_size(&self, parent_size: [crate::types::size::StdSize; 2], context: &SharedContext) -> [f32; 2] {
        todo!()
    }

    fn default_size(&self) -> [f32; 2] {
        todo!()
    }

    fn render(
        &mut self,
        // ui environment
        parent_size: [crate::types::size::StdSize; 2],
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
