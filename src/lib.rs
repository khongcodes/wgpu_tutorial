use winit::{
    event::*,
    event_loop::{ ControlFlow, EventLoop },
    window::{ WindowBuilder, Window },
    dpi::PhysicalSize
};

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
   surface: wgpu::Surface,
   device: wgpu::Device,
   queue: wgpu::Queue,
   config: wgpu::SurfaceConfiguration,
   size: winit::dpi::PhysicalSize<u32>,
   window: Window,
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

      Self {
         window,
         surface,
         device,
         queue,
         config,
         size
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
      false
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
      let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { 
         label:Some("Render Pass"),
         color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &view,
            resolve_target: None,
            ops: wgpu::Operations {
               load: wgpu::LoadOp::Clear(wgpu::Color {
                  r: 0.1,
                  g: 0.2,
                  b: 0.3,
                  a: 1.0,
               }),
               store:true,
            }
         })], 
         depth_stencil_attachment: None, 
      });

      drop(_render_pass);

      // Finish the command buffer and send to gpu's render queue
      self.queue.submit(std::iter::once(encoder.finish()));
      output.present();

      Ok(())
   }
}