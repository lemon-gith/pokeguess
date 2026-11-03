//! The API end to end against a real Postgres, given as `TEST_DATABASE_URL`
//! (CI provides one). Each test gets its own schema, so they can run in
//! parallel.

use std::str::FromStr;
use std::sync::atomic::{AtomicU32, Ordering};

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::postgres::{PgConnectOptions, PgConnection};
use sqlx::{ConnectOptions, Connection};
use tower::ServiceExt;
use user::{AppState, Config, WEB_ORIGIN, allowed_origins, router};

struct Api {
    app: Router,
    state: AppState,
}

struct Reply {
    status: StatusCode,
    headers: HeaderMap,
    body: Value,
}

impl Reply {
    /// The session token this reply sets, if any.
    fn cookie(&self) -> Option<String> {
        let set = self.headers.get(header::SET_COOKIE)?.to_str().ok()?;
        let token = set.strip_prefix("__Host-session=")?.split(';').next()?;
        (!token.is_empty()).then(|| token.to_string())
    }

    fn error(&self) -> &str {
        self.body["error"].as_str().unwrap_or_default()
    }
}

/// A fresh API with its own schema, or `None` (skipping the test) without a
/// database to test against.
async fn api() -> Option<Api> {
    let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
        // CI always has one, so there a missing one is a mistake, not a skip
        assert!(
            std::env::var_os("CI").is_none(),
            "TEST_DATABASE_URL isn't set in CI"
        );
        eprintln!("skipping: TEST_DATABASE_URL isn't set");
        return None;
    };
    static COUNT: AtomicU32 = AtomicU32::new(0);
    let schema = format!(
        "test_{}_{}_{}",
        std::process::id(),
        COUNT.fetch_add(1, Ordering::Relaxed),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let options = PgConnectOptions::from_str(&url).expect("TEST_DATABASE_URL");
    let mut admin: PgConnection = options.connect().await.expect("connect");
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&mut admin)
        .await
        .unwrap();
    admin.close().await.unwrap();

    let options = options.options([("search_path", schema.as_str())]);
    let state = AppState::new(Config {
        database: options.clone(),
        migrations: options,
        origins: allowed_origins(false),
    });
    Some(Api {
        app: router(state.clone()),
        state,
    })
}

impl Api {
    async fn send(&self, request: Request<Body>) -> Reply {
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        Reply {
            status,
            headers,
            body,
        }
    }

    /// A request from the web app, signed in with `cookie` if given.
    async fn call(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
        cookie: Option<&str>,
    ) -> Reply {
        let mut request = Request::builder()
            .method(method)
            .uri(path)
            .header(header::ORIGIN, WEB_ORIGIN);
        if let Some(token) = cookie {
            request = request.header(
                header::COOKIE,
                format!("theme=dark; __Host-session={token}"),
            );
        }
        let request = match body {
            Some(body) => request
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string())),
            None => request.body(Body::empty()),
        };
        self.send(request.unwrap()).await
    }

    async fn signup(&self, username: &str, password: &str) -> Reply {
        let body = json!({ "username": username, "password": password });
        self.call(Method::POST, "/api/signup", Some(body), None)
            .await
    }

    async fn login(&self, username: &str, password: &str) -> Reply {
        let body = json!({ "username": username, "password": password });
        self.call(Method::POST, "/api/login", Some(body), None)
            .await
    }

    async fn me(&self, cookie: &str) -> Reply {
        self.call(Method::GET, "/api/me", None, Some(cookie)).await
    }

    async fn sql(&self, query: &str) {
        sqlx::query(query).execute(self.state.pool()).await.unwrap();
    }
}

#[tokio::test]
async fn health_needs_no_database() {
    let state = AppState::new(Config {
        // nothing listens here
        database: PgConnectOptions::from_str("postgres://nobody@127.0.0.1:1/none").unwrap(),
        migrations: PgConnectOptions::from_str("postgres://nobody@127.0.0.1:1/none").unwrap(),
        origins: allowed_origins(true),
    });
    let api = Api {
        app: router(state.clone()),
        state,
    };
    let reply = api.call(Method::GET, "/api/health", None, None).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body["status"], "ok");
    assert_eq!(reply.headers[header::CACHE_CONTROL], "no-store");
}

