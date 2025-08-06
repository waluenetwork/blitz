use anyhow::Result;
use gl::types::*;
use rustc_hash::FxHashMap;
use std::ffi::CString;

pub struct ShaderManager {
    programs: FxHashMap<ShaderType, GLuint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderType {
    Fill,
    Stroke,
    Texture,
    Text,
    Presentation,
}

impl ShaderManager {
    pub fn new() -> Result<Self> {
        let mut manager = Self {
            programs: FxHashMap::default(),
        };
        
        manager.compile_all_shaders()?;
        Ok(manager)
    }

    fn compile_all_shaders(&mut self) -> Result<()> {
        let fill_program = self.create_shader_program(
            FILL_VERTEX_SHADER,
            FILL_FRAGMENT_SHADER,
        )?;
        self.programs.insert(ShaderType::Fill, fill_program);

        let stroke_program = self.create_shader_program(
            STROKE_VERTEX_SHADER,
            STROKE_FRAGMENT_SHADER,
        )?;
        self.programs.insert(ShaderType::Stroke, stroke_program);

        let texture_program = self.create_shader_program(
            TEXTURE_VERTEX_SHADER,
            TEXTURE_FRAGMENT_SHADER,
        )?;
        self.programs.insert(ShaderType::Texture, texture_program);

        let text_program = self.create_shader_program(
            TEXT_VERTEX_SHADER,
            TEXT_FRAGMENT_SHADER,
        )?;
        self.programs.insert(ShaderType::Text, text_program);

        let presentation_program = self.create_shader_program(
            PRESENTATION_VERTEX_SHADER,
            PRESENTATION_FRAGMENT_SHADER,
        )?;
        self.programs.insert(ShaderType::Presentation, presentation_program);

        Ok(())
    }

    fn create_shader_program(&self, vertex_source: &str, fragment_source: &str) -> Result<GLuint> {
        unsafe {
            let vertex_shader = self.compile_shader(vertex_source, gl::VERTEX_SHADER)?;
            let fragment_shader = self.compile_shader(fragment_source, gl::FRAGMENT_SHADER)?;

            let program = gl::CreateProgram();
            gl::AttachShader(program, vertex_shader);
            gl::AttachShader(program, fragment_shader);
            gl::LinkProgram(program);

            let mut success = 0;
            gl::GetProgramiv(program, gl::LINK_STATUS, &mut success);
            if success == 0 {
                let mut len = 0;
                gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
                let mut buffer = vec![0u8; len as usize];
                gl::GetProgramInfoLog(
                    program,
                    len,
                    std::ptr::null_mut(),
                    buffer.as_mut_ptr() as *mut GLchar,
                );
                return Err(anyhow::anyhow!(
                    "Shader linking failed: {}",
                    String::from_utf8_lossy(&buffer)
                ));
            }

            gl::DeleteShader(vertex_shader);
            gl::DeleteShader(fragment_shader);

            Ok(program)
        }
    }

    unsafe fn compile_shader(&self, source: &str, shader_type: GLenum) -> Result<GLuint> {
        unsafe {
            let shader = gl::CreateShader(shader_type);
            let c_str = CString::new(source)?;
            gl::ShaderSource(shader, 1, &c_str.as_ptr(), std::ptr::null());
            gl::CompileShader(shader);

            let mut success = 0;
            gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
            if success == 0 {
                let mut len = 0;
                gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
                let mut buffer = vec![0u8; len as usize];
                gl::GetShaderInfoLog(
                    shader,
                    len,
                    std::ptr::null_mut(),
                    buffer.as_mut_ptr() as *mut GLchar,
                );
                return Err(anyhow::anyhow!(
                    "Shader compilation failed: {}",
                    String::from_utf8_lossy(&buffer)
                ));
            }

            Ok(shader)
        }
    }

    pub fn use_program(&self, shader_type: ShaderType) -> Result<GLuint> {
        let program = self.programs.get(&shader_type)
            .ok_or_else(|| anyhow::anyhow!("Shader program not found: {:?}", shader_type))?;
        
        unsafe {
            gl::UseProgram(*program);
        }
        
        Ok(*program)
    }

