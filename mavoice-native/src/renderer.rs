use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// Uniform buffer layout for user shader — 48 bytes, matches shader.wgsl
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct UserUniforms {
    pub resolution: [f32; 2], // 8 bytes  (offset 0)
    pub time: f32,            // 4 bytes  (offset 8)
    pub intensity: f32,       // 4 bytes  (offset 12)
    pub levels: [f32; 4],     // 16 bytes (offset 16)
    pub color: [f32; 3],      // 12 bytes (offset 32)
    pub mode: f32,            // 4 bytes  (offset 44)
}                             // total: 48 bytes

/// Uniform buffer layout for AI shader — 48 bytes, matches ai_shader.wgsl
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct AiUniforms {
    pub resolution: [f32; 2], // 8 bytes  (offset 0)
    pub time: f32,            // 4 bytes  (offset 8)
    pub intensity: f32,       // 4 bytes  (offset 12)
    pub levels: [f32; 4],     // 16 bytes (offset 16)
    pub color: [f32; 3],      // 12 bytes (offset 32)
    pub _pad: f32,            // 4 bytes  (offset 44)
}                             // total: 48 bytes

/// Shared GPU resources — created once, shared between both renderers
pub struct GpuContext {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub start_time: std::time::Instant,
}

impl GpuContext {
    pub async fn new() -> Self {
        // Use default backends — we don't need a surface for render-to-texture
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN | wgpu::Backends::GL,
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: None, // No surface needed — we render to texture
                force_fallback_adapter: false,
            })
            .await
            .expect("No suitable GPU adapter found");

        log::info!("GPU adapter: {:?}", adapter.get_info().name);

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("mavoice-device"),
                required_features: wgpu::Features::MAPPABLE_PRIMARY_BUFFERS,
                ..Default::default()
            })
            .await
            .or_else(|_| {
                // Fallback without MAPPABLE_PRIMARY_BUFFERS
                log::info!("Falling back to default device features");
                pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                    label: Some("mavoice-device"),
                    ..Default::default()
                }))
            })
            .expect("Failed to create device");

        Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
            start_time: std::time::Instant::now(),
        }
    }

    pub fn elapsed(&self) -> f32 {
        self.start_time.elapsed().as_secs_f32()
    }
}

/// Per-window renderer — renders shader to texture, blits to window via softbuffer.
/// Bypasses wgpu surface compositing (which is Opaque on NVIDIA X11) by writing
/// ARGB pixels directly to the X11 window's 32-bit backing pixmap.
pub struct Renderer {
    // GPU resources
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    // Render-to-texture target
    render_texture: wgpu::Texture,
    render_view: wgpu::TextureView,
    readback_buffer: wgpu::Buffer,
    // Softbuffer for X11 ARGB compositing
    _sb_context: softbuffer::Context<Arc<winit::window::Window>>,
    sb_surface: softbuffer::Surface<Arc<winit::window::Window>, Arc<winit::window::Window>>,
    // Dimensions
    pub width: u32,
    pub height: u32,
}

impl Renderer {
    pub fn new(
        gpu: &GpuContext,
        window: Arc<winit::window::Window>,
        shader_source: &str,
        uniform_size: usize,
    ) -> Self {
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        let format = wgpu::TextureFormat::Rgba8Unorm;

        // Create render target texture
        let render_texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("render-target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let render_view = render_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Readback buffer for texture → CPU transfer
        let bytes_per_row = Self::aligned_bytes_per_row(width);
        let readback_size = (bytes_per_row * height) as u64;
        let readback_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback-buffer"),
            size: readback_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Softbuffer context + surface for X11 ARGB presentation
        let sb_context =
            softbuffer::Context::new(window.clone()).expect("Failed to create softbuffer context");
        let sb_surface =
            softbuffer::Surface::new(&sb_context, window.clone()).expect("Failed to create softbuffer surface");

        // Shader module
        let shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("shader"),
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

        // Uniform buffer
        let uniform_data = vec![0u8; uniform_size];
        let uniform_buffer =
            gpu.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("uniform-buffer"),
                    contents: &uniform_data,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        // Bind group layout + bind group
        let bind_group_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("uniform-bind-group-layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let bind_group = gpu
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("uniform-bind-group"),
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }],
            });

        // Pipeline layout
        let pipeline_layout =
            gpu.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("pipeline-layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    immediate_size: 0,
                });

        // Render pipeline — premultiplied alpha blending
        let pipeline = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });

        Self {
            pipeline,
            uniform_buffer,
            bind_group,
            device: gpu.device.clone(),
            queue: gpu.queue.clone(),
            render_texture,
            render_view,
            readback_buffer,
            _sb_context: sb_context,
            sb_surface,
            width,
            height,
        }
    }

    /// Bytes per row aligned to wgpu's COPY_BYTES_PER_ROW_ALIGNMENT (256)
    fn aligned_bytes_per_row(width: u32) -> u32 {
        let unaligned = width * 4;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        (unaligned + align - 1) / align * align
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 || (width == self.width && height == self.height) {
            return;
        }
        self.width = width;
        self.height = height;

        // Recreate render texture
        self.render_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("render-target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        self.render_view = self
            .render_texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Recreate readback buffer
        let bytes_per_row = Self::aligned_bytes_per_row(width);
        let readback_size = (bytes_per_row * height) as u64;
        self.readback_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback-buffer"),
            size: readback_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Resize softbuffer
        let _ = self.sb_surface.resize(
            std::num::NonZeroU32::new(width).unwrap(),
            std::num::NonZeroU32::new(height).unwrap(),
        );
    }

    pub fn render_bytes(&mut self, uniform_bytes: &[u8]) {
        // Update uniform buffer
        self.queue
            .write_buffer(&self.uniform_buffer, 0, uniform_bytes);

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render-encoder"),
            });

        // Render shader to texture
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.render_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.draw(0..4, 0..1);
        }

        // Copy texture to readback buffer
        let bytes_per_row = Self::aligned_bytes_per_row(self.width);
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.render_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.readback_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        // Map readback buffer and blit to softbuffer
        let buffer_slice = self.readback_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        let _ = self.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        });

        if rx.recv().ok().and_then(|r| r.ok()).is_some() {
            let data = buffer_slice.get_mapped_range();
            let width = self.width as usize;
            let height = self.height as usize;
            let stride = bytes_per_row as usize;

            // Resize softbuffer to match render dimensions, then blit
            let _ = self.sb_surface.resize(
                std::num::NonZeroU32::new(self.width).unwrap(),
                std::num::NonZeroU32::new(self.height).unwrap(),
            );
            // Write to softbuffer — RGBA premultiplied → packed u32 (0xAARRGGBB for softbuffer)
            if let Ok(mut buffer) = self.sb_surface.buffer_mut() {
                for y in 0..height {
                    let row_start = y * stride;
                    for x in 0..width {
                        let px = row_start + x * 4;
                        let r = data[px] as u32;
                        let g = data[px + 1] as u32;
                        let b = data[px + 2] as u32;
                        let a = data[px + 3] as u32;

                        // Shader outputs straight alpha with sRGB gamma already applied.
                        // softbuffer uses 0xAARRGGBB format — pack directly.
                        buffer[y * width + x] = (a << 24) | (r << 16) | (g << 8) | b;
                    }
                }
                let _ = buffer.present();
            }
            drop(data);
        }
        self.readback_buffer.unmap();
    }
}
