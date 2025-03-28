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
    vertex::uv_vertex::UvVertex,
};

pub struct ButtonDescriptor<T>
where
    T: Send + Clone + 'static,
{
    pub label: Option<String>,

    // default
    pub size: [Size; 2],
    pub radius: f32,
    pub background_color: Color,
    pub border_width: f32,
    pub border_color: Color,

    // hover
    pub hover_background_color: Option<Color>,
    pub hover_border_width: Option<f32>,
    pub hover_border_color: Option<Color>,

    // logic
    pub onclick: Option<T>,

    // inner content
    pub content_position: Option<nalgebra::Matrix4<f32>>,
    pub content: Option<Box<dyn Dom<T>>>,
}

impl<T> Default for ButtonDescriptor<T>
where
    T: Send + Clone + 'static,
{
    fn default() -> Self {
        Self {
            label: None,
            size: [Size::Pixel(100.0), Size::Pixel(100.0)],
            radius: 0.0,
            background_color: Color::Rgba8USrgb {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
            border_width: 0.0,
            border_color: Color::Rgba8USrgb {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
            hover_background_color: None,
            hover_border_width: None,
            hover_border_color: None,
            onclick: None,
            content_position: None,
            content: None,
        }
    }
}

impl<T> ButtonDescriptor<T>
where
    T: Send + Clone + 'static,
{
    pub fn new(label: Option<&str>) -> Self {
        Self {
            label: label.map(|s| s.to_string()),
            ..Default::default()
        }
    }

    pub fn normal(
        mut self,
        size: [Size; 2],
        radius: f32,
        background_color: Color,
        border_width: f32,
        border_color: Color,
    ) -> Self {
        self.size = size;
        self.radius = radius;
        self.background_color = background_color;
        self.border_width = border_width;
        self.border_color = border_color;
        self
    }

    pub fn hover(
        mut self,
        hover_background_color: Color,
        hover_border_width: f32,
        hover_border_color: Color,
    ) -> Self {
        self.hover_background_color = Some(hover_background_color);
        self.hover_border_width = Some(hover_border_width);
        self.hover_border_color = Some(hover_border_color);
        self
    }

    pub fn onclick(self, onclick: T) -> Self {
        ButtonDescriptor {
            label: self.label,
            size: self.size,
            radius: self.radius,
            background_color: self.background_color,
            border_width: self.border_width,
            border_color: self.border_color,
            hover_background_color: self.hover_background_color,
            hover_border_width: self.hover_border_width,
            hover_border_color: self.hover_border_color,
            onclick: Some(onclick),
            content_position: self.content_position,
            content: self.content,
        }
    }

    pub fn content(mut self, content: Box<dyn Dom<T>>) -> Self {
        self.content = Some(content);
        self
    }
}

pub struct Button<T>
where
    T: Send + Clone + 'static,
{
    label: Option<String>,

    // default
    size:   [Size; 2],
    radius: f32,
    background_color: Color,
    border_width: f32,
    border_color: Color,
    // hover
    hover_background_color: Option<Color>,
    hover_border_width: Option<f32>,
    hover_border_color: Option<Color>,
    // logic
    onclick: Option<T>,
    // inner content
    content_position: Option<nalgebra::Matrix4<f32>>,
    content: Option<Box<dyn Dom<T>>>,
}

impl<T> Button<T>
where
    T: Send + Clone + 'static,
{
    pub fn new(disc: ButtonDescriptor<T>) -> Box<Self> {
        Box::new(Self {
            label: disc.label,
            size: disc.size,
            radius: disc.radius,
            background_color: disc.background_color,
            border_width: disc.border_width,
            border_color: disc.border_color,
            hover_background_color: disc.hover_background_color,
            hover_border_width: disc.hover_border_width,
            hover_border_color: disc.hover_border_color,
            onclick: disc.onclick,
            content_position: disc.content_position,
            content: disc.content,
        })
    }
}

impl<T: Send + 'static> Dom<T> for Button<T>
where
    T: Send + Clone + 'static,
{
    fn build_widget_tree(&self) -> Box<dyn Widget<T>> {
        todo!()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct ButtonWidget<T>
where
    T: Send + Clone + 'static,
{
    label: Option<String>,
    // default
    size: Size,
    radius: f32,
    background_color: Color,
    border_width: f32,
    border_color: Color,
    // hover
    hover_background_color: Option<Color>,
    hover_border_width: Option<f32>,
    hover_border_color: Option<Color>,
    // logic
    onclick: Option<T>,
    // inner content
    content_position: Option<nalgebra::Matrix4<f32>>,
    content: Option<Box<dyn Widget<T>>>,

    // input status
    is_hover: bool,

    // rendering
    scene: vello::Scene,
    texture: Option<Arc<wgpu::Texture>>,
    texture_hover: Option<Arc<wgpu::Texture>>,
    vertex: Option<Arc<Vec<UvVertex>>>,
    index: Arc<Vec<u16>>,
}

impl<T: Send + 'static> Widget<T> for ButtonWidget<T>
where
    T: Send + Clone + 'static,
{
    fn label(&self) -> Option<&str> {
        todo!()
    }

    fn update_widget_tree(&mut self, dom: &dyn Dom<T>) -> Result<(), ()> {
        todo!()
    }

    fn compare(&self, dom: &dyn Dom<T>) -> DomComPareResult {
        todo!()
    }

    fn widget_event(
        &mut self,
        event: &UiEvent,
        parent_size: [crate::types::size::StdSize; 2],
        context: &SharedContext,
    ) -> crate::events::UiEventResult<T> {
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
