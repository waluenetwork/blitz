
mod gl_context;
mod image_renderer;
mod scene;
mod window_renderer;
mod tessellation;
mod text_renderer;
mod shader_manager;

pub use gl_context::*;
pub use image_renderer::GlesImageRenderer;
pub use scene::GlesScenePainter;
pub use window_renderer::GlesWindowRenderer;

pub use gl;

use std::num::NonZeroUsize;

#[cfg(target_os = "macos")]
const DEFAULT_THREADS: Option<NonZeroUsize> = NonZeroUsize::new(1);
#[cfg(not(target_os = "macos"))]
const DEFAULT_THREADS: Option<NonZeroUsize> = None;
