mod ics;
mod plan;

use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use chrono::Local;
use plan::{generate_plan, AthleteInput, TrainingPlan};
use tower_http::{cors::CorsLayer, services::ServeDir};

async fn generate_handler(Json(input): Json<AthleteInput>) -> Json<TrainingPlan> {
    let today = Local::now().date_naive();
    Json(generate_plan(&input, today))
}

async fn export_ics_handler(Json(input): Json<AthleteInput>) -> Response {
    let today = Local::now().date_naive();
    let generated = generate_plan(&input, today);
    let ics_body = ics::generate_ics(&generated);

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/calendar; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"50k-training-plan.ics\"",
            ),
        ],
        ics_body,
    )
        .into_response()
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/api/generate", post(generate_handler))
        .route("/api/export-ics", post(export_ics_handler))
        .fallback_service(ServeDir::new("static"))
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind to port 3000");

    println!("Ultra Planner running at http://localhost:3000");
    axum::serve(listener, app).await.expect("server error");
}