#[tokio::test]
async fn signing_up_signs_in() {
    let Some(api) = api().await else { return };
    let reply = api.signup("Ash_K", "pikachu123").await;
    assert_eq!(reply.status, StatusCode::CREATED);
    assert_eq!(reply.body["username"], "Ash_K");
    assert!(reply.body["created_at"].as_f64().unwrap() > 1.7e12);
    let set_cookie = reply.headers[header::SET_COOKIE].to_str().unwrap();
    for attribute in [
        "Path=/",
        "Max-Age=2592000",
        "Secure",
        "HttpOnly",
        "SameSite=Lax",
    ] {
        assert!(set_cookie.contains(attribute), "{set_cookie}");
    }
    assert!(
        !set_cookie.contains("Domain"),
        "a __Host- cookie has no Domain"
    );

    let me = api.me(&reply.cookie().unwrap()).await;
    assert_eq!(me.status, StatusCode::OK);
    assert_eq!(me.body["username"], "Ash_K");
    // only renewed once it's a day old
    assert!(me.headers.get(header::SET_COOKIE).is_none());
}

#[tokio::test]
async fn usernames_are_unique_regardless_of_case() {
    let Some(api) = api().await else { return };
    assert_eq!(
        api.signup("misty", "staryu123").await.status,
        StatusCode::CREATED
    );
    let reply = api.signup("MISTY", "psyduck123").await;
    assert_eq!(reply.status, StatusCode::CONFLICT);
    assert_eq!(reply.error(), "username_taken");
}

#[tokio::test]
async fn signup_checks_the_username_and_password() {
    let Some(api) = api().await else { return };
    let reply = api.signup("no spaces", "pikachu123").await;
    assert_eq!(
        (reply.status, reply.error()),
        (StatusCode::BAD_REQUEST, "invalid_username")
    );
    let reply = api.signup("brock", "short").await;
    assert_eq!(
        (reply.status, reply.error()),
        (StatusCode::BAD_REQUEST, "invalid_password")
    );
    assert!(reply.body["message"].as_str().unwrap().contains('8'));
}

#[tokio::test]
async fn logging_in() {
    let Some(api) = api().await else { return };
    api.signup("Gary", "eevee12345").await;

    for (username, password) in [("Gary", "wrong-password"), ("nobody", "eevee12345")] {
        let reply = api.login(username, password).await;
        assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
        assert_eq!(reply.error(), "wrong_credentials", "the same either way");
        assert!(reply.cookie().is_none());
    }

    let reply = api.login("gARY", "eevee12345").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(
        reply.body["username"], "Gary",
        "as it was typed when signing up"
    );
    assert_eq!(
        api.me(&reply.cookie().unwrap()).await.status,
        StatusCode::OK
    );
}

