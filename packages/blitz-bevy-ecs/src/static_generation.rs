use crate::ecs_context::EcsWorldContext;
use bevy_ecs::prelude::*;
use dioxus::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StaticGenerationError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Route generation failed: {0}")]
    RouteError(String),
    #[error("Asset processing failed: {0}")]
    AssetError(String),
}

#[derive(Clone, Debug)]
pub struct RouteConfig {
    pub path: String,
    pub template: String,
    pub output_file: String,
    pub ecs_queries: Vec<String>,
}

impl RouteConfig {
    pub fn new(path: impl Into<String>, template: impl Into<String>) -> Self {
        let path = path.into();
        let output_file = if path == "/" {
            "index.html".to_string()
        } else {
            format!("{}.html", path.trim_start_matches('/').replace('/', "_"))
        };
        
        Self {
            path,
            template: template.into(),
            output_file,
            ecs_queries: Vec::new(),
        }
    }
    
    pub fn with_queries(mut self, queries: Vec<String>) -> Self {
        self.ecs_queries = queries;
        self
    }
}

pub struct StaticSiteGenerator {
    routes: Vec<RouteConfig>,
    output_dir: PathBuf,
    asset_dir: Option<PathBuf>,
    ecs_context: EcsWorldContext,
}

impl StaticSiteGenerator {
    pub fn new(output_dir: impl AsRef<Path>, ecs_context: EcsWorldContext) -> Self {
        Self {
            routes: Vec::new(),
            output_dir: output_dir.as_ref().to_path_buf(),
            asset_dir: None,
            ecs_context,
        }
    }
    
    pub fn add_route(mut self, route: RouteConfig) -> Self {
        self.routes.push(route);
        self
    }
    
    pub fn with_assets(mut self, asset_dir: impl AsRef<Path>) -> Self {
        self.asset_dir = Some(asset_dir.as_ref().to_path_buf());
        self
    }
    
    pub async fn generate(&self) -> Result<(), StaticGenerationError> {
        fs::create_dir_all(&self.output_dir)?;
        
        if let Some(ref asset_dir) = self.asset_dir {
            self.copy_assets(asset_dir)?;
        }
        
        for route in &self.routes {
            self.generate_route(route).await?;
        }
        
        Ok(())
    }
    
    async fn generate_route(&self, route: &RouteConfig) -> Result<(), StaticGenerationError> {
        let html_content = self.render_route_html(route).await?;
        
        let output_path = self.output_dir.join(&route.output_file);
        fs::write(output_path, html_content)?;
        
        Ok(())
    }
    
    async fn render_route_html(&self, route: &RouteConfig) -> Result<String, StaticGenerationError> {
        let template_html = self.process_template(&route.template, route)?;
        
        let full_html = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Generated Page</title>
    <style>
        body {{ font-family: system-ui, sans-serif; margin: 0; padding: 20px; }}
        .container {{ max-width: 1200px; margin: 0 auto; }}
    </style>
</head>
<body>
    <div class="container">
        {}
    </div>
</body>
</html>"#,
            template_html
        );
        
        Ok(full_html)
    }
    
    fn process_template(&self, template: &str, route: &RouteConfig) -> Result<String, StaticGenerationError> {
        let mut processed = template.to_string();
        
        for query_name in &route.ecs_queries {
            let query_result = self.execute_ecs_query(query_name)?;
            processed = processed.replace(&format!("{{{{{}}}}}", query_name), &query_result);
        }
        
        Ok(processed)
    }
    
    fn execute_ecs_query(&self, query_name: &str) -> Result<String, StaticGenerationError> {
        match query_name {
            "entity_count" => {
                let count_system = |query: Query<Entity>| query.iter().count();
                match self.ecs_context.run_system(count_system) {
                    Ok(count) => Ok(count.to_string()),
                    Err(e) => Err(StaticGenerationError::RouteError(format!("Query failed: {}", e))),
                }
            }
            _ => Ok(format!("Unknown query: {}", query_name)),
        }
    }
    
    fn copy_assets(&self, asset_dir: &Path) -> Result<(), StaticGenerationError> {
        if !asset_dir.exists() {
            return Ok(());
        }
        
        let assets_output = self.output_dir.join("assets");
        fs::create_dir_all(&assets_output)?;
        
        self.copy_dir_recursive(asset_dir, &assets_output)?;
        
        Ok(())
    }
    
    fn copy_dir_recursive(&self, src: &Path, dst: &Path) -> Result<(), StaticGenerationError> {
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            
            if src_path.is_dir() {
                fs::create_dir_all(&dst_path)?;
                self.copy_dir_recursive(&src_path, &dst_path)?;
            } else {
                fs::copy(&src_path, &dst_path)?;
            }
        }
        
        Ok(())
    }
}

