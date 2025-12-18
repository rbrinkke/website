use axum::{
    middleware,
    response::Redirect,
    routing::{get, get_service, post},
    Router,
};
use dotenvy::dotenv;
use http::header::{HeaderValue, CACHE_CONTROL};
use sqlx::sqlite::SqlitePoolOptions;
use std::env;
use std::net::SocketAddr;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;

use website::web::middleware::auth as auth_middleware;
use website::web::routes::{
    activities, activity, auth, chat_api, chats, discovery, images, location, user,
};

#[tokio::main]
async fn main() {
    // Laad .env bestand
    dotenv().ok();

    // 1. Start logging
    tracing_subscriber::fmt::init();

    // 2. Verbind met de Database
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL moet in .env staan");
    println!("Verbinden met database: {}", db_url);

    let pool = SqlitePoolOptions::new()
        .connect(&db_url)
        .await
        .expect("Kan niet verbinden met DB");

    // 3. Protected routes onder één middleware layer
    let protected_routes = Router::new()
        .route("/discovery", get(discovery::discovery_handler))
        .route("/activities", get(activities::activities_handler))
        .route("/chats", get(chats::chats_handler))
        .route("/chats/:conversation_id", get(chats::chat_detail_handler))
        .route("/api/chat/health", get(chat_api::health_handler))
        .route(
            "/api/chat/resolve-conversation",
            get(chat_api::resolve_conversation_handler),
        )
        .route("/api/chat/ws-ticket", post(chat_api::ws_ticket_handler))
        .route(
            "/api/chat/conversations/:conversation_id/messages",
            get(chat_api::list_messages_handler).post(chat_api::send_message_handler),
        )
        .route(
            "/activities/:activity_id",
            get(activity::activity_detail_handler),
        )
        .route(
            "/activities/:activity_id/summary",
            get(activity::activity_summary_handler),
        )
        .route(
            "/activities/:activity_id/signup",
            post(activity::activity_signup_command_handler),
        )
        .route(
            "/activities/:activity_id/waitlist",
            post(activity::activity_waitlist_command_handler),
        )
        .route("/users/:user_id", get(user::user_profile_handler))
        .route("/users/:user_id/summary", get(user::user_summary_handler))
        .route(
            "/users/:user_id/friendship",
            post(user::friendship_command_handler),
        )
        .route("/images/:image_id", get(images::image_proxy))
        .route("/api/location/search", get(location::search_locations))
        .route("/logout", post(auth::logout_handler))
        .layer(middleware::from_fn_with_state(
            pool.clone(),
            auth_middleware::require_auth,
        ));

    // 4. Bouw de hele applicatie
    let app = Router::new()
        // Public routes
        .route("/", get(|| async { Redirect::to("/activities") }))
        .route("/login", get(auth::login_page).post(auth::login_handler))
        // Protected routes
        .merge(protected_routes)
        // Static files
        .nest_service(
            "/assets",
            get_service(ServeDir::new("assets")).layer(SetResponseHeaderLayer::if_not_present(
                CACHE_CONTROL,
                HeaderValue::from_static("no-store"),
            )),
        )
        // Layers
        .layer(SetResponseHeaderLayer::if_not_present(
            CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ))
        .layer(CatchPanicLayer::new())
        // State
        .with_state(pool);

    // 4. Start de server (met fallback poort)
    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3000);
    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .expect("Kan host/port niet parsen");

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!(
                "⚠️  Kon niet binden op {}: {}. Probeer fallback {}:{}",
                addr,
                e,
                host,
                port + 1
            );
            let fallback: SocketAddr = format!("{}:{}", host, port + 1)
                .parse()
                .expect("Kan fallback niet parsen");
            tokio::net::TcpListener::bind(fallback)
                .await
                .expect("Kan niet binden op fallback poort")
        }
    };

    let bound_addr = listener.local_addr().unwrap();
    println!("🚀 Server draait op http://{}", bound_addr);
    println!("📍 Ga naar http://{}/login om te beginnen", bound_addr);

    axum::serve(listener, app).await.unwrap();
}
