use axum::{extract::Query, response::IntoResponse, routing::get, Router};
use image::{codecs::png::PngEncoder, EncodableLayout};
use serde::Deserialize;
use xcap::{image::ImageEncoder, Monitor, Window};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let port = std::env::var("PORT")
        .unwrap_or("3000".to_string())
        .parse::<u32>()?;

    let router = Router::new().route("/capture", get(capture));

    let address = format!("0.0.0.0:{port}");
    println!("starting screen-shotting server listening at {address}");

    let listener = tokio::net::TcpListener::bind(&address).await?;
    axum::serve(listener, router).await?;

    Ok(())
}

#[derive(Deserialize)]
struct CaptureRequest {
    window_title: Option<String>,
}

async fn capture(Query(capture_request): Query<CaptureRequest>) -> impl IntoResponse {
    println!("initiating screenshot..");
    let img = match capture_request.window_title {
        Some(title) => Window::all()
            .unwrap()
            .into_iter()
            .find(|w| w.title().contains(&title))
            .unwrap()
            .capture_image()
            .unwrap(),
        None => Monitor::all().unwrap()[0].capture_image().unwrap(),
    };
    let mut img_data = Vec::new();
    PngEncoder::new(&mut img_data)
        .write_image(
            img.as_bytes(),
            img.width(),
            img.height(),
            image::ColorType::Rgba8,
        )
        .unwrap();

    println!("reponding with image data..");
    img_data
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