#[tokio::test]
async fn logging_out_ends_the_session() {
    let Some(api) = api().await else { return };
    let token = api.signup("oak", "professor1").await.cookie().unwrap();
    let reply = api
        .call(Method::POST, "/api/logout", None, Some(&token))
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    assert!(
        reply.headers[header::SET_COOKIE]
            .to_str()
            .unwrap()
            .contains("Max-Age=0")
    );
    let reply = api.me(&token).await;
    assert_eq!(
        (reply.status, reply.error()),
        (StatusCode::UNAUTHORIZED, "unauthenticated")
    );
    // logging out again, or without a session, is fine
    let reply = api
        .call(Method::POST, "/api/logout", None, Some(&token))
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn sessions_are_stored_only_as_hashes() {
    let Some(api) = api().await else { return };
    let token = api.signup("jessie", "meowth1234").await.cookie().unwrap();
    let (stored,): (Vec<u8>,) = sqlx::query_as("SELECT token_hash FROM sessions")
        .fetch_one(api.state.pool())
        .await
        .unwrap();
    assert_eq!(stored.len(), 32);
    assert_ne!(stored, token.as_bytes());
    let (hash,): (String,) = sqlx::query_as("SELECT password_hash FROM users")
        .fetch_one(api.state.pool())
        .await
        .unwrap();
    assert!(hash.starts_with("$argon2id$"));
}

#[tokio::test]
async fn expired_sessions_are_refused_and_old_ones_renewed() {
    let Some(api) = api().await else { return };
    let token = api.signup("james", "wobbuffet1").await.cookie().unwrap();

    // two days in: renewed
    api.sql("UPDATE sessions SET expires_at = now() + interval '28 days'")
        .await;
    let reply = api.me(&token).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(
        reply.cookie().as_deref(),
        Some(token.as_str()),
        "same token, longer life"
    );
    let (days,): (f64,) = sqlx::query_as(
        "SELECT EXTRACT(EPOCH FROM expires_at - now())::float8 / 86400 FROM sessions",
    )
    .fetch_one(api.state.pool())
    .await
    .unwrap();
    assert!(days > 29.9, "{days}");

    api.sql("UPDATE sessions SET expires_at = now() - interval '1 second'")
        .await;
    assert_eq!(api.me(&token).await.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn changes_need_the_web_apps_origin() {
    let Some(api) = api().await else { return };
    let body = json!({ "username": "rocket", "password": "team-rocket" }).to_string();
    for origin in [
        None,
        Some("https://evil.example"),
        Some("https://flamingo.piercew.xyz"),
    ] {
        let mut request =
            Request::post("/api/signup").header(header::CONTENT_TYPE, "application/json");
        if let Some(origin) = origin {
            request = request.header(header::ORIGIN, origin);
        }
        let reply = api
            .send(request.body(Body::from(body.clone())).unwrap())
            .await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN, "{origin:?}");
        assert_eq!(reply.error(), "forbidden_origin");
    }
    // the local dev server is allowed outside production
    let request = Request::post("/api/signup")
        .header(header::ORIGIN, "http://localhost:8000")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();
    assert_eq!(api.send(request).await.status, StatusCode::CREATED);
}

#[tokio::test]
async fn cors_allows_only_the_web_app_with_credentials() {
    let Some(api) = api().await else { return };
    let preflight = |origin: &str| {
        Request::builder()
            .method(Method::OPTIONS)
            .uri("/api/login")
            .header(header::ORIGIN, origin)
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
            .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "content-type")
            .body(Body::empty())
            .unwrap()
    };
    let reply = api.send(preflight(WEB_ORIGIN)).await;
    assert!(reply.status.is_success());
    assert_eq!(
        reply.headers[header::ACCESS_CONTROL_ALLOW_ORIGIN],
        WEB_ORIGIN
    );
    assert_eq!(
        reply.headers[header::ACCESS_CONTROL_ALLOW_CREDENTIALS],
        "true"
    );

    let reply = api.send(preflight("https://evil.example")).await;
    assert!(
        reply
            .headers
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .is_none()
    );

    // and production doesn't allow localhost
    assert!(
        !allowed_origins(true)
            .iter()
            .any(|o| o.contains("localhost"))
    );
}

#[tokio::test]
async fn too_many_wrong_passwords_are_rate_limited() {
    let Some(api) = api().await else { return };
    api.signup("brock", "onix123456").await;
    for _ in 0..10 {
        assert_eq!(
            api.login("brock", "wrong-guess").await.status,
            StatusCode::UNAUTHORIZED
        );
    }
    // even the right password, until the window passes
    let reply = api.login("BROCK", "onix123456").await;
    assert_eq!(
        (reply.status, reply.error()),
        (StatusCode::TOO_MANY_REQUESTS, "rate_limited")
    );
    let retry_after: u64 = reply.headers[header::RETRY_AFTER]
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!((1..=900).contains(&retry_after), "{retry_after}");

    api.sql("UPDATE attempts SET at = at - interval '16 minutes'")
        .await;
    assert_eq!(
        api.login("brock", "onix123456").await.status,
        StatusCode::OK
    );
}

#[tokio::test]
async fn changing_the_password_signs_out_other_sessions() {
    let Some(api) = api().await else { return };
    let here = api.signup("may", "torchic123").await.cookie().unwrap();
    let elsewhere = api.login("may", "torchic123").await.cookie().unwrap();

    let change =
        |current: &str, new: &str| json!({ "current_password": current, "new_password": new });
    let reply = api
        .call(
            Method::POST,
            "/api/password",
            Some(change("wrong-one", "blaziken12")),
            Some(&here),
        )
        .await;
    assert_eq!(
        (reply.status, reply.error()),
        (StatusCode::FORBIDDEN, "wrong_credentials")
    );
    let reply = api
        .call(
            Method::POST,
            "/api/password",
            Some(change("torchic123", "short")),
            Some(&here),
        )
        .await;
    assert_eq!(reply.error(), "invalid_password");

    let reply = api
        .call(
            Method::POST,
            "/api/password",
            Some(change("torchic123", "blaziken12")),
            Some(&here),
        )
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    assert_eq!(api.me(&here).await.status, StatusCode::OK);
    assert_eq!(api.me(&elsewhere).await.status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        api.login("may", "torchic123").await.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(api.login("may", "blaziken12").await.status, StatusCode::OK);
}

#[tokio::test]
async fn deleting_the_account() {
    let Some(api) = api().await else { return };
    let token = api.signup("dawn", "piplup1234").await.cookie().unwrap();
    api.signup("other", "someone-else").await;

    let reply = api
        .call(
            Method::DELETE,
            "/api/me",
            Some(json!({ "password": "nope-nope" })),
            Some(&token),
        )
        .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    assert_eq!(api.me(&token).await.status, StatusCode::OK);

    let reply = api
        .call(
            Method::DELETE,
            "/api/me",
            Some(json!({ "password": "piplup1234" })),
            Some(&token),
        )
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    assert!(
        reply.headers[header::SET_COOKIE]
            .to_str()
            .unwrap()
            .contains("Max-Age=0")
    );
    assert_eq!(api.me(&token).await.status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        api.login("dawn", "piplup1234").await.status,
        StatusCode::UNAUTHORIZED
    );
    let (users, sessions): (i64, i64) =
        sqlx::query_as("SELECT (SELECT count(*) FROM users), (SELECT count(*) FROM sessions)")
            .fetch_one(api.state.pool())
            .await
            .unwrap();
    assert_eq!((users, sessions), (1, 1), "only the other account is left");
    // and the username is free again
    assert_eq!(
        api.signup("DAWN", "piplup1234").await.status,
        StatusCode::CREATED
    );
}

#[tokio::test]
async fn every_error_is_json() {
    let Some(api) = api().await else { return };
    let reply = api.call(Method::GET, "/api/nothing-here", None, None).await;
    assert_eq!(
        (reply.status, reply.error()),
        (StatusCode::NOT_FOUND, "not_found")
    );

    let request = Request::post("/api/login")
        .header(header::ORIGIN, WEB_ORIGIN)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{not json"))
        .unwrap();
    let reply = api.send(request).await;
    assert_eq!(
        (reply.status, reply.error()),
        (StatusCode::BAD_REQUEST, "bad_request")
    );

    // JSON only, which also means a cross-site form can't post here
    let request = Request::post("/api/login")
        .header(header::ORIGIN, WEB_ORIGIN)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from("username=a&password=b"))
        .unwrap();
    let reply = api.send(request).await;
    assert_eq!(
        (reply.status, reply.error()),
        (StatusCode::UNSUPPORTED_MEDIA_TYPE, "bad_request")
    );

    assert_eq!(
        api.call(Method::GET, "/api/me", None, None).await.error(),
        "unauthenticated"
    );
}
