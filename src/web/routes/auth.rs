use askama::Template;
use axum::{
    http::header,
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use cookie::Cookie;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::error;

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate;

#[derive(Deserialize)]
pub struct LoginForm {
    email: String,
    password: String,
}

#[derive(Deserialize, Serialize)]
struct AuthResponse {
    access_token: String,
    refresh_token: String,
    #[serde(default)]
    mfa_required: bool,
}

#[derive(Deserialize)]
struct AuthServiceResponse {
    #[serde(rename = "success")]
    _success: bool,
    data: AuthResponse,
}

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorTemplate {
    message: String,
}

pub async fn login_page() -> Html<String> {
    let template = LoginTemplate;
    Html(template.render().unwrap())
}

pub async fn login_handler(Form(form): Form<LoginForm>) -> Result<Response, Html<String>> {
    println!("📝 LOGIN ATTEMPT: email={}", form.email);

    // HTTP client maken
    let client = reqwest::Client::new();

    // POST request naar auth-service
    println!("🔐 Sending auth request to http://auth.localhost:8080/api/v1/auth/login");
    let response = client
        .post("http://auth.localhost:8080/api/v1/auth/login")
        .json(&json!({
            "email": form.email,
            "password": form.password,
        }))
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            println!("✅ Got response from auth-service: status={}", status);

            if resp.status().is_success() {
                // Read body as text first for debugging
                let body_text = resp.text().await.unwrap_or_default();
                println!("📦 Raw auth response: {}", body_text);

                // Parse tokens from text
                println!("📦 Parsing auth response...");
                let auth_resp: AuthResponse =
                    match serde_json::from_str::<AuthServiceResponse>(&body_text) {
                        Ok(wrapper) => {
                            println!("✅ Successfully parsed response!");
                            wrapper.data
                        }
                        Err(e) => {
                            println!("❌ PARSE ERROR: {}", e);
                            error!("Kan auth response niet parsen: {}", e);
                            let template = ErrorTemplate {
                                message: format!("Parse error: {}", e),
                            };
                            return Err(Html(template.render().unwrap()));
                        }
                    };

                // Maak cookies
                println!("🍪 Creating cookies...");
                let mut access_cookie = Cookie::new("access_token", auth_resp.access_token.clone());
                access_cookie.set_path("/");
                access_cookie.set_http_only(true);
                access_cookie.set_same_site(cookie::SameSite::Lax);

                let mut refresh_cookie = Cookie::new("refresh_token", auth_resp.refresh_token);
                refresh_cookie.set_path("/");
                refresh_cookie.set_http_only(true);
                refresh_cookie.set_same_site(cookie::SameSite::Lax);

                // Build response with cookies
                println!("➡️  Redirecting to /activities...");
                let mut response = Redirect::to("/activities").into_response();
                response.headers_mut().append(
                    header::SET_COOKIE,
                    access_cookie.to_string().parse().unwrap(),
                );
                response.headers_mut().append(
                    header::SET_COOKIE,
                    refresh_cookie.to_string().parse().unwrap(),
                );

                println!("✅ LOGIN SUCCESS!");
                Ok(response)
            } else {
                println!("❌ Auth service returned error status: {}", status);
                error!("Auth service error: {}", status);
                let template = ErrorTemplate {
                    message: format!("Login failed: {}", status),
                };
                Err(Html(template.render().unwrap()))
            }
        }
        Err(e) => {
            println!("❌ CONNECTION ERROR: {}", e);
            error!("Request naar auth-service failed: {}", e);
            let template = ErrorTemplate {
                message: format!("Connection error: {}", e),
            };
            Err(Html(template.render().unwrap()))
        }
    }
}

pub async fn logout_handler() -> Response {
    // Clear cookies
    let mut access_cookie = Cookie::new("access_token", "");
    access_cookie.set_path("/");
    access_cookie.set_http_only(true);
    access_cookie.set_same_site(cookie::SameSite::Lax);
    access_cookie.set_max_age(None);

    let mut refresh_cookie = Cookie::new("refresh_token", "");
    refresh_cookie.set_path("/");
    refresh_cookie.set_http_only(true);
    refresh_cookie.set_same_site(cookie::SameSite::Lax);
    refresh_cookie.set_max_age(None);

    let mut response = Redirect::to("/login").into_response();
    response.headers_mut().append(
        header::SET_COOKIE,
        access_cookie.to_string().parse().unwrap(),
    );
    response.headers_mut().append(
        header::SET_COOKIE,
        refresh_cookie.to_string().parse().unwrap(),
    );

    response
}