    pub fn get_uniform_location(&self, shader_type: ShaderType, name: &str) -> Result<GLint> {
        let program = self.programs.get(&shader_type)
            .ok_or_else(|| anyhow::anyhow!("Shader program not found: {:?}", shader_type))?;
        
        let c_name = CString::new(name)?;
        let location = unsafe { gl::GetUniformLocation(*program, c_name.as_ptr()) };
        
        if location == -1 {
            return Err(anyhow::anyhow!("Uniform '{}' not found in shader {:?}", name, shader_type));
        }
        
        Ok(location)
    }
}

impl Drop for ShaderManager {
    fn drop(&mut self) {
        unsafe {
            for program in self.programs.values() {
                gl::DeleteProgram(*program);
            }
        }
    }
}

const FILL_VERTEX_SHADER: &str = r#"
#version 300 es
precision mediump float;

layout(location = 0) in vec2 a_position;
layout(location = 1) in vec4 a_color;

uniform mat4 u_transform;
uniform mat4 u_projection;

out vec4 v_color;

void main() {
    gl_Position = u_projection * u_transform * vec4(a_position, 0.0, 1.0);
    v_color = a_color;
}
"#;

const FILL_FRAGMENT_SHADER: &str = r#"
#version 300 es
precision mediump float;

in vec4 v_color;
out vec4 fragColor;

void main() {
    fragColor = v_color;
}
"#;

const STROKE_VERTEX_SHADER: &str = r#"
#version 300 es
precision mediump float;

layout(location = 0) in vec2 a_position;
layout(location = 1) in vec4 a_color;

uniform mat4 u_transform;
uniform mat4 u_projection;

out vec4 v_color;

void main() {
    gl_Position = u_projection * u_transform * vec4(a_position, 0.0, 1.0);
    v_color = a_color;
}
"#;

const STROKE_FRAGMENT_SHADER: &str = r#"
#version 300 es
precision mediump float;

in vec4 v_color;
out vec4 fragColor;

void main() {
    fragColor = v_color;
}
"#;

const TEXTURE_VERTEX_SHADER: &str = r#"
#version 300 es
precision mediump float;

layout(location = 0) in vec2 a_position;
layout(location = 1) in vec2 a_texcoord;

uniform mat4 u_transform;
uniform mat4 u_projection;

out vec2 v_texcoord;

void main() {
    gl_Position = u_projection * u_transform * vec4(a_position, 0.0, 1.0);
    v_texcoord = a_texcoord;
}
"#;

const TEXTURE_FRAGMENT_SHADER: &str = r#"
#version 300 es
precision mediump float;

in vec2 v_texcoord;
uniform sampler2D u_texture;

out vec4 fragColor;

void main() {
    fragColor = texture(u_texture, v_texcoord);
}
"#;

const TEXT_VERTEX_SHADER: &str = r#"
#version 300 es
precision mediump float;

layout(location = 0) in vec2 a_position;
layout(location = 1) in vec2 a_texcoord;
layout(location = 2) in vec4 a_color;

uniform mat4 u_transform;
uniform mat4 u_projection;

out vec2 v_texcoord;
out vec4 v_color;

void main() {
    gl_Position = u_projection * u_transform * vec4(a_position, 0.0, 1.0);
    v_texcoord = a_texcoord;
    v_color = a_color;
}
"#;

const TEXT_FRAGMENT_SHADER: &str = r#"
#version 300 es
precision mediump float;

in vec2 v_texcoord;
in vec4 v_color;
uniform sampler2D u_texture;

out vec4 fragColor;

void main() {
    float alpha = texture(u_texture, v_texcoord).r;
    fragColor = vec4(v_color.rgb, v_color.a * alpha);
}
"#;

const PRESENTATION_VERTEX_SHADER: &str = r#"
#version 300 es
precision mediump float;

layout(location = 0) in vec2 a_position;
layout(location = 1) in vec2 a_texcoord;

out vec2 v_texcoord;

void main() {
    gl_Position = vec4(a_position, 0.0, 1.0);
    v_texcoord = a_texcoord;
}
"#;

const PRESENTATION_FRAGMENT_SHADER: &str = r#"
#version 300 es
precision mediump float;

in vec2 v_texcoord;
uniform sampler2D u_texture;

out vec4 fragColor;

void main() {
    fragColor = texture(u_texture, v_texcoord);
}
"#;
