use axum::{
    body::Bytes, extract::DefaultBodyLimit, response::IntoResponse, routing::post, Json, Router,
};
use image::EncodableLayout;
use ocr::OCRWord;
use regex::Regex;
use tesseract::Tesseract;

type Res<T> = Result<T, Box<dyn std::error::Error>>;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Res<()> {
    let port = std::env::var("PORT")
        .unwrap_or("3000".to_string())
        .parse::<u32>()?;

    let router = Router::new()
        .route("/ocr", post(ocr))
        .layer(DefaultBodyLimit::max(10000000000));

    let address = format!("0.0.0.0:{port}");
    println!("started ocr server listening at {address}");
    let listener = tokio::net::TcpListener::bind(&address).await?;
    axum::serve(listener, router).await?;

    Ok(())
}

async fn ocr(body: Bytes) -> impl IntoResponse {
    println!("handling ocr job request..");

    let mut ocr = get_ocr().unwrap();
    #[cfg(debug_assertions)]
    {
        use std::io::Write;
        std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open("./screen-shot.png")
            .unwrap()
            .write_all(&body.as_bytes())
            .unwrap();
    }
    ocr = ocr.set_image_from_mem(body.as_bytes()).unwrap();
    // reads an XML document containing the OCR information
    let ocr_words: Vec<OCRWord> = ocr
        .get_hocr_text(0)
        .unwrap()
        .lines()
        .flat_map(parse_box)
        .collect();

    Json(ocr_words)
}

fn get_ocr() -> Res<Tesseract> {
    let mut tess = tesseract::Tesseract::new(None, Some("eng"))?
        .set_variable("tessedit_char_whitelist", "0123456789")?;
    tess.set_page_seg_mode(tesseract::PageSegMode::PsmSingleColumn);
    Ok(tess)
}

fn parse_box(line: &str) -> Res<OCRWord> {
    let reg = Regex::new(r"bbox (\d+) (\d+) (\d+) (\d+); x_wconf (\d+).+>(.*)</span>").unwrap();
    for (_, [x1, y1, x2, y2, confidence, text]) in
        reg.captures_iter(line).take(5).map(|c| c.extract())
    {
        let confidence = confidence.parse().unwrap();
        if confidence < 30 {
            println!("skipping candidate with confidence: {confidence}");
            continue;
        }
        return Ok(OCRWord {
            confidence,
            x1: x1.parse().unwrap(),
            x2: x2.parse().unwrap(),
            y1: y1.parse().unwrap(),
            y2: y2.parse().unwrap(),
            text: text.to_string(),
        });
    }
    Err("no".into())
}
