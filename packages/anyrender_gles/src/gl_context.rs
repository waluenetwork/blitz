use anyhow::Result;
use glutin::{
    config::{Config, ConfigTemplateBuilder},
    context::{ContextApi, ContextAttributesBuilder, Version},
    display::{Display, DisplayApiPreference},
    prelude::*,
    surface::{Surface, SurfaceAttributesBuilder, WindowSurface},
};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::ffi::CString;
use std::num::NonZeroU32;

pub struct GlContext {
    display: Display,
    context: glutin::context::PossiblyCurrentContext,
    surface: Surface<WindowSurface>,
    config: Config,
}

impl GlContext {
    pub fn new<W>(window: &W) -> Result<Self>
    where
        W: HasWindowHandle + HasDisplayHandle + ?Sized,
    {
        let raw_display_handle = window.display_handle()?.as_raw();
        let raw_window_handle = window.window_handle()?.as_raw();

        let display = unsafe {
            Display::new(raw_display_handle, DisplayApiPreference::Egl)?
        };

        let template = ConfigTemplateBuilder::new()
            .with_alpha_size(8)
            .with_depth_size(24)
            .with_stencil_size(8)
            .with_multisampling(4)
            .build();

        let config = unsafe { display.find_configs(template)? }
            .reduce(|accum, config| {
                let transparency_check = config.supports_transparency().unwrap_or(false)
                    & !accum.supports_transparency().unwrap_or(false);

                if transparency_check || config.num_samples() > accum.num_samples() {
                    config
                } else {
                    accum
                }
            })
            .ok_or_else(|| anyhow::anyhow!("No suitable GL config found"))?;

        let context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::Gles(Some(Version::new(3, 0))))
            .build(Some(raw_window_handle));

        let not_current_context = unsafe {
            display.create_context(&config, &context_attributes)?
        };

        let surface_attributes = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            raw_window_handle,
            NonZeroU32::new(800).unwrap(),
            NonZeroU32::new(600).unwrap(),
        );

        let surface = unsafe {
            display.create_window_surface(&config, &surface_attributes)?
        };

        let context = not_current_context.make_current(&surface)?;

        gl::load_with(|symbol| {
            let c_str = CString::new(symbol).unwrap();
            display.get_proc_address(c_str.as_c_str()).cast()
        });

        Ok(Self {
            display,
            context,
            surface,
            config,
        })
    }

    pub fn make_current(&self) -> Result<()> {
        self.context.make_current(&self.surface)?;
        Ok(())
    }

    pub fn swap_buffers(&self) -> Result<()> {
        self.surface.swap_buffers(&self.context)?;
        Ok(())
    }

    pub fn resize(&self, width: u32, height: u32) -> Result<()> {
        self.surface.resize(
            &self.context,
            NonZeroU32::new(width).unwrap_or(NonZeroU32::new(1).unwrap()),
            NonZeroU32::new(height).unwrap_or(NonZeroU32::new(1).unwrap()),
        );
        unsafe {
            gl::Viewport(0, 0, width as i32, height as i32);
        }
        Ok(())
    }

    pub fn get_framebuffer_size(&self) -> (u32, u32) {
        (800, 600)
    }
}
