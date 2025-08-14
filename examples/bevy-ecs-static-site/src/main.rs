use blitz_bevy_ecs::prelude::*;
use bevy_ecs::prelude::*;
use std::path::Path;

#[derive(Resource, Clone, PartialEq)]
struct SiteName(String);

#[derive(Resource, Clone, PartialEq)]
struct SiteDescription(String);

#[derive(Component)]
struct BlogPost {
    title: String,
    content: String,
    slug: String,
}

#[derive(Component)]
struct Author {
    name: String,
    bio: String,
}

fn setup_world() -> World {
    let mut world = World::new();
    
    world.insert_resource(SiteName("My ECS Blog".to_string()));
    world.insert_resource(SiteDescription("A blog powered by Bevy ECS and Blitz".to_string()));
    
    world.spawn((
        BlogPost {
            title: "Welcome to ECS Blogging".to_string(),
            content: "This is our first post using Bevy ECS for content management!".to_string(),
            slug: "welcome-to-ecs-blogging".to_string(),
        },
        Author {
            name: "ECS Developer".to_string(),
            bio: "Passionate about Entity Component Systems".to_string(),
        },
    ));
    
    world.spawn((
        BlogPost {
            title: "Building Static Sites with Blitz".to_string(),
            content: "Learn how to generate static sites using Blitz rendering engine.".to_string(),
            slug: "building-static-sites-with-blitz".to_string(),
        },
        Author {
            name: "Blitz Contributor".to_string(),
            bio: "Contributing to the future of web rendering".to_string(),
        },
    ));
    
    world
}

fn create_routes() -> Router {
    Router::new()
        .add_route(
            Route::new("/", "HomePage")
                .with_title("Home - My ECS Blog")
                .with_meta("description", "Welcome to our ECS-powered blog")
                .with_ecs_dependency("SiteName")
                .with_ecs_dependency("SiteDescription")
        )
        .add_route(
            Route::new("/about", "AboutPage")
                .with_title("About - My ECS Blog")
                .with_meta("description", "Learn about our ECS blog")
        )
        .add_route(
            Route::new("/blog", "BlogListPage")
                .with_title("Blog Posts - My ECS Blog")
                .with_meta("description", "All our blog posts")
                .with_ecs_dependency("BlogPost")
        )
        .with_fallback(
            Route::new("/404", "NotFoundPage")
                .with_title("Page Not Found")
        )
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting ECS Static Site Generation...");
    
    let world = setup_world();
    let ecs_context = EcsWorldContext::new(world);
    let router = create_routes();
    
    let output_dir = Path::new("./dist");
    
    let home_route = RouteConfig::new("/", r#"
        <div class="hero">
            <h1>{{site_name}}</h1>
            <p>{{site_description}}</p>
            <p>Total entities in our system: {{entity_count}}</p>
        </div>
        <nav>
            <a href="/blog">Read Blog Posts</a> |
            <a href="/about">About Us</a>
        </nav>
    "#).with_queries(vec![
        "site_name".to_string(),
        "site_description".to_string(),
        "entity_count".to_string(),
    ]);
    
    let about_route = RouteConfig::new("/about", r#"
        <div class="about">
            <h1>About Our ECS Blog</h1>
            <p>This blog demonstrates the power of combining:</p>
            <ul>
                <li><strong>Bevy ECS</strong> - For data management and queries</li>
                <li><strong>Dioxus</strong> - For reactive UI components</li>
                <li><strong>Blitz</strong> - For high-performance rendering</li>
            </ul>
            <p>Total system entities: {{entity_count}}</p>
            <a href="/">← Back to Home</a>
        </div>
    "#).with_queries(vec!["entity_count".to_string()]);
    
    let blog_route = RouteConfig::new("/blog", r#"
        <div class="blog">
            <h1>Blog Posts</h1>
            <p>We have {{entity_count}} entities in our content system.</p>
            <div class="posts">
                <article>
                    <h2>Welcome to ECS Blogging</h2>
                    <p>This is our first post using Bevy ECS for content management!</p>
                    <small>By: ECS Developer</small>
                </article>
                <article>
                    <h2>Building Static Sites with Blitz</h2>
                    <p>Learn how to generate static sites using Blitz rendering engine.</p>
                    <small>By: Blitz Contributor</small>
                </article>
            </div>
            <a href="/">← Back to Home</a>
        </div>
    "#).with_queries(vec!["entity_count".to_string()]);
    
    let generator = StaticSiteGenerator::new(output_dir, ecs_context)
        .add_route(home_route)
        .add_route(about_route)
        .add_route(blog_route);
    
    println!("📝 Generating static site...");
    generator.generate().await?;
    
    let sitemap = SiteMap::from_router(&router, "https://example.com");
    sitemap.save_to_file(&output_dir.join("sitemap.xml"))?;
    
    println!("✅ Static site generated successfully!");
    println!("📁 Output directory: {}", output_dir.display());
    println!("🗺️  Sitemap created: {}/sitemap.xml", output_dir.display());
    
    let routes = router.generate_route_list();
    println!("🔗 Generated routes:");
    for route in routes {
        println!("   - {}", route);
    }
    
    Ok(())
}
