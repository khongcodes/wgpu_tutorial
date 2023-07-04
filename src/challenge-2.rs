use winit::{
    event::*,
    event_loop::{ ControlFlow, EventLoop },
    window::{ WindowBuilder, Window },
    dpi::PhysicalSize
};
use rand::prelude::*;

#[cfg(target_arch="wasm32")]
use wasm_bindgen::prelude::*;

// if wasm32. wasm_bindgen should call run() when starting
#[cfg_attr(target_arch="wasm32", wasm_bindgen(start))]
pub async fn run() {

   // set and init console_log and console_error_panic_hook if wasm32
   // otherwise use env_logger
   cfg_if::cfg_if! {
      if #[cfg(target_arch="wasm32")] {
         std::panic::set_hook(Box::new(console_error_panic_hook::hook));
         console_log::init_with_level(log::Level::Warn).expect("Couldn't initialize logger");
      } else {
         env_logger::init();
      }
   }

   let event_loop = EventLoop::new();
   let window = WindowBuilder::new().build(&event_loop).unwrap();

   #[cfg(target_arch = "wasm32")]
   {
      // Winit prevents sizing with CSS so we have to set the size
      // manually when on web
      use winit::dpi::PhysicalSize;
      window.set_inner_size(PhysicalSize::new(450, 400));

      use winit::platform::web::WindowExtWebSys;
      web_sys::window()
         .and_then(|win| win.document())
         .and_then(|doc| {
            let dst = doc.get_element_by_id("wasm-entry")?;
            let canvas = web_sys::Element::from(window.canvas());
            dst.append_child(&canvas).ok()?;
            Some(())
         })
         .expect("Couldn't append canvas to document body.");
   }

   let mut state = State::new(window).await;

   event_loop.run(move |event, _, control_flow| match event {
      Event::WindowEvent { 
         window_id, 
         ref event 
      } if window_id == state.window.id() => if !state.input(event) {
         match event {
            WindowEvent::CloseRequested |
            WindowEvent::KeyboardInput {
               input: KeyboardInput {
                  state: ElementState::Pressed,
                  virtual_keycode: Some(VirtualKeyCode::Escape),
                  ..
               },
               ..
            } => *control_flow = ControlFlow::Exit,
            WindowEvent::Resized(physical_size) => {
               state.resize(*physical_size);
            },
            WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
               // new_inner_size is &&mut so we have to deref twice
               state.resize(**new_inner_size);
            },
            _ => {}
         }
      },
      Event::RedrawRequested(window_id) 
      if window_id == state.window().id() => {
         state.update();
         match state.render() {
            Ok(_) => {},
            // Reconfigure the surface if it is lost
            Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
            // The system is out of memory, we should quit
            Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
            // All other errors
            Err(e) => eprintln!("{:?}", e),
         }   
      },
      Event::MainEventsCleared => {
         // RedrawRequested will only trigger once, unless we manually
         // request it
         state.window().request_redraw();
      },
      _ => {}
   });

}

struct State {
   instance: wgpu::Instance,
   adapter: wgpu::Adapter,
   surface: wgpu::Surface,
   device: wgpu::Device,
   queue: wgpu::Queue,
   config: wgpu::SurfaceConfiguration,
   size: winit::dpi::PhysicalSize<u32>,
   window: Window,
   use_color: bool,
   render_pipeline: wgpu::RenderPipeline,
}