pub struct AssetProcessor {
    input_dir: PathBuf,
    output_dir: PathBuf,
}

impl AssetProcessor {
    pub fn new(input_dir: impl AsRef<Path>, output_dir: impl AsRef<Path>) -> Self {
        Self {
            input_dir: input_dir.as_ref().to_path_buf(),
            output_dir: output_dir.as_ref().to_path_buf(),
        }
    }
    
    pub fn process_assets(&self) -> Result<HashMap<String, String>, StaticGenerationError> {
        let mut asset_map = HashMap::new();
        
        if !self.input_dir.exists() {
            return Ok(asset_map);
        }
        
        fs::create_dir_all(&self.output_dir)?;
        
        self.process_directory(&self.input_dir, &self.output_dir, &mut asset_map)?;
        
        Ok(asset_map)
    }
    
    fn process_directory(
        &self,
        src_dir: &Path,
        dst_dir: &Path,
        asset_map: &mut HashMap<String, String>,
    ) -> Result<(), StaticGenerationError> {
        for entry in fs::read_dir(src_dir)? {
            let entry = entry?;
            let src_path = entry.path();
            let file_name = entry.file_name();
            let dst_path = dst_dir.join(&file_name);
            
            if src_path.is_dir() {
                fs::create_dir_all(&dst_path)?;
                self.process_directory(&src_path, &dst_path, asset_map)?;
            } else {
                self.process_file(&src_path, &dst_path, asset_map)?;
            }
        }
        
        Ok(())
    }
    
    fn process_file(
        &self,
        src_path: &Path,
        dst_path: &Path,
        asset_map: &mut HashMap<String, String>,
    ) -> Result<(), StaticGenerationError> {
        fs::copy(src_path, dst_path)?;
        
        if let (Some(src_name), Some(dst_name)) = (
            src_path.file_name().and_then(|n| n.to_str()),
            dst_path.file_name().and_then(|n| n.to_str()),
        ) {
            asset_map.insert(src_name.to_string(), dst_name.to_string());
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::prelude::*;
    use tempfile::TempDir;

    #[derive(Component)]
    struct TestComponent(i32);

    #[test]
    fn test_route_config_creation() {
        let route = RouteConfig::new("/about", "about_template");
        assert_eq!(route.path, "/about");
        assert_eq!(route.template, "about_template");
        assert_eq!(route.output_file, "about.html");
    }

    #[test]
    fn test_route_config_root_path() {
        let route = RouteConfig::new("/", "home_template");
        assert_eq!(route.output_file, "index.html");
    }

    #[test]
    fn test_static_site_generator_creation() {
        let temp_dir = TempDir::new().unwrap();
        let world = World::new();
        let ecs_context = EcsWorldContext::new(world);
        
        let generator = StaticSiteGenerator::new(temp_dir.path(), ecs_context);
        assert_eq!(generator.routes.len(), 0);
        assert_eq!(generator.output_dir, temp_dir.path());
    }

    #[test]
    fn test_asset_processor_creation() {
        let temp_input = TempDir::new().unwrap();
        let temp_output = TempDir::new().unwrap();
        
        let processor = AssetProcessor::new(temp_input.path(), temp_output.path());
        assert_eq!(processor.input_dir, temp_input.path());
        assert_eq!(processor.output_dir, temp_output.path());
    }

    #[tokio::test]
    async fn test_static_generation_basic() {
        let temp_dir = TempDir::new().unwrap();
        let mut world = World::new();
        world.spawn(TestComponent(1));
        world.spawn(TestComponent(2));
        
        let ecs_context = EcsWorldContext::new(world);
        
        let route = RouteConfig::new("/test", "<h1>Test Page</h1><p>Entities: {{entity_count}}</p>")
            .with_queries(vec!["entity_count".to_string()]);
        
        let generator = StaticSiteGenerator::new(temp_dir.path(), ecs_context)
            .add_route(route);
        
        let result = generator.generate().await;
        assert!(result.is_ok());
        
        let output_file = temp_dir.path().join("test.html");
        assert!(output_file.exists());
        
        let content = fs::read_to_string(output_file).unwrap();
        assert!(content.contains("Test Page"));
        assert!(content.contains("Entities: 2"));
    }
}
