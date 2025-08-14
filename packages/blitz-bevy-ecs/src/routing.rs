use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RoutingError {
    #[error("Route not found: {0}")]
    RouteNotFound(String),
    #[error("Invalid route pattern: {0}")]
    InvalidPattern(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub path: String,
    pub component: String,
    pub title: Option<String>,
    pub meta: HashMap<String, String>,
    pub ecs_dependencies: Vec<String>,
}

impl Route {
    pub fn new(path: impl Into<String>, component: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            component: component.into(),
            title: None,
            meta: HashMap::new(),
            ecs_dependencies: Vec::new(),
        }
    }
    
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
    
    pub fn with_meta(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.meta.insert(key.into(), value.into());
        self
    }
    
    pub fn with_ecs_dependency(mut self, dependency: impl Into<String>) -> Self {
        self.ecs_dependencies.push(dependency.into());
        self
    }
    
    pub fn matches(&self, path: &str) -> bool {
        if self.path.contains(':') {
            self.matches_dynamic(path)
        } else {
            self.path == path
        }
    }
    
    fn matches_dynamic(&self, path: &str) -> bool {
        let route_segments: Vec<&str> = self.path.split('/').collect();
        let path_segments: Vec<&str> = path.split('/').collect();
        
        if route_segments.len() != path_segments.len() {
            return false;
        }
        
        for (route_seg, path_seg) in route_segments.iter().zip(path_segments.iter()) {
            if route_seg.starts_with(':') {
                continue;
            }
            if route_seg != path_seg {
                return false;
            }
        }
        
        true
    }
    
    pub fn extract_params(&self, path: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();
        
        if !self.path.contains(':') {
            return params;
        }
        
        let route_segments: Vec<&str> = self.path.split('/').collect();
        let path_segments: Vec<&str> = path.split('/').collect();
        
        for (route_seg, path_seg) in route_segments.iter().zip(path_segments.iter()) {
            if let Some(param_name) = route_seg.strip_prefix(':') {
                params.insert(param_name.to_string(), path_seg.to_string());
            }
        }
        
        params
    }
}

#[derive(Debug)]
pub struct Router {
    routes: Vec<Route>,
    fallback_route: Option<Route>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            fallback_route: None,
        }
    }
    
    pub fn add_route(mut self, route: Route) -> Self {
        self.routes.push(route);
        self
    }
    
    pub fn with_fallback(mut self, route: Route) -> Self {
        self.fallback_route = Some(route);
        self
    }
    
    pub fn resolve(&self, path: &str) -> Result<&Route, RoutingError> {
        for route in &self.routes {
            if route.matches(path) {
                return Ok(route);
            }
        }
        
        if let Some(ref fallback) = self.fallback_route {
            Ok(fallback)
        } else {
            Err(RoutingError::RouteNotFound(path.to_string()))
        }
    }
    
    pub fn get_all_routes(&self) -> &[Route] {
        &self.routes
    }
    
    pub fn get_static_routes(&self) -> Vec<&Route> {
        self.routes
            .iter()
            .filter(|route| !route.path.contains(':'))
            .collect()
    }
    
    pub fn generate_route_list(&self) -> Vec<String> {
        self.get_static_routes()
            .iter()
            .map(|route| route.path.clone())
            .collect()
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SiteMap {
    pub routes: Vec<Route>,
    pub base_url: String,
    pub last_modified: String,
}

impl SiteMap {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            routes: Vec::new(),
            base_url: base_url.into(),
            last_modified: chrono::Utc::now().to_rfc3339(),
        }
    }
    
    pub fn from_router(router: &Router, base_url: impl Into<String>) -> Self {
        let mut sitemap = Self::new(base_url);
        sitemap.routes = router.get_static_routes().into_iter().cloned().collect();
        sitemap
    }
    
    pub fn to_xml(&self) -> String {
        let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
"#);
        
        for route in &self.routes {
            xml.push_str(&format!(
                r#"  <url>
    <loc>{}{}</loc>
    <lastmod>{}</lastmod>
    <changefreq>weekly</changefreq>
    <priority>0.8</priority>
  </url>
"#,
                self.base_url.trim_end_matches('/'),
                route.path,
                self.last_modified
            ));
        }
        
        xml.push_str("</urlset>\n");
        xml
    }
    
    pub fn save_to_file(&self, path: &PathBuf) -> Result<(), RoutingError> {
        let xml_content = self.to_xml();
        std::fs::write(path, xml_content)
            .map_err(|e| RoutingError::InvalidPattern(format!("Failed to write sitemap: {}", e)))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_creation() {
        let route = Route::new("/about", "AboutPage")
            .with_title("About Us")
            .with_meta("description", "About our company")
            .with_ecs_dependency("UserResource");
        
        assert_eq!(route.path, "/about");
        assert_eq!(route.component, "AboutPage");
        assert_eq!(route.title, Some("About Us".to_string()));
        assert_eq!(route.meta.get("description"), Some(&"About our company".to_string()));
        assert_eq!(route.ecs_dependencies, vec!["UserResource"]);
    }

    #[test]
    fn test_static_route_matching() {
        let route = Route::new("/about", "AboutPage");
        assert!(route.matches("/about"));
        assert!(!route.matches("/contact"));
    }

    #[test]
    fn test_dynamic_route_matching() {
        let route = Route::new("/user/:id", "UserPage");
        assert!(route.matches("/user/123"));
        assert!(route.matches("/user/abc"));
        assert!(!route.matches("/user/123/profile"));
        assert!(!route.matches("/admin/123"));
    }

    #[test]
    fn test_param_extraction() {
        let route = Route::new("/user/:id/post/:slug", "PostPage");
        let params = route.extract_params("/user/123/post/hello-world");
        
        assert_eq!(params.get("id"), Some(&"123".to_string()));
        assert_eq!(params.get("slug"), Some(&"hello-world".to_string()));
    }

    #[test]
    fn test_router_resolution() {
        let router = Router::new()
            .add_route(Route::new("/", "HomePage"))
            .add_route(Route::new("/about", "AboutPage"))
            .add_route(Route::new("/user/:id", "UserPage"))
            .with_fallback(Route::new("/404", "NotFoundPage"));
        
        assert_eq!(router.resolve("/").unwrap().component, "HomePage");
        assert_eq!(router.resolve("/about").unwrap().component, "AboutPage");
        assert_eq!(router.resolve("/user/123").unwrap().component, "UserPage");
        assert_eq!(router.resolve("/nonexistent").unwrap().component, "NotFoundPage");
    }

    #[test]
    fn test_static_routes_filtering() {
        let router = Router::new()
            .add_route(Route::new("/", "HomePage"))
            .add_route(Route::new("/about", "AboutPage"))
            .add_route(Route::new("/user/:id", "UserPage"));
        
        let static_routes = router.get_static_routes();
        assert_eq!(static_routes.len(), 2);
        assert!(static_routes.iter().any(|r| r.path == "/"));
        assert!(static_routes.iter().any(|r| r.path == "/about"));
        assert!(!static_routes.iter().any(|r| r.path == "/user/:id"));
    }

    #[test]
    fn test_sitemap_generation() {
        let router = Router::new()
            .add_route(Route::new("/", "HomePage"))
            .add_route(Route::new("/about", "AboutPage"));
        
        let sitemap = SiteMap::from_router(&router, "https://example.com");
        let xml = sitemap.to_xml();
        
        assert!(xml.contains("https://example.com/"));
        assert!(xml.contains("https://example.com/about"));
        assert!(xml.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    }
}
