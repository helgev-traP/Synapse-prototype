use nalgebra as na;
use std::any::Any;
use std::sync::Arc;
use wgpu::ImageCopyTextureBase;

use crate::events::UiEvent;
use crate::types::size::{Size, StdSize};
use crate::{
    context::SharedContext,
    events::UiEventResult,
    ui::{Dom, DomComPareResult, Widget},
    vertex::uv_vertex::UvVertex,
};

pub struct TeacupDescriptor {
    pub label: Option<String>,
    pub size: [Size; 2],
    pub frame_size: [Size; 2],
    pub position: [f32; 2],
    pub rotate: f32,
    pub visible: bool,
}

impl Default for TeacupDescriptor {
    fn default() -> Self {
        Self {
            label: None,
            size: [Size::Pixel(100.0), Size::Pixel(100.0)],
            frame_size: [Size::Pixel(100.0), Size::Pixel(100.0)],
            position: [0.0, 0.0],
            rotate: 0.0,
            visible: true,
        }
    }
}

pub struct Teacup {
    label: Option<String>,
    size: [Size; 2],
    frame_size: [Size; 2],
    position: [f32; 2],
    rotate_dig: f32,
    visible: bool,
}

impl Teacup {
    pub fn new(disc: TeacupDescriptor) -> Box<Self> {
        Box::new(Self {
            label: disc.label,
            size: disc.size,
            frame_size: disc.frame_size,
            position: disc.position,
            rotate_dig: disc.rotate,
            visible: disc.visible,
        })
    }
}

impl<R: 'static> Dom<R> for Teacup {
    fn build_widget_tree(&self) -> Box<dyn Widget<R>> {
        todo!()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct TeacupRenderNode {
    label: Option<String>,

    teacup_rgba: image::RgbaImage,
    picture_size: [u32; 2],
    position: [f32; 2],
    rotate: f32,

    size: crate::types::size::Size,
    frame_size: crate::types::size::Size,

    visible: bool,

    texture: Option<Arc<wgpu::Texture>>,
    vertex: Arc<Vec<UvVertex>>,
    index: Arc<Vec<u16>>,
}

impl<R: 'static> Widget<R> for TeacupRenderNode {
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
        parent_size: [StdSize; 2],
        context: &SharedContext,
    ) -> UiEventResult<R> {
        todo!()
    }

    fn size(&self) -> [Size; 2] {
        todo!()
    }

    fn px_size(&self, parent_size: [StdSize; 2], context: &SharedContext) -> [f32; 2] {
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
        renderer: &crate::renderer::Renderer,
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