impl State {
   // Creating some wgpu types requires async code
   async fn new(window: Window) -> Self {
      let size = window.inner_size();

      // The instance is the first thing you create when using wgpu
      //    Main purpose is to create Adapters and Surfaces
      // 
      // Adapter is a handle to our actual graphics card
      // You can use this to get info about the graphics card
      // You use this to create Device and Queue
      // 
      // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
      let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
         backends: wgpu::Backends::all(),
         dx12_shader_compiler: Default::default(),
      });

      // The surface is the part of the window that we draw to
      // 
      // # Safety
      // 
      // The surface needs to live as long as the window that created it
      // State owns the window so this should be safe
      let surface = unsafe { instance.create_surface(&window) }.unwrap();

      let adapter = instance.request_adapter(
         &wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
         },
      ).await.unwrap();
      
      let (device, queue) = adapter.request_device(
         &wgpu::DeviceDescriptor {
            // WebGL doesn't support all of wgpu's features, so if
            // we're building for the web we'll have to disable some.
            // Available features may be dependent on device's GPU card
            features: wgpu::Features::empty(),
            // Available limits (describes limit of certain types of resources)
            // may be dependent on device's GPU card
            limits: if cfg!(target_arch = "wasm32") {
               wgpu::Limits::downlevel_webgl2_defaults()
            } else {
               wgpu::Limits::default()
            },
            label: None,
         },
         None,
      ).await.unwrap();

      let surface_caps = surface.get_capabilities(&adapter);

      // Shader code in this tutorial assumes an sRGB surface texture. Using a different
      // one will result all the colors coming out darker. If you want to support
      // non sRGB surfaces, you'll need to account for that when drawing to the frame
      let surface_format = surface_caps.formats.iter()
         .copied()
         .find(|f| f.is_srgb())
         .unwrap_or(surface_caps.formats[0]);

      // We define a config for our surface - how the surface creates its
      // underlying SurfaceTextures
      let config = wgpu::SurfaceConfiguration {
         usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
         format: surface_format,
         width: size.width,
         height: size.height,
         present_mode: surface_caps.present_modes[0],
         alpha_mode: surface_caps.alpha_modes[0],
         view_formats: vec![]
      };
      surface.configure(&device, &config);


      // SET UP PIPELINE

      let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
         label: Some("Shader"),
         source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
      });

      let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
         label: Some("Render Pipeline Layout"),
         bind_group_layouts: &[],
         push_constant_ranges: &[]
      });

      let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor { 
         label: Some("Render Pipeline"),
         layout: Some(&render_pipeline_layout), 
         vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main", // 1.
            buffers: &[] // 2.
         }, 
         fragment: Some(wgpu::FragmentState { // 3.
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState { // 4.
               format: config.format,
               blend: Some(wgpu::BlendState::REPLACE),
               write_mask: wgpu::ColorWrites::ALL
            })]
         }), 
         primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList, // 5.
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw, // 6.
            cull_mode: Some(wgpu::Face::Back),
            // below: Setting polygon_mode to anything other than Fill requires 
            //          Features::NON_FILL_POLYGON_MODE
            polygon_mode: wgpu::PolygonMode::Fill,
            // below: requires Features::DEPTH_CLIP_CONTROL
            unclipped_depth: false,
            // below: requires Features::CONSERVATIVE_RASTERIZATION
            conservative: false,
         }, 
         depth_stencil: None, // 7.
         multisample: wgpu::MultisampleState {
            count: 1, // 8.
            mask: !0, // 9.
            alpha_to_coverage_enabled: false, // 10.
         }, 
         multiview: None, // 11.
      });

      // 1. Specify which function inside the shader should be the entry_point:
      //       functions we marked with @vertex and @fragment
      // 
      // 2. buffers tells the wgpu what type of vertices we want to pass to the
      //       vertex shader - we're specifying the vertices in the shader itself
      //       so this can be empty
      // 
      // 3. Fragment is technically optional so we wrap it in Some()
      //       needed if we want to store color data to surface
      // 
      // 4. targets field tells wgpu what color outputs it should set up.
      //       We only need one for the surface. We use the surface's format
      //       so copying to the surface is easy.
      //       We specify that the blending should replace old pixel data with new
      //       We tell wgpu to write to R,G,B, and A (all colors)
      // 
      // 5. Using PrimitiveTopology::TriangleList means that every three vertices
      //       will correspond to one triangle
      // 
      // 6. front_face and cull_mode tell wgpu how to determine whether a given
      //       triangle is facing forward or noot
      //       FrontFace::Ccw means that a triangle is facing forward if the
      //       vertices are arranged in a counter-clockwise direction -
      //       triangles not facing forward are culled (not included in render)
      //       as specified by CullMode::Back
      // 
      // 7. We're not using a depth/stencil buffer so we leave this as None for now
      // 
      // 8. count field determines how many samples the pipeline will use
      // 
      // 9. mask field specifies which samples should be active
      // 
      // 10. alpha_to_coverage_enabled - anti-aliasing-related
      // 
      // 11. multiview - how many array layers the render attachments can have
      //       We won't be rendering to array textures so we can set this as None

      let use_color = true;

      Self {
         instance,
         adapter,
         window,
         surface,
         device,
         queue,
         config,
         size,
         clear_color: wgpu::Color::BLACK,
         use_color,
         render_pipeline
      }
   }

   pub fn window(&self) -> &Window {
      &self.window
   }

   fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
      if new_size.width > 0 && new_size.height > 0 {
         self.size = new_size;
         self.config.width = new_size.width;
         self.config.height = new_size.height;
         self.surface.configure(&self.device, &self.config);
      }
   }

   fn input(&mut self, event: &WindowEvent) -> bool {
      match event {
         WindowEvent::CursorMoved { position, ..} => {
            self.clear_color = wgpu::Color {
               r: position.x as f64 / self.size.width as f64,
               g: position.y as f64 / self.size.height as f64,
               b: 1.0,
               a: 1.0
            };
            true
         },
         WindowEvent::KeyboardInput { 
            input: KeyboardInput {
               state,
               virtual_keycode: Some(VirtualKeyCode::Space),
               ..
            }, .. } => {
            self.use_color = *state == ElementState::Released;
            true
         }
         _ => false
      }
   }

   fn update(&mut self) {
      // todo!()
   }

   fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
      // 1. get_current_texture will wait for surface to provide a new 
      //    SurfaceTexture that we will render to
      // 
      // 2. Createt a TextureView with default settings - to control 
      //    how render code interacts with the texture
      // 
      // 3. Create a CommandEncoder to create actual commands to send 
      //    to the gpu. Most modern graphics frameworks expect commands 
      //    to be stored in a command buffer before they are sent to gpu - 
      //    the encoder builds that command buffer
      let output = self.surface.get_current_texture()?;
      let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
      let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
         label: Some("Render Encoder"),
      });

      // Now we can clear the screen - we need to use the encoder to create
      // a RenderPass - this has all the methods for actual drawing
      // 
      // begin_render_pass borrows encoder mutably and we can't call
      //    encoder.finish() until we release that mutable borrow
      // 
      // RenderPassColorAttachment fields - 
      //    view - tells wgpu what texture to save the colors to
      // 
      //    resolve_target - texture that will receive resolved output
      // 
      //    ops - takes wgpu::Operations object; tells wgpu what to do
      //          with the colors on the texture
      let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { 
         label:Some("Render Pass"),
         color_attachments: &[
            // This is what @location(0) in the fragment shader targets
            Some(wgpu::RenderPassColorAttachment {
               view: &view,
               resolve_target: None,
               ops: wgpu::Operations {
                  load: wgpu::LoadOp::Clear(self.clear_color),
                  store:true,
               }
            }
         )], 
         depth_stencil_attachment: None, 
      });

      // After we set the pipeline to our built render pipeline, we can 
      //    tell wgpu too draw smoething with 3 vertices and 1 instance
      render_pass.set_pipeline(&self.render_pipeline);
      render_pass.draw(0..3, 0..1);

      drop(render_pass);

      // Finish the command buffer and send to gpu's render queue
      self.queue.submit(std::iter::once(encoder.finish()));
      output.present();

      Ok(())
   }
}